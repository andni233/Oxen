use crate::errors::OxenHttpError;
use crate::helpers::get_repo;
use crate::params::{app_data, parse_resource, path_param};
use crate::view::PaginatedLinesResponse;

use liboxen::constants::AVG_CHUNK_SIZE;
use liboxen::util;
use liboxen::view::http::{MSG_RESOURCE_FOUND, STATUS_SUCCESS};
use liboxen::{constants, current_function};

use actix_web::{web, HttpRequest, HttpResponse};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures_util::stream::StreamExt as _;
use serde::Deserialize;

use std::fs::File;
use std::io::prelude::*;

#[derive(Deserialize, Debug)]
pub struct PageNumQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct ChunkQuery {
    pub chunk_start: Option<u64>,
    pub chunk_size: Option<u64>,
}

pub async fn download_data_from_version_paths(
    req: HttpRequest,
    mut body: web::Payload,
) -> actix_web::Result<HttpResponse, OxenHttpError> {
    let app_data = app_data(&req)?;
    let namespace = path_param(&req, "namespace")?;
    let repo_name = path_param(&req, "repo_name")?;
    let repo = get_repo(&app_data.path, namespace, &repo_name)?;

    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item.unwrap());
    }
    log::debug!(
        "{} got repo [{}] and content_ids size {}",
        current_function!(),
        repo_name,
        bytes.len()
    );

    let mut gz = GzDecoder::new(&bytes[..]);
    let mut line_delimited_files = String::new();
    gz.read_to_string(&mut line_delimited_files).unwrap();

    let content_files: Vec<&str> = line_delimited_files.split('\n').collect();

    let enc = GzEncoder::new(Vec::new(), Compression::default());
    let mut tar = tar::Builder::new(enc);

    log::debug!("Got {} content ids", content_files.len());
    for content_file in content_files.iter() {
        if content_file.is_empty() {
            // last line might be empty on split \n
            continue;
        }

        let version_path = repo.path.join(content_file);
        if version_path.exists() {
            tar.append_path_with_name(version_path, content_file)
                .unwrap();
        } else {
            log::error!(
                "Could not find content: {:?} -> {:?}",
                content_file,
                version_path
            );
        }
    }

    tar.finish().unwrap();
    let buffer: Vec<u8> = tar.into_inner().unwrap().finish().unwrap();
    Ok(HttpResponse::Ok().body(buffer))
}

/// Download a chunk of a larger file
pub async fn download_chunk(
    req: HttpRequest,
    query: web::Query<ChunkQuery>,
) -> actix_web::Result<HttpResponse, OxenHttpError> {
    let app_data = app_data(&req)?;
    let namespace = path_param(&req, "namespace")?;
    let repo_name = path_param(&req, "repo_name")?;
    let repo = get_repo(&app_data.path, namespace, &repo_name)?;
    let resource = parse_resource(&req, &repo)?;

    log::debug!(
        "{} resource {}/{}",
        current_function!(),
        repo_name,
        resource
    );

    let version_path =
        util::fs::version_path_for_commit_id(&repo, &resource.commit.id, &resource.file_path)?;
    let chunk_start: u64 = query.chunk_start.unwrap_or(0);
    let chunk_size: u64 = query.chunk_size.unwrap_or(AVG_CHUNK_SIZE);

    let mut f = File::open(version_path).unwrap();
    f.seek(std::io::SeekFrom::Start(chunk_start)).unwrap();
    let mut buffer = vec![0u8; chunk_size as usize];
    f.read_exact(&mut buffer).unwrap();

    Ok(HttpResponse::Ok().body(buffer))
}

pub async fn list_lines_in_file(
    req: HttpRequest,
    query: web::Query<PageNumQuery>,
) -> actix_web::Result<HttpResponse, OxenHttpError> {
    let app_data = app_data(&req)?;
    let namespace = path_param(&req, "namespace")?;
    let repo_name = path_param(&req, "repo_name")?;
    let repo = get_repo(&app_data.path, namespace, &repo_name)?;
    let resource = parse_resource(&req, &repo)?;

    log::debug!(
        "{} resource {}/{}",
        current_function!(),
        repo_name,
        resource
    );

    // default to first page with first ten values
    let page: usize = query.page.unwrap_or(constants::DEFAULT_PAGE_NUM);
    let page_size: usize = query.page_size.unwrap_or(constants::DEFAULT_PAGE_SIZE);

    log::debug!(
        "{} page {} page_size {}",
        current_function!(),
        page,
        page_size,
    );

    let version_path =
        util::fs::version_path_for_commit_id(&repo, &resource.commit.id, &resource.file_path)?;
    let start = page * page_size;
    let (lines, total_entries) =
        liboxen::util::fs::read_lines_paginated_ret_size(&version_path, start, page_size);

    let total_pages = (total_entries as f64 / page_size as f64).ceil() as usize;
    Ok(HttpResponse::Ok().json(PaginatedLinesResponse {
        status: String::from(STATUS_SUCCESS),
        status_message: String::from(MSG_RESOURCE_FOUND),
        lines,
        page_size,
        page_number: page,
        total_pages,
        total_entries,
    }))
}
