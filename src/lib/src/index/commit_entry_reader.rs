
use crate::error::OxenError;
use crate::index::Committer;
use crate::model::{Commit, CommitEntry};

use rocksdb::{DBWithThreadMode, IteratorMode, LogLevel, MultiThreaded, Options};
use std::path::{Path, PathBuf};
use std::str;

use crate::model::LocalRepository;


pub struct CommitEntryReader {
    db: DBWithThreadMode<MultiThreaded>,
}

impl CommitEntryReader {
    pub fn db_opts() -> Options {
        let mut opts = Options::default();
        opts.set_log_level(LogLevel::Fatal);
        opts.create_if_missing(true);
        opts
    }

    pub fn new(repository: &LocalRepository, commit: &Commit) -> Result<CommitEntryReader, OxenError> {
        let db_path = Committer::history_dir(&repository.path).join(commit.id.to_owned());
        let opts = CommitEntryReader::db_opts();
        Ok(CommitEntryReader {
            db: DBWithThreadMode::open_for_read_only(&opts, &db_path, false)?,
        })
    }

    pub fn num_entries(&self) -> Result<usize, OxenError> {
        log::debug!("num_entries_in_head reading from db: {:?}", self.db.path());
        Ok(self.db.iterator(IteratorMode::Start).count())
    }

    pub fn get_path_hash(
        &self,
        path: &Path,
    ) -> Result<String, OxenError> {
        let key = path.to_str().unwrap();
        let bytes = key.as_bytes();
        match self.db.get(bytes) {
            Ok(Some(value)) => {
                let value = str::from_utf8(&*value)?;
                let entry: CommitEntry = serde_json::from_str(value)?;
                Ok(entry.hash)
            }
            Ok(None) => Ok(String::from("")), // no hash, empty string
            Err(err) => {
                let err = format!("get_path_hash() Err: {}", err);
                Err(OxenError::basic_str(&err))
            }
        }
    }

    pub fn list_files(&self) -> Result<Vec<PathBuf>, OxenError> {
        let mut paths: Vec<PathBuf> = vec![];
        let iter = self.db.iterator(IteratorMode::Start);
        for (key, _value) in iter {
            paths.push(PathBuf::from(str::from_utf8(&*key)?));
        }
        Ok(paths)
    }

    pub fn list_entries(&self) -> Result<Vec<CommitEntry>, OxenError> {
        let mut paths: Vec<CommitEntry> = vec![];
        let iter = self.db.iterator(IteratorMode::Start);
        for (_key, value) in iter {
            let entry: CommitEntry = serde_json::from_str(str::from_utf8(&*value)?)?;
            paths.push(entry);
        }
        Ok(paths)
    }

    pub fn list_entry_page(
        &self,
        page_num: usize,
        page_size: usize,
    ) -> Result<Vec<CommitEntry>, OxenError> {
        // The iterator doesn't technically have a skip method as far as I can tell
        // so we are just going to manually do it
        let mut paths: Vec<CommitEntry> = vec![];
        let iter = self.db.iterator(IteratorMode::Start);
        // Do not go negative, and start from 0
        let start_page = if page_num == 0 { 0 } else { page_num - 1 };
        let start_idx = start_page * page_size;
        for (entry_i, (_key, value)) in iter.enumerate() {
            // limit to page_size
            if paths.len() >= page_size {
                break;
            }

            // only grab values after start_idx based on page_num and page_size
            if entry_i >= start_idx {
                let entry: CommitEntry = serde_json::from_str(str::from_utf8(&*value)?)?;
                paths.push(entry);
            }
        }
        Ok(paths)
    }

    pub fn has_prefix_in_dir(&self, prefix: &Path) -> bool {
        match self.list_entries() {
            Ok(entries) => entries.into_iter().any(|entry| entry.path.starts_with(prefix)),
            _ => false,
        }
    }

    pub fn list_files_from_dir(&self, dir: &Path) -> Vec<CommitEntry> {
        match self.list_entries() {
            Ok(entries) => entries
                .into_iter()
                .filter(|entry| entry.path.starts_with(dir))
                .collect(),
            _ => {
                vec![]
            }
        }
    }

    pub fn get_entry(&self, path: &Path) -> Result<Option<CommitEntry>, OxenError> {

        let key = path.to_str().unwrap();
        let bytes = key.as_bytes();
        match self.db.get(bytes) {
            Ok(Some(value)) => match str::from_utf8(&*value) {
                Ok(value) => {
                    let entry: CommitEntry = serde_json::from_str(value)?;
                    Ok(Some(entry))
                }
                Err(_) => Err(OxenError::basic_str(
                    "get_local_entry_from_commit invalid entry",
                )),
            },
            Ok(None) => Ok(None),
            Err(err) => {
                let err = format!("get_local_entry_from_commit Error reading db\nErr: {}", err);
                Err(OxenError::basic_str(&err))
            }
        }
    }

    pub fn contains_path(&self, path: &Path) -> Result<bool, OxenError> {
        // Check if path is in this commit
        let key = path.to_str().unwrap();
        let bytes = key.as_bytes();
        match self.db.get(bytes) {
            Ok(Some(_value)) => Ok(true),
            Ok(None) => Ok(false),
            Err(err) => {
                let err = format!("head_contains_file Error reading db\nErr: {}", err);
                Err(OxenError::basic_str(&err))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::command;
    use crate::error::OxenError;
    use crate::index::CommitEntryReader;
    use crate::test;

    use std::path::Path;

    #[test]
    fn test_check_if_file_exists() -> Result<(), OxenError> {
        test::run_training_data_repo_test_no_commits(|repo| {
            let filename = "labels.txt";
            let filepath = repo.path.join(filename);
            command::add(&repo, &filepath)?;
            let commit = command::commit(&repo, "Adding labels file")?.unwrap();

            let reader = CommitEntryReader::new(&repo, &commit)?;
            let path = Path::new(filename);
            assert!(reader.contains_path(&path)?);

            Ok(())
        })
    }
}
