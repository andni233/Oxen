use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::ProgressBar;
use rayon::prelude::*;
use rocksdb::{DBWithThreadMode, MultiThreaded};
use serde_json::json;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use crate::api;
use crate::config::{AuthConfig, RepoConfig};
use crate::error::OxenError;
use crate::http::response::{CommitMsgResponse, RepositoryHeadResponse, RepositoryResponse};
use crate::index::committer::HISTORY_DIR;
use crate::index::Committer;
use crate::model::{CommitHead, CommitMsg, Dataset, Repository};

use crate::util::{hasher, FileUtil};

pub const OXEN_HIDDEN_DIR: &str = ".oxen";
pub const REPO_CONFIG_FILE: &str = "config.toml";

pub struct Indexer {
    pub root_dir: PathBuf,
    pub hidden_dir: PathBuf,
    config_file: PathBuf,
    auth_config: AuthConfig,
    repo_config: Option<RepoConfig>,
}

impl Indexer {
    pub fn new(root_dir: &Path) -> Indexer {
        let hidden_dir = PathBuf::from(&root_dir).join(Path::new(OXEN_HIDDEN_DIR));
        let config_file = PathBuf::from(&hidden_dir).join(Path::new(REPO_CONFIG_FILE));
        let auth_config = AuthConfig::default().unwrap();

        // Load repo config if exists
        let repo_config: Option<RepoConfig> = match config_file.exists() {
            true => Some(RepoConfig::new(&config_file)),
            false => None,
        };
        Indexer {
            root_dir: root_dir.to_path_buf(),
            hidden_dir,
            config_file,
            auth_config,
            repo_config,
        }
    }

    pub fn repo_exists(dirname: &Path) -> bool {
        let hidden_dir = PathBuf::from(dirname).join(Path::new(OXEN_HIDDEN_DIR));
        hidden_dir.exists()
    }

    pub fn is_initialized(&self) -> bool {
        Indexer::repo_exists(&self.root_dir)
    }

    pub fn init(&self) -> Result<(), OxenError> {
        if self.is_initialized() {
            println!("Repository already exists for: {:?}", self.root_dir);
            return Ok(());
        }

        // Get name from current directory name
        if let Some(name) = self.root_dir.file_name() {
            self.init_with_name(name.to_str().unwrap())
        } else {
            let err = format!(
                "Could not find parent directories name: {:?}",
                self.root_dir
            );
            Err(OxenError::basic_str(&err))
        }
    }

    pub fn init_with_name(&self, name: &str) -> Result<(), OxenError> {
        if self.is_initialized() {
            println!("Repository already exists for: {:?}", self.root_dir);
            return Ok(());
        }

        println!("Initializing 🐂 repository with name: {}", name);

        // Make hidden .oxen dir
        std::fs::create_dir(&self.hidden_dir)?;

        let auth_cfg = AuthConfig::default()?;
        let repository = Repository {
            id: format!("{}", uuid::Uuid::new_v4()),
            name: String::from(name),
            url: String::from(""), // no remote to start
        };
        let repo_config = RepoConfig::from(&auth_cfg, &repository);
        let repo_config_file = self.hidden_dir.join(REPO_CONFIG_FILE);
        repo_config.save(&repo_config_file)?;
        println!("Repository initialized at {:?}", self.hidden_dir);
        Ok(())
    }

    pub fn set_remote(&mut self, url: &str) -> Result<(), OxenError> {
        let repository = api::repositories::get_by_url(&self.auth_config, url)?;
        self.repo_config = Some(RepoConfig::from(&self.auth_config, &repository));
        self.repo_config
            .as_ref()
            .unwrap()
            .save(Path::new(&self.config_file))?;
        println!("Remote set: {}", url);
        Ok(())
    }

    fn push_entries(
        &self,
        committer: &Arc<Committer>,
        commit: &CommitMsg,
    ) -> Result<(), OxenError> {
        let paths = committer.list_unsynced_files_for_commit(&commit.id)?;

        println!("🐂 push {} files", paths.len());

        // len is usize and progressbar requires u64, I don't think we'll overflow...
        let size: u64 = unsafe { std::mem::transmute(paths.len()) };
        let bar = ProgressBar::new(size);

        let commit_db = &committer.head_commit_db;

        // Create threadpool with N workers
        // https://docs.rs/threadpool/latest/threadpool/

        paths.par_iter().for_each(|path| {
            self.hash_and_push(committer, commit_db, path);
            bar.inc(1);
        });

        bar.finish();

        Ok(())
    }

