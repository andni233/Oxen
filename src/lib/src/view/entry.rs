use crate::{
    model::{CommitEntry, DirEntry, RemoteEntry},
    util,
};
use serde::{Deserialize, Serialize};

use super::StatusMessage;

#[derive(Deserialize, Serialize, Debug)]
pub struct EntryResponse {
    #[serde(flatten)]
    pub status: StatusMessage,
    pub entry: CommitEntry,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RemoteEntryResponse {
    #[serde(flatten)]
    pub status: StatusMessage,
    pub entry: RemoteEntry,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ResourceVersion {
    pub path: String,
    pub version: String,
}

impl ResourceVersion {
    pub fn from_parsed_resource(resource: &crate::model::ParsedResource) -> ResourceVersion {
        ResourceVersion {
            path: resource.file_path.to_string_lossy().to_string(),
            version: resource.version(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PaginatedEntries {
    #[serde(flatten)]
    pub status: StatusMessage,
    pub entries: Vec<RemoteEntry>,
    pub page_size: usize,
    pub page_number: usize,
    pub total_pages: usize,
    pub total_entries: usize,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PaginatedDirEntries {
    #[serde(flatten)]
    pub status: StatusMessage,
    pub entries: Vec<DirEntry>,
    pub resource: Option<ResourceVersion>,
    pub page_size: usize,
    pub page_number: usize,
    pub total_pages: usize,
    pub total_entries: usize,
}

impl PaginatedDirEntries {
    pub fn from_entries(
        entries: Vec<DirEntry>,
        resource: Option<ResourceVersion>,
        page_num: usize,
        page_size: usize,
        total: usize,
    ) -> PaginatedDirEntries {
        log::debug!(
            "PaginatedDirEntries::from_entries entries.len() {} page_num {} page_size {} total {} ",
            entries.len(),
            page_num,
            page_size,
            total
        );

        let (paginated, total_pages) = util::paginate(entries, page_num, page_size);
        PaginatedDirEntries {
            status: StatusMessage::resource_found(),
            entries: paginated,
            resource,
            page_size,
            page_number: page_num,
            total_pages,
            total_entries: total,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PaginatedDirEntriesResponse {
    #[serde(flatten)]
    pub status: StatusMessage,
    pub entries: Vec<DirEntry>,
    pub resource: Option<ResourceVersion>,
    pub page_size: usize,
    pub page_number: usize,
    pub total_pages: usize,
    pub total_entries: usize,
}

impl PaginatedDirEntriesResponse {
    pub fn ok_from(paginated: PaginatedDirEntries) -> Self {
        Self {
            status: StatusMessage::resource_found(),
            entries: paginated.entries,
            resource: paginated.resource,
            page_size: paginated.page_size,
            page_number: paginated.page_number,
            total_pages: paginated.total_pages,
            total_entries: paginated.total_entries,
        }
    }
}
