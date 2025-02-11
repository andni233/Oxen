use crate::api;
use crate::api::remote::client;
use crate::error::OxenError;
use crate::model::{Branch, Commit, LocalRepository, RemoteRepository};
use crate::view::{BranchNewFromExisting, BranchResponse, ListBranchesResponse, StatusMessage};
use serde_json::json;

pub async fn get_by_name(
    repository: &RemoteRepository,
    branch_name: &str,
) -> Result<Option<Branch>, OxenError> {
    let uri = format!("/branches/{branch_name}");
    let url = api::endpoint::url_from_repo(repository, &uri)?;

    let client = client::new_for_url(&url)?;
    if let Ok(res) = client.get(&url).send().await {
        let status = res.status();
        if 404 == status {
            return Ok(None);
        }

        let body = client::parse_json_body(&url, res).await?;
        let response: Result<BranchResponse, serde_json::Error> = serde_json::from_str(&body);
        match response {
            Ok(j_res) => Ok(Some(j_res.branch)),
            Err(err) => {
                log::debug!(
                    "remote::branches::get_by_name() Could not deserialize response [{}] {}",
                    err,
                    body
                );
                Ok(None)
            }
        }
    } else {
        let err = "Failed to get branch";
        log::error!("remote::branches::get_by_name() err: {}", err);
        Err(OxenError::basic_str(err))
    }
}

pub async fn create_from_or_get(
    repository: &RemoteRepository,
    new_name: &str,
    from_name: &str,
) -> Result<Branch, OxenError> {
    let url = api::endpoint::url_from_repo(repository, "/branches")?;
    log::debug!("create_from_or_get {}", url);

    let params = serde_json::to_string(&BranchNewFromExisting {
        new_name: new_name.to_string(),
        from_name: from_name.to_string(),
    })?;

    let client = client::new_for_url(&url)?;
    if let Ok(res) = client.post(&url).body(params).send().await {
        let body = client::parse_json_body(&url, res).await?;
        let response: Result<BranchResponse, serde_json::Error> = serde_json::from_str(&body);
        match response {
            Ok(response) => Ok(response.branch),
            Err(err) => {
                let err = format!(
                    "Could not find branch [{}] or create it from branch [{}]: {}\n{}",
                    new_name, from_name, err, body
                );
                Err(OxenError::basic_str(err))
            }
        }
    } else {
        let msg = format!("Could not create branch {new_name}");
        log::error!("remote::branches::create_from_or_get() {}", msg);
        Err(OxenError::basic_str(&msg))
    }
}

pub async fn list(repository: &RemoteRepository) -> Result<Vec<Branch>, OxenError> {
    let url = api::endpoint::url_from_repo(repository, "/branches")?;

    let client = client::new_for_url(&url)?;
    if let Ok(res) = client.get(&url).send().await {
        let body = client::parse_json_body(&url, res).await?;
        let response: Result<ListBranchesResponse, serde_json::Error> = serde_json::from_str(&body);
        match response {
            Ok(j_res) => Ok(j_res.branches),
            Err(err) => {
                log::debug!(
                    "remote::branches::list() Could not deserialize response [{}] {}",
                    err,
                    body
                );
                Err(OxenError::basic_str("Could not list remote branches"))
            }
        }
    } else {
        let err = "Failed to list branches";
        log::error!("remote::branches::list() err: {}", err);
        Err(OxenError::basic_str(err))
    }
}

pub async fn update(
    repository: &RemoteRepository,
    branch_name: &str,
    commit: &Commit,
) -> Result<Branch, OxenError> {
    let uri = format!("/branches/{branch_name}");
    let url = api::endpoint::url_from_repo(repository, &uri)?;
    log::debug!("remote::branches::update url: {}", url);

    let params = serde_json::to_string(&json!({ "commit_id": commit.id }))?;

    let client = client::new_for_url(&url)?;
    if let Ok(res) = client.put(&url).body(params).send().await {
        let body = client::parse_json_body(&url, res).await?;
        let response: Result<BranchResponse, serde_json::Error> = serde_json::from_str(&body);
        match response {
            Ok(response) => Ok(response.branch),
            Err(err) => {
                let err = format!(
                    "Could not update branch [{}]: {}\n{}",
                    repository.name, err, body
                );
                Err(OxenError::basic_str(err))
            }
        }
    } else {
        let msg = format!("Could not update branch {branch_name}");
        log::error!("remote::branches::update() {}", msg);
        Err(OxenError::basic_str(&msg))
    }
}