    fn hash_and_push(
        &self,
        committer: &Arc<Committer>,
        db: &Option<DBWithThreadMode<MultiThreaded>>,
        path: &Path,
    ) {
        // hash the file
        // find the entry in the history commit db
        // compare it to the last hash
        // TODO: if it is different, upload it, and mark it as being changed?
        //       maybe on the server we make a linked list of the changes with the commit id?
        // if it is the same, don't re-upload
        // Update the hash for this specific commit for this path
        if let Ok(hash) = hasher::hash_file_contents(path) {
            match FileUtil::path_relative_to_dir(path, &self.root_dir) {
                Ok(path) => {
                    // Compare last hash to new one
                    let old_hash = committer.get_path_hash(db, &path).unwrap();
                    if old_hash == hash {
                        // we don't need to upload if hash is the same
                        // println!("Hash is the same! don't upload again {:?}", path);
                        return;
                    }

                    // Upload entry to server
                    match api::entries::create(self.repo_config.as_ref().unwrap(), &path, &hash) {
                        Ok(_entry) => {
                            // The last thing we do is update the hash in the local db
                            // after it has been posted to the server, so that even if the process
                            // is killed, and we don't get here, the worst thing that can happen
                            // is we re-upload it.
                            match committer.update_path_hash(db, &path, &hash) {
                                Ok(_) => {
                                    // println!("Updated hash! {:?} => {}", path, hash);
                                }
                                Err(err) => {
                                    eprintln!("Error updating hash {:?} {}", path, err)
                                }
                            }
                        }
                        Err(err) => {
                            eprintln!("Error uploading {:?} {}", path, err)
                        }
                    }
                }
                Err(_) => {
                    eprintln!("Could not get relative path...");
                }
            }
        }
    }

    pub fn push(&self, committer: &Arc<Committer>) -> Result<(), OxenError> {
        self.create_or_get_repo()?;
        match committer.get_head_commit() {
            Ok(Some(commit)) => {
                // maybe_push() will recursively check commits head against remote head
                // and sync ones that have not been synced
                let remote_head = self.get_remote_head()?;
                self.maybe_push(committer, &remote_head, &commit.id, 0)?;
                Ok(())
            }
            Ok(None) => Err(OxenError::basic_str("No commits to push.")),
            Err(err) => {
                let msg = format!("Err: {}", err);
                Err(OxenError::basic_str(&msg))
            }
        }
    }

