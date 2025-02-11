//! # oxen remote commit
//!
//! Commit remote staged data on a branch
//!

use crate::api;
use crate::config::UserConfig;
use crate::error::OxenError;
use crate::model::{Commit, CommitBody, LocalRepository, User};

/// Commit changes that are staged on the remote repository on the current
/// checked out local branch
pub async fn commit(repo: &LocalRepository, message: &str) -> Result<Option<Commit>, OxenError> {
    let branch = api::local::branches::current_branch(repo)?;
    if branch.is_none() {
        return Err(OxenError::must_be_on_valid_branch());
    }
    let branch = branch.unwrap();

    let remote_repo = api::remote::repositories::get_default_remote(repo).await?;
    let cfg = UserConfig::get()?;
    let body = CommitBody {
        message: message.to_string(),
        user: User {
            name: cfg.name,
            email: cfg.email,
        },
    };
    let user_id = UserConfig::identifier()?;
    let commit =
        api::remote::staging::commit_staged(&remote_repo, &branch.name, &user_id, &body).await?;
    Ok(Some(commit))
}
