use crate::model::RemoteEntry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use filetime::FileTime;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CommitEntry {
    pub id: String,
    pub path: PathBuf,
    pub is_synced: bool,
    pub hash: String,
    pub last_modified_seconds: i64,
    pub last_modified_nanoseconds: u32,
}

impl CommitEntry {
    pub fn filename_from_commit_id(&self, commit_id: &str) -> PathBuf {
        PathBuf::from(format!("{}.{}", commit_id, self.extension()))
    }

    pub fn extension(&self) -> String {
        String::from(self.path.extension().unwrap().to_str().unwrap())
    }

    pub fn to_synced(&self) -> CommitEntry {
        CommitEntry {
            id: self.id.to_owned(),
            path: self.path.to_owned(),
            is_synced: true,
            hash: self.hash.to_owned(),
            last_modified_seconds: self.last_modified_seconds,
            last_modified_nanoseconds: self.last_modified_nanoseconds
        }
    }

    pub fn to_remote(&self) -> RemoteEntry {
        RemoteEntry {
            id: self.id.to_owned(),
            filename: self.path.to_str().unwrap_or("").to_string(),
            hash: self.hash.to_owned(),
        }
    }

    pub fn to_uri_encoded(&self) -> String {
        serde_url_params::to_string(&self).unwrap()
    }

    pub fn has_different_modification_time(&self, time: &FileTime) -> bool {
        self.last_modified_nanoseconds != time.nanoseconds() ||
        self.last_modified_seconds != time.unix_seconds()
    }
}