/// # Delete a remote branch
pub async fn delete_remote(
    repo: &LocalRepository,
    remote: &str,
    branch_name: &str,
) -> Result<(), OxenError> {
    if let Some(remote) = repo.get_remote(remote) {
        if let Some(remote_repo) = api::remote::repositories::get_by_remote(&remote).await? {
            if let Some(branch) =
                api::remote::branches::get_by_name(&remote_repo, branch_name).await?
            {
                api::remote::branches::delete(&remote_repo, &branch.name).await?;
                Ok(())
            } else {
                Err(OxenError::remote_branch_not_found(branch_name))
            }
        } else {
            Err(OxenError::remote_repo_not_found(&remote.url))
        }
    } else {
        Err(OxenError::remote_not_set(remote))
    }
}

pub async fn delete(
    repository: &RemoteRepository,
    branch_name: &str,
) -> Result<StatusMessage, OxenError> {
    let uri = format!("/branches/{branch_name}");
    let url = api::endpoint::url_from_repo(repository, &uri)?;
    log::debug!("Deleting branch: {}", url);

    let client = client::new_for_url(&url)?;
    if let Ok(res) = client.delete(&url).send().await {
        let body = client::parse_json_body(&url, res).await?;
        let response: Result<StatusMessage, serde_json::Error> = serde_json::from_str(&body);
        match response {
            Ok(val) => Ok(val),
            Err(_) => Err(OxenError::basic_str(format!(
                "could not delete branch \n\n{body}"
            ))),
        }
    } else {
        Err(OxenError::basic_str(
            "api::branches::delete() Request failed",
        ))
    }
}

#[cfg(test)]
mod tests {

    use crate::api;
    use crate::constants::DEFAULT_BRANCH_NAME;
    use crate::error::OxenError;
    use crate::test;

    #[tokio::test]
    async fn test_create_remote_branch() -> Result<(), OxenError> {
        test::run_empty_remote_repo_test(|_local_repo, remote_repo| async move {
            let name = "my-branch";
            let branch =
                api::remote::branches::create_from_or_get(&remote_repo, name, "main").await?;
            assert_eq!(branch.name, name);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_create_remote_branch_from_existing() -> Result<(), OxenError> {
        test::run_empty_remote_repo_test(|_local_repo, remote_repo| async move {
            let name = "my-branch";
            let from = "old-branch";
            api::remote::branches::create_from_or_get(&remote_repo, from, DEFAULT_BRANCH_NAME)
                .await?;
            let branch =
                api::remote::branches::create_from_or_get(&remote_repo, name, from).await?;
            assert_eq!(branch.name, name);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_get_branch_by_name() -> Result<(), OxenError> {
        test::run_empty_remote_repo_test(|_local_repo, remote_repo| async move {
            let branch_name = "my-branch";
            api::remote::branches::create_from_or_get(&remote_repo, branch_name, "main").await?;

            let branch = api::remote::branches::get_by_name(&remote_repo, branch_name).await?;
            assert!(branch.is_some());
            assert_eq!(branch.unwrap().name, branch_name);

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_list_remote_branches() -> Result<(), OxenError> {
        test::run_empty_remote_repo_test(|_local_repo, remote_repo| async move {
            api::remote::branches::create_from_or_get(
                &remote_repo,
                "branch-1",
                DEFAULT_BRANCH_NAME,
            )
            .await?;
            api::remote::branches::create_from_or_get(
                &remote_repo,
                "branch-2",
                DEFAULT_BRANCH_NAME,
            )
            .await?;

            let branches = api::remote::branches::list(&remote_repo).await?;
            assert_eq!(branches.len(), 3);

            assert!(branches.iter().any(|b| b.name == "branch-1"));
            assert!(branches.iter().any(|b| b.name == "branch-2"));
            assert!(branches.iter().any(|b| b.name == DEFAULT_BRANCH_NAME));

            Ok(remote_repo)
        })
        .await
    }

    #[tokio::test]
    async fn test_delete_branch() -> Result<(), OxenError> {
        test::run_empty_remote_repo_test(|_local_repo, remote_repo| async move {
            let branch_name = "my-branch";
            api::remote::branches::create_from_or_get(
                &remote_repo,
                branch_name,
                DEFAULT_BRANCH_NAME,
            )
            .await?;

            let branch = api::remote::branches::get_by_name(&remote_repo, branch_name).await?;
            assert!(branch.is_some());
            let branch = branch.unwrap();
            assert_eq!(branch.name, branch_name);

            api::remote::branches::delete(&remote_repo, branch_name).await?;

            let deleted_branch =
                api::remote::branches::get_by_name(&remote_repo, branch_name).await?;
            assert!(deleted_branch.is_none());

            Ok(remote_repo)
        })
        .await
    }
}