    pub fn create_or_get_repo(&self) -> Result<(), OxenError> {
        // TODO move into another api class, and better error handling...just cranking this out
        let name = &self.repo_config.as_ref().unwrap().repository.name;
        let url = "http://0.0.0.0:3000/repositories".to_string();
        let params = json!({ "name": name });

        let client = reqwest::blocking::Client::new();
        if let Ok(res) = client.post(url).json(&params).send() {
            let body = res.text()?;
            let response: Result<RepositoryResponse, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(_) => Ok(()),
                Err(_) => Ok(()), // we are just assuming this error is already exists for now
            }
        } else {
            Err(OxenError::basic_str(
                "create_or_get_repo() Could not create repo",
            ))
        }
    }

    fn maybe_push(
        &self,
        committer: &Arc<Committer>,
        remote_head: &Option<CommitHead>,
        commit_id: &str,
        depth: usize,
    ) -> Result<(), OxenError> {
        if let Some(head) = remote_head {
            if commit_id == head.commit_id {
                if depth == 0 && head.is_synced() {
                    println!("No commits to push, remote is synced.");
                    return Ok(());
                } else if head.is_synced() {
                    return Ok(());
                }
            }
        }

        if let Some(commit) = committer.get_commit_by_id(commit_id)? {
            if let Some(parent_id) = &commit.parent_id {
                self.maybe_push(committer, remote_head, parent_id, depth + 1)?;
            }
            // Unroll stack to post in reverse order

            // TODO: enable pushing of entries first, then final step push the commit..?
            // or somehow be able to resume pushing the commit? Like check # of synced files on the server
            // and compare
            self.post_commit_to_server(&commit)?;
            self.push_entries(committer, &commit)?;
        } else {
            eprintln!("Err: could not find commit: {}", commit_id);
        }

        Ok(())
    }

    pub fn get_remote_head(&self) -> Result<Option<CommitHead>, OxenError> {
        // TODO move into another api class, need to better delineate what we call these
        // also is this remote the one in the config? I think so, need to draw out a diagram
        let name = &self.repo_config.as_ref().unwrap().repository.name;
        let url = format!("http://0.0.0.0:3000/repositories/{}", name);
        let client = reqwest::blocking::Client::new();
        if let Ok(res) = client.get(url).send() {
            // TODO: handle if remote repo does not exist...
            // Do we create it then push for now? Or add separate command to create?
            // I think we create and push, and worry about authorized keys etc later
            let body = res.text()?;
            let response: Result<RepositoryHeadResponse, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(j_res) => Ok(j_res.head),
                Err(err) => Err(OxenError::basic_str(&format!(
                    "get_remote_head() Could not serialize response [{}]\n{}",
                    err, body
                ))),
            }
        } else {
            Err(OxenError::basic_str("get_remote_head() Request failed"))
        }
    }

    pub fn post_commit_to_server(&self, commit: &CommitMsg) -> Result<(), OxenError> {
        // zip up the rocksdb in history dir, and post to server
        let commit_dir = self.hidden_dir.join(HISTORY_DIR).join(&commit.id);
        let path_to_compress = format!("history/{}", commit.id);

        println!("Compressing commit {}...", commit.id);
        let enc = GzEncoder::new(Vec::new(), Compression::default());
        let mut tar = tar::Builder::new(enc);

        tar.append_dir_all(path_to_compress, commit_dir)?;
        tar.finish()?;
        let buffer: Vec<u8> = tar.into_inner()?.finish()?;
        self.post_tarball_to_server(&buffer, commit)?;

        Ok(())
    }

    fn post_tarball_to_server(&self, buffer: &[u8], commit: &CommitMsg) -> Result<(), OxenError> {
        println!("Syncing commit {}...", commit.id);

        let name = &self.repo_config.as_ref().unwrap().repository.name;
        let client = reqwest::blocking::Client::new();
        let url = format!(
            "http://0.0.0.0:3000/repositories/{}/commits?{}",
            name,
            commit.to_uri_encoded()
        );
        if let Ok(res) = client
            .post(url)
            .body(reqwest::blocking::Body::from(buffer.to_owned()))
            .send()
        {
            let status = res.status();
            let body = res.text()?;
            let response: Result<CommitMsgResponse, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(_) => Ok(()),
                Err(_) => Err(OxenError::basic_str(&format!(
                    "Error serializing CommitMsgResponse: status_code[{}] \n\n{}",
                    status, body
                ))),
            }
        } else {
            Err(OxenError::basic_str(
                "post_tarball_to_server error sending data from file",
            ))
        }
    }

    pub fn list_datasets(&self) -> Result<Vec<Dataset>, OxenError> {
        api::datasets::list(self.repo_config.as_ref().unwrap())
    }

    pub fn pull(&self) -> Result<(), OxenError> {
        // Get list of commits we have to pull

        // For each commit
        // - pull dbs
        // - pull entries given the db

        let total: usize = 0;
        println!("🐂 pulling {} entries", total);
        let size: u64 = unsafe { std::mem::transmute(total) };
        let bar = ProgressBar::new(size);
        // dataset_pages.par_iter().for_each(|dataset_pages| {
        //     let (dataset, num_pages) = dataset_pages;
        //     match self.pull_dataset(dataset, num_pages, &bar) {
        //         Ok(_) => {}
        //         Err(err) => {
        //             eprintln!("Error pulling dataset: {}", err)
        //         }
        //     }
        // });
        bar.finish();
        Ok(())
    }

    /*
    fn download_url(
        &self,
        dataset: &Dataset,
        entry: &crate::model::Entry,
    ) -> Result<(), OxenError> {
        let path = Path::new(&dataset.name);
        let fname = path.join(&entry.filename);
        // println!("Downloading file {:?}", &fname);
        if !fname.exists() {
            let mut response = reqwest::blocking::get(&entry.url)?;
            let mut dest = { File::create(fname)? };
            response.copy_to(&mut dest)?;
        }
        Ok(())
    }
    */
}

#[cfg(test)]
mod tests {
    use crate::error::OxenError;
    use crate::index::indexer::OXEN_HIDDEN_DIR;
    use crate::index::Indexer;
    use crate::model::Repository;
    use crate::test;

    const BASE_DIR: &str = "data/test/runs";

    #[test]
    fn test_1_indexer_init() -> Result<(), OxenError> {
        test::setup_env();

        let repo_dir = test::create_repo_dir(BASE_DIR)?;
        let indexer = Indexer::new(&repo_dir);
        indexer.init()?;

        let repository = Repository::from(&repo_dir);
        let hidden_dir = repo_dir.join(OXEN_HIDDEN_DIR);
        assert!(hidden_dir.exists());
        assert!(!repository.id.is_empty());
        let name = repo_dir.file_name().unwrap().to_str().unwrap();
        assert_eq!(repository.name, name);

        // cleanup
        std::fs::remove_dir_all(repo_dir)?;

        Ok(())
    }

    #[test]
    fn test_1_indexer_init_with_name() -> Result<(), OxenError> {
        test::setup_env();

        let repo_dir = test::create_repo_dir(BASE_DIR)?;
        let indexer = Indexer::new(&repo_dir);

        let name = "gschoeni/Repo-Name";
        indexer.init_with_name(name)?;

        let repository = Repository::from(&repo_dir);
        let hidden_dir = repo_dir.join(OXEN_HIDDEN_DIR);
        assert!(hidden_dir.exists());
        assert!(!repository.id.is_empty());
        assert_eq!(repository.name, name);

        // cleanup
        std::fs::remove_dir_all(repo_dir)?;

        Ok(())
    }
}