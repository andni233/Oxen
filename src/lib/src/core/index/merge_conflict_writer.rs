//! Writes merge conflicts to database
//!

use crate::constants::{MERGE_HEAD_FILE, ORIG_HEAD_FILE};
use crate::core::db;
use crate::error::OxenError;
use crate::model::{Commit, LocalRepository, MergeConflict};
use crate::util;

use rocksdb::DB;

use super::merger;

/// Creates the DB object then calls write_conflicts_to_disk
pub fn write_conflicts_to_db(
    repo: &LocalRepository,
    merge_commit: &Commit,
    base_commit: &Commit,
    conflicts: &[MergeConflict],
) -> Result<(), OxenError> {
    let db_path = merger::db_path(repo);
    let opts = db::opts::default();
    let db = DB::open(&opts, dunce::simplified(&db_path))?;

    write_conflicts_to_disk(repo, &db, merge_commit, base_commit, conflicts)
}

/// Writes merge conflicts to database, and write merge commit and head commit to disk
pub fn write_conflicts_to_disk(
    repo: &LocalRepository,
    db: &DB,
    merge_commit: &Commit,
    base_commit: &Commit,
    conflicts: &[MergeConflict],
) -> Result<(), OxenError> {
    // Write two files which are the merge commit and head commit so that we can make these parents later
    let hidden_dir = util::fs::oxen_hidden_dir(&repo.path);
    let merge_head_path = hidden_dir.join(MERGE_HEAD_FILE);
    let orig_head_path = hidden_dir.join(ORIG_HEAD_FILE);
    util::fs::write_to_path(&merge_head_path, &merge_commit.id)?;
    util::fs::write_to_path(&orig_head_path, &base_commit.id)?;

    for conflict in conflicts.iter() {
        let key = conflict.base_entry.path.to_str().unwrap();
        let key_bytes = key.as_bytes();
        let val_json = serde_json::to_string(&conflict)?;

        db.put(key_bytes, val_json.as_bytes())?;
    }

    Ok(())
}
