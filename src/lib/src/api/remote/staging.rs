use crate::api;
use crate::api::remote::client;
use crate::error::OxenError;
use crate::model::entry::mod_entry::ModType;
use crate::model::{Commit, CommitBody, DataFrameDiff, ModEntry, ObjectID, RemoteRepository};
use crate::model::{ContentType, Schema};
use crate::view::{
    CommitResponse, FilePathsResponse, ListStagedFileModResponseDF, RemoteStagedStatus,
    RemoteStagedStatusResponse, StagedFileModResponse,
};

use std::path::{Path, PathBuf};

pub async fn status(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    path: &Path,
    page: usize,
    page_size: usize,
) -> Result<RemoteStagedStatus, OxenError> {
    let path_str = path.to_str().unwrap();
    let uri = format!(
        "/staging/{identifier}/status/{branch_name}/{path_str}?page={page}&page_size={page_size}"
    );
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;
    log::debug!("status url: {url}");

    let client = client::new_for_url(&url)?;
    match client.get(&url).send().await {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            log::debug!("status got body: {}", body);
            let response: Result<RemoteStagedStatusResponse, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(val) => Ok(val.staged),
                Err(err) => Err(OxenError::basic_str(format!(
                    "api::staging::status error parsing response from {url}\n\nErr {err:?} \n\n{body}"
                ))),
            }
        }
        Err(err) => {
            let err = format!("api::staging::status Request failed: {url}\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

pub async fn add_file(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    directory_name: &str,
    path: PathBuf,
) -> Result<PathBuf, OxenError> {
    let uri = format!("/staging/{identifier}/file/{branch_name}/{directory_name}");
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;

    let file_name = path
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .ok()
        .unwrap();
    let file = std::fs::read(&path).unwrap();
    let file_part = reqwest::multipart::Part::bytes(file).file_name(file_name);
    let form = reqwest::multipart::Form::new().part("file", file_part);
    let client = client::new_for_url(&url)?;
    match client.post(&url).multipart(form).send().await {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            let response: Result<FilePathsResponse, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(val) => {
                    let path = val.paths[0].clone();
                    Ok(path)
                }
                Err(err) => {
                    let err = format!("api::staging::add_file error parsing response from {url}\n\nErr {err:?} \n\n{body}");
                    Err(OxenError::basic_str(err))
                }
            }
        }
        Err(err) => {
            let err = format!("api::staging::add_file Request failed: {url}\n\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

pub async fn add_files(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    directory_name: &str,
    paths: Vec<PathBuf>,
) -> Result<Vec<PathBuf>, OxenError> {
    let uri = format!("/staging/{identifier}/file/{branch_name}/{directory_name}");
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;

    let mut form = reqwest::multipart::Form::new();
    for path in paths {
        let file_name = path
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .ok()
            .unwrap();
        let file = std::fs::read(&path).unwrap();
        let file_part = reqwest::multipart::Part::bytes(file).file_name(file_name);
        form = form.part("file", file_part);
    }

    let client = client::new_for_url(&url)?;
    match client.post(&url).multipart(form).send().await {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            let response: Result<FilePathsResponse, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(val) => Ok(val.paths),
                Err(err) => {
                    let err = format!("api::staging::add_files error parsing response from {url}\n\nErr {err:?} \n\n{body}");
                    Err(OxenError::basic_str(err))
                }
            }
        }
        Err(err) => {
            let err = format!("api::staging::add_files Request failed: {url}\n\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

pub async fn stage_modification(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    path: &Path,
    data: String,
    content_type: ContentType,
    mod_type: ModType,
) -> Result<ModEntry, OxenError> {
    if mod_type != ModType::Append {
        return Err(OxenError::basic_str(
            "api::staging::stage_modification only supports ModType::Append",
        ));
    }

    let file_path_str = path.to_str().unwrap();
    let uri = format!("/staging/{identifier}/df/rows/{branch_name}/{file_path_str}");
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;
    log::debug!("stage_modification {url}\n{data}");

    let client = client::new_for_url(&url)?;
    match client
        .post(&url)
        .header("Content-Type", content_type.to_http_content_type())
        .body(data)
        .send()
        .await
    {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            let response: Result<StagedFileModResponse, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(val) => Ok(val.modification),
                Err(err) => {
                    let err = format!("api::staging::stage_modification error parsing response from {url}\n\nErr {err:?} \n\n{body}");
                    Err(OxenError::basic_str(err))
                }
            }
        }
        Err(err) => {
            let err =
                format!("api::staging::stage_modification Request failed: {url}\n\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

pub async fn commit_staged(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    commit: &CommitBody,
) -> Result<Commit, OxenError> {
    let uri = format!("/staging/{identifier}/commit/{branch_name}");
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;
    let body = serde_json::to_string(&commit).unwrap();
    log::debug!("commit_staged {}\n{}", url, body);

    let client = client::new_for_url(&url)?;
    match client
        .post(&url)
        .body(reqwest::Body::from(body))
        .send()
        .await
    {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            log::debug!("commit_staged got body: {}", body);
            let response: Result<CommitResponse, serde_json::Error> = serde_json::from_str(&body);
            match response {
                Ok(val) => Ok(val.commit),
                Err(err) => Err(OxenError::basic_str(format!(
                    "api::staging::commit_staged error parsing response from {url}\n\nErr {err:?} \n\n{body}"
                ))),
            }
        }
        Err(err) => {
            let err = format!("api::staging::commit_staged Request failed: {url}\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

pub async fn rm_staged_file(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    path: impl AsRef<Path>,
) -> Result<(), OxenError> {
    let file_name = path.as_ref().to_string_lossy();
    let uri = format!("/staging/{identifier}/file/{branch_name}/{file_name}");
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;
    log::debug!("rm_staged_file {}", url);
    let client = client::new_for_url(&url)?;
    match client.delete(&url).send().await {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            log::debug!("rm_staged_file got body: {}", body);
            Ok(())
        }
        Err(err) => {
            let err = format!("rm_staged_file Request failed: {url}\n\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

pub async fn restore_df(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    path: impl AsRef<Path>,
) -> Result<(), OxenError> {
    let file_name = path.as_ref().to_string_lossy();
    let uri = format!("/staging/{identifier}/modifications/{branch_name}/{file_name}");
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;
    log::debug!("restore_df {}", url);
    let client = client::new_for_url(&url)?;
    match client.delete(&url).send().await {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            log::debug!("restore_df got body: {}", body);
            Ok(())
        }
        Err(err) => {
            let err = format!("restore_df Request failed: {url}\n\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

pub async fn delete_staged_modification(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    path: impl AsRef<Path>,
    uuid: &str,
) -> Result<ModEntry, OxenError> {
    let file_name = path.as_ref().to_string_lossy();
    let uri = format!("/staging/{identifier}/df/rows/{branch_name}/{file_name}");
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;
    log::debug!("delete_staged_modification [{}] {}", uuid, url);
    let client = client::new_for_url(&url)?;
    let id = ObjectID {
        id: uuid.to_string(),
    };
    let json_id = serde_json::to_string(&id).unwrap();
    match client.delete(&url).body(json_id).send().await {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            log::debug!("delete_staged_modification got body: {}", body);
            let response: Result<StagedFileModResponse, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(val) => Ok(val.modification),
                Err(err) => {
                    let err = format!("api::staging::delete_staged_modification error parsing response from {url}\n\nErr {err:?} \n\n{body}");
                    Err(OxenError::basic_str(err))
                }
            }
        }
        Err(err) => {
            let err = format!("delete_staged_modification Request failed: {url}\n\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

pub async fn diff_staged_file(
    remote_repo: &RemoteRepository,
    branch_name: &str,
    identifier: &str,
    path: impl AsRef<Path>,
    page: usize,
    page_size: usize,
) -> Result<DataFrameDiff, OxenError> {
    let path_str = path.as_ref().to_str().unwrap();
    let uri = format!(
        "/staging/{identifier}/diff/{branch_name}/{path_str}?page={page}&page_size={page_size}"
    );
    let url = api::endpoint::url_from_repo(remote_repo, &uri)?;

    let client = client::new_for_url(&url)?;
    match client.get(&url).send().await {
        Ok(res) => {
            let body = client::parse_json_body(&url, res).await?;
            log::debug!("diff_staged_file got body: {}", body);
            let response: Result<ListStagedFileModResponseDF, serde_json::Error> =
                serde_json::from_str(&body);
            match response {
                Ok(val) => {
                    let mods = val.modifications;
                    let added_rows = mods.added_rows.map(|added| added.to_df());
                    let schema = Schema::from_polars(&added_rows.as_ref().unwrap().schema());

                    Ok(DataFrameDiff {
                        base_schema: schema,
                        added_rows,
                        removed_rows: None,
                        added_cols: None,
                        removed_cols: None,
                    })
                },
                Err(err) => Err(OxenError::basic_str(format!(
                    "api::staging::diff_staged_file error parsing response from {url}\n\nErr {err:?} \n\n{body}"
                ))),
            }
        }
        Err(err) => {
            let err = format!("api::staging::diff_staged_file Request failed: {url}\nErr {err:?}");
            Err(OxenError::basic_str(err))
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::config::UserConfig;
    use crate::constants::{
        DEFAULT_BRANCH_NAME, DEFAULT_PAGE_NUM, DEFAULT_PAGE_SIZE, DEFAULT_REMOTE_NAME,
    };
    use crate::error::OxenError;
    use crate::model::entry::mod_entry::ModType;
    use crate::model::ContentType;
    use crate::model::{CommitBody, User};
    use crate::opts::CloneOpts;
    use crate::test;
    use crate::{api, command, constants};

    use std::path::Path;

    #[tokio::test]
    async fn test_list_empty_staging_dir_empty_remote() -> Result<(), OxenError> {
        test::run_empty_remote_repo_test(|mut local_repo, remote_repo| async move {
            let branch_name = "add-images";
            api::local::branches::create_checkout(&local_repo, branch_name)?;
            let remote = test::repo_remote_url_from(&local_repo.dirname());
            command::config::set_remote(&mut local_repo, constants::DEFAULT_REMOTE_NAME, &remote)?;
            command::push(&local_repo).await?;

            // client can decide what to use for id
            let identifier = UserConfig::identifier()?;
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);

            let page_num = constants::DEFAULT_PAGE_NUM;
            let page_size = constants::DEFAULT_PAGE_SIZE;
            let path = Path::new("images");
            let entries = api::remote::staging::status(
                &remote_repo,
                branch_name,
                &identifier,
                path,
                page_num,
                page_size,
            )
            .await?;
            assert_eq!(entries.added_files.entries.len(), 0);
            assert_eq!(entries.added_files.total_entries, 0);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_list_empty_staging_dir_all_data_pushed() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);

            let identifier = UserConfig::identifier()?;
            let page_num = constants::DEFAULT_PAGE_NUM;
            let page_size = constants::DEFAULT_PAGE_SIZE;
            let path = Path::new("images");
            let entries = api::remote::staging::status(
                &remote_repo,
                branch_name,
                &identifier,
                path,
                page_num,
                page_size,
            )
            .await?;
            assert_eq!(entries.added_files.entries.len(), 0);
            assert_eq!(entries.added_files.total_entries, 0);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_stage_single_file() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);

            let directory_name = "images";
            let identifier = UserConfig::identifier()?;
            let path = test::test_jpeg_file().to_path_buf();
            let result = api::remote::staging::add_file(
                &remote_repo,
                branch_name,
                &identifier,
                directory_name,
                path,
            )
            .await;
            assert!(result.is_ok());

            let page_num = constants::DEFAULT_PAGE_NUM;
            let page_size = constants::DEFAULT_PAGE_SIZE;
            let path = Path::new(directory_name);
            let entries = api::remote::staging::status(
                &remote_repo,
                branch_name,
                &identifier,
                path,
                page_num,
                page_size,
            )
            .await?;
            assert_eq!(entries.added_files.entries.len(), 1);
            assert_eq!(entries.added_files.total_entries, 1);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_rm_staged_file() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);

            let identifier = UserConfig::identifier()?;
            let directory_name = "images";
            let path = test::test_jpeg_file().to_path_buf();
            let result = api::remote::staging::add_file(
                &remote_repo,
                branch_name,
                &identifier,
                directory_name,
                path,
            )
            .await;
            assert!(result.is_ok());

            // Remove the file
            let result = api::remote::staging::rm_staged_file(
                &remote_repo,
                branch_name,
                &identifier,
                result.unwrap(),
            )
            .await;
            assert!(result.is_ok());

            // Make sure we have 0 files staged
            let page_num = constants::DEFAULT_PAGE_NUM;
            let page_size = constants::DEFAULT_PAGE_SIZE;
            let path = Path::new(directory_name);
            let entries = api::remote::staging::status(
                &remote_repo,
                branch_name,
                &identifier,
                path,
                page_num,
                page_size,
            )
            .await?;
            assert_eq!(entries.added_files.entries.len(), 0);
            assert_eq!(entries.added_files.total_entries, 0);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_stage_multiple_files() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-data";
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);

            let identifier = UserConfig::identifier()?;
            let directory_name = "data";
            let paths = vec![
                test::test_jpeg_file().to_path_buf(),
                test::test_jpeg_file_with_name("cole_anthony.jpeg"),
            ];
            let result = api::remote::staging::add_files(
                &remote_repo,
                branch_name,
                &identifier,
                directory_name,
                paths,
            )
            .await;
            assert!(result.is_ok());

            let page_num = constants::DEFAULT_PAGE_NUM;
            let page_size = constants::DEFAULT_PAGE_SIZE;
            let path = Path::new(directory_name);
            let entries = api::remote::staging::status(
                &remote_repo,
                branch_name,
                &identifier,
                path,
                page_num,
                page_size,
            )
            .await?;
            assert_eq!(entries.added_files.entries.len(), 2);
            assert_eq!(entries.added_files.total_entries, 2);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_commit_staged_single_file_and_pull() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-data";
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);

            let identifier = UserConfig::identifier()?;
            let file_to_post = test::test_jpeg_file().to_path_buf();
            let directory_name = "data";
            let result = api::remote::staging::add_file(
                &remote_repo,
                branch_name,
                &identifier,
                directory_name,
                file_to_post,
            )
            .await;
            assert!(result.is_ok());

            let body = CommitBody {
                message: "Add one image".to_string(),
                user: User {
                    name: "Test User".to_string(),
                    email: "test@oxen.ai".to_string(),
                },
            };
            let commit =
                api::remote::staging::commit_staged(&remote_repo, branch_name, &identifier, &body)
                    .await?;

            let remote_commit = api::remote::commits::get_by_id(&remote_repo, &commit.id).await?;
            assert!(remote_commit.is_some());
            assert_eq!(commit.id, remote_commit.unwrap().id);

            let remote_repo_cloned = remote_repo.clone();
            test::run_empty_dir_test_async(|cloned_repo_dir| async move {
                // Clone repo
                let opts = CloneOpts::new(remote_repo.remote.url, &cloned_repo_dir);
                let cloned_repo = command::clone(&opts).await?;

                // Make sure that image is not on main branch
                let path = cloned_repo
                    .path
                    .join(directory_name)
                    .join(test::test_jpeg_file().file_name().unwrap());
                assert!(!path.exists());

                // Pull the branch with new data
                command::pull_remote_branch(&cloned_repo, DEFAULT_REMOTE_NAME, "add-data").await?;

                // We should have the commit locally
                let log = api::local::commits::list(&cloned_repo)?;
                assert_eq!(log.first().unwrap().id, commit.id);

                // The file should exist locally
                println!("Looking for file at path: {:?}", path);
                assert!(path.exists());

                Ok(cloned_repo_dir)
            })
            .await?;

            Ok(remote_repo_cloned)
        })
        .await
    }

    #[tokio::test]
    async fn test_commit_staged_multiple_files() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-data";
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);

            let identifier = UserConfig::identifier()?;
            let directory_name = "data";
            let paths = vec![
                test::test_jpeg_file().to_path_buf(),
                test::test_jpeg_file_with_name("cole_anthony.jpeg"),
            ];
            let result = api::remote::staging::add_files(
                &remote_repo,
                branch_name,
                &identifier,
                directory_name,
                paths,
            )
            .await;
            assert!(result.is_ok());

            let body = CommitBody {
                message: "Add staged data".to_string(),
                user: User {
                    name: "Test User".to_string(),
                    email: "test@oxen.ai".to_string(),
                },
            };
            let commit =
                api::remote::staging::commit_staged(&remote_repo, branch_name, &identifier, &body)
                    .await?;

            let remote_commit = api::remote::commits::get_by_id(&remote_repo, &commit.id).await?;
            assert!(remote_commit.is_some());
            assert_eq!(commit.id, remote_commit.unwrap().id);

            println!("DONE!!");

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_should_not_stage_invalid_schema_for_dataframe() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);
            let identifier = UserConfig::identifier()?;

            // train/dog_1.jpg,dog,101.5,32.0,385,330
            let path = Path::new("annotations")
                .join("train")
                .join("bounding_box.csv");
            let data = "{\"id\": 1, \"name\": \"greg\"}";
            let result = api::remote::staging::stage_modification(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                data.to_string(),
                ContentType::Json,
                ModType::Append,
            )
            .await;

            assert!(result.is_err());

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_stage_row_on_dataframe_json() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(&remote_repo, branch_name, DEFAULT_BRANCH_NAME).await?;
            assert_eq!(branch.name, branch_name);
            let identifier = UserConfig::identifier()?;

            // train/dog_1.jpg,dog,101.5,32.0,385,330
            let path = Path::new("annotations").join("train").join("bounding_box.csv");
            let data = "{\"file\":\"image1.jpg\", \"label\": \"dog\", \"min_x\":13, \"min_y\":14, \"width\": 100, \"height\": 100}";
            let result =
                api::remote::staging::stage_modification(
                    &remote_repo,
                    branch_name,
                    &identifier,
                    &path,
                    data.to_string(),
                    ContentType::Json,
                    ModType::Append
                ).await;

            assert!(result.is_ok());
            println!("{:?}", result.unwrap());

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_stage_row_on_dataframe_csv() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            assert_eq!(branch.name, branch_name);
            let identifier = UserConfig::identifier()?;

            // train/dog_1.jpg,dog,101.5,32.0,385,330
            let path = Path::new("annotations")
                .join("train")
                .join("bounding_box.csv");
            let data = "image1.jpg, dog, 13, 14, 100, 100";
            let result = api::remote::staging::stage_modification(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                data.to_string(),
                ContentType::Csv,
                ModType::Append,
            )
            .await;

            assert!(result.is_ok());
            println!("{:?}", result.unwrap());

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_list_status_modified_dataframe() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(&remote_repo, branch_name, DEFAULT_BRANCH_NAME).await?;
            assert_eq!(branch.name, branch_name);
            let identifier = UserConfig::identifier()?;

            // train/dog_1.jpg,dog,101.5,32.0,385,330
            let directory = Path::new("annotations").join("train");
            let path = directory.join("bounding_box.csv");
            let data = "{\"file\":\"image1.jpg\", \"label\": \"dog\", \"min_x\":13, \"min_y\":14, \"width\": 100, \"height\": 100}";
            api::remote::staging::stage_modification(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                data.to_string(),
                ContentType::Json,
                ModType::Append
            ).await?;

            let page_num = constants::DEFAULT_PAGE_NUM;
            let page_size = constants::DEFAULT_PAGE_SIZE;
            let entries = api::remote::staging::status(
                &remote_repo,
                branch_name,
                &identifier,
                &directory,
                page_num,
                page_size,
            )
            .await?;
            assert_eq!(entries.modified_files.entries.len(), 1);
            assert_eq!(entries.modified_files.total_entries, 1);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_list_delete_row_from_modified_dataframe() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(&remote_repo, branch_name, DEFAULT_BRANCH_NAME).await?;
            assert_eq!(branch.name, branch_name);
            let identifier = UserConfig::identifier()?;

            // train/dog_1.jpg,dog,101.5,32.0,385,330
            let directory = Path::new("annotations").join("train");
            let path = directory.join("bounding_box.csv");
            let data = "{\"file\":\"image1.jpg\", \"label\": \"dog\", \"min_x\":13, \"min_y\":14, \"width\": 100, \"height\": 100}";
            let result_1 = api::remote::staging::stage_modification(
                    &remote_repo,
                    branch_name,
                    &identifier,
                    &path,
                    data.to_string(),
                    ContentType::Json,
                    ModType::Append
                ).await;
            assert!(result_1.is_ok());

            let data = "{\"file\":\"image2.jpg\", \"label\": \"cat\", \"min_x\":13, \"min_y\":14, \"width\": 100, \"height\": 100}";
            let result_2 = api::remote::staging::stage_modification(
                    &remote_repo,
                    branch_name,
                    &identifier,
                    &path,
                    data.to_string(),
                    ContentType::Json,
                    ModType::Append
                ).await?;

            // Make sure both got staged
            let diff = api::remote::staging::diff_staged_file(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                DEFAULT_PAGE_NUM,
                DEFAULT_PAGE_SIZE
            ).await?;
            let added_rows = diff.added_rows.unwrap();
            assert_eq!(added_rows.height(), 2);

            // Delete result_2
            let result_delete = api::remote::staging::delete_staged_modification(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                &result_2.uuid
            ).await;
            assert!(result_delete.is_ok());

            // Make there is only one left
            let diff = api::remote::staging::diff_staged_file(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                DEFAULT_PAGE_NUM,
                DEFAULT_PAGE_SIZE
            ).await?;
            let added_rows = diff.added_rows.unwrap();
            assert_eq!(added_rows.height(), 1);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_restore_modified_dataframe() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(&remote_repo, branch_name, DEFAULT_BRANCH_NAME).await?;
            assert_eq!(branch.name, branch_name);
            let identifier = UserConfig::identifier()?;

            // train/dog_1.jpg,dog,101.5,32.0,385,330
            let directory = Path::new("annotations").join("train");
            let path = directory.join("bounding_box.csv");
            let data = "{\"file\":\"image1.jpg\", \"label\": \"dog\", \"min_x\":13, \"min_y\":14, \"width\": 100, \"height\": 100}";
            let result_1 = api::remote::staging::stage_modification(
                    &remote_repo,
                    branch_name,
                    &identifier,
                    &path,
                    data.to_string(),
                    ContentType::Json,
                    ModType::Append
                ).await;
            assert!(result_1.is_ok());

            let data = "{\"file\":\"image2.jpg\", \"label\": \"cat\", \"min_x\":13, \"min_y\":14, \"width\": 100, \"height\": 100}";
            let result_2 = api::remote::staging::stage_modification(
                    &remote_repo,
                    branch_name,
                    &identifier,
                    &path,
                    data.to_string(),
                    ContentType::Json,
                    ModType::Append
                ).await;
            assert!(result_2.is_ok());

            // Make sure both got staged
            let diff = api::remote::staging::diff_staged_file(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                DEFAULT_PAGE_NUM,
                DEFAULT_PAGE_SIZE
            ).await?;
            let added_rows = diff.added_rows.unwrap();
            assert_eq!(added_rows.height(), 2);

            // Delete result_2
            let result_delete = api::remote::staging::restore_df(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
            ).await;
            assert!(result_delete.is_ok());

            // Should be cleared
            let diff = api::remote::staging::diff_staged_file(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                DEFAULT_PAGE_NUM,
                DEFAULT_PAGE_SIZE
            ).await?;
            let added_rows = diff.added_rows.unwrap();
            assert_eq!(added_rows.height(), 0);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_diff_modified_dataframe() -> Result<(), OxenError> {
        test::run_remote_repo_test_all_data_pushed(|remote_repo| async move {
            let branch_name = "add-images";
            let branch = api::remote::branches::create_from_or_get(&remote_repo, branch_name, DEFAULT_BRANCH_NAME).await?;
            assert_eq!(branch.name, branch_name);
            let identifier = UserConfig::identifier()?;

            // train/dog_1.jpg,dog,101.5,32.0,385,330
            let directory = Path::new("annotations").join("train");
            let path = directory.join("bounding_box.csv");
            let data = "{\"file\":\"image1.jpg\", \"label\": \"dog\", \"min_x\":13, \"min_y\":14, \"width\": 100, \"height\": 100}";
            api::remote::staging::stage_modification(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                data.to_string(),
                ContentType::Json,
                ModType::Append
            ).await?;

            let diff = api::remote::staging::diff_staged_file(
                &remote_repo,
                branch_name,
                &identifier,
                &path,
                DEFAULT_PAGE_NUM,
                DEFAULT_PAGE_SIZE
            ).await?;

            let added_rows = diff.added_rows.unwrap();
            assert_eq!(added_rows.height(), 1);
            assert_eq!(added_rows.width(), 7); // 6+1 for _id

            Ok(remote_repo)
        })
        .await
    }
}
