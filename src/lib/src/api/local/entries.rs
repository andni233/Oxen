use crate::index::Committer;
use crate::model::{Commit, CommitEntry, LocalRepository};

use crate::error::OxenError;

pub fn list_all(repo: &LocalRepository, commit: &Commit) -> Result<Vec<CommitEntry>, OxenError> {
    let committer = Committer::new(repo)?;
    let entries = committer.list_entries_for_commit(commit)?;
    Ok(entries)
}

pub fn count_for_commit(repo: &LocalRepository, commit: &Commit) -> Result<usize, OxenError> {
    let committer = Committer::new(repo)?;
    committer.num_entries_in_commit(&commit.id)
}

pub fn list_page(
    repo: &LocalRepository,
    commit: &Commit,
    page_num: usize,
    page_size: usize,
) -> Result<Vec<CommitEntry>, OxenError> {
    let committer = Committer::new(repo)?;
    let entries = committer.list_entry_page_for_commit(commit, page_num, page_size)?;
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use crate::api;
    use crate::command;
    use crate::error::OxenError;
    use crate::test;
    use crate::util;

    #[test]
    fn test_api_local_entries_list_all() -> Result<(), OxenError> {
        test::run_training_data_repo_test_no_commits(|repo| {
            // (file already created in helper)
            let file_to_add = repo.path.join("labels.txt");

            // Commit the file
            command::add(&repo, &file_to_add)?;
            let commit = command::commit(&repo, "Adding labels file")?.unwrap();

            let entries = api::local::entries::list_all(&repo, &commit)?;
            assert_eq!(entries.len(), 1);

            Ok(())
        })
    }

    #[test]
    fn test_api_local_entries_count_one_for_commit() -> Result<(), OxenError> {
        test::run_training_data_repo_test_no_commits(|repo| {
            // (file already created in helper)
            let file_to_add = repo.path.join("labels.txt");

            // Commit the file
            command::add(&repo, &file_to_add)?;
            let commit = command::commit(&repo, "Adding labels file")?.unwrap();

            let count = api::local::entries::count_for_commit(&repo, &commit)?;
            assert_eq!(count, 1);

            Ok(())
        })
    }

    #[test]
    fn test_api_local_entries_count_many_for_commit() -> Result<(), OxenError> {
        test::run_training_data_repo_test_no_commits(|repo| {
            // (files already created in helper)
            let dir_to_add = repo.path.join("train");
            let num_files = util::fs::rcount_files_in_dir(&dir_to_add);

            // Commit the dir
            command::add(&repo, &dir_to_add)?;
            let commit = command::commit(&repo, "Adding training data")?.unwrap();

            let count = api::local::entries::count_for_commit(&repo, &commit)?;
            assert_eq!(count, num_files);

            Ok(())
        })
    }

    #[test]
    fn test_api_local_entries_list_page_first_page() -> Result<(), OxenError> {
        test::run_training_data_repo_test_no_commits(|repo| {
            // (files already created in helper)
            let dir_to_add = repo.path.join("train");

            // Commit the dir
            command::add(&repo, &dir_to_add)?;
            let commit = command::commit(&repo, "Adding training data")?.unwrap();

            let page_size = 3;
            let entries = api::local::entries::list_page(&repo, &commit, 1, page_size)?;
            assert_eq!(entries.len(), page_size);

            Ok(())
        })
    }
}