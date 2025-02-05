use actix_web::web;
use liboxen::opts::DFOpts;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct DFOptsQuery {
    pub slice: Option<String>,
    pub take: Option<String>,
    pub delimiter: Option<String>,
    pub columns: Option<String>,
    pub filter: Option<String>,
    pub aggregate: Option<String>,
    pub sort_by: Option<String>,
    pub randomize: Option<bool>,
    pub reverse: Option<bool>,
    pub page: Option<usize>,
    pub page_size: Option<usize>,
}

/// Provide some default vals for opts
pub fn parse_opts(query: &web::Query<DFOptsQuery>, filter_ops: &mut DFOpts) -> DFOpts {
    // Default to 0..10 unless they ask for "all"
    if let Some(slice) = query.slice.clone() {
        if slice == "all" {
            // Return everything...probably don't want to do this unless explicitly asked for
            filter_ops.slice = None;
        } else {
            // Return what they asked for
            filter_ops.slice = Some(slice);
        }
    } else {
        // No slice val supplied, only return first 10
        filter_ops.slice = Some(String::from("0..10"));
    }

    // we are already filtering the hidden columns
    if let Some(columns) = query.columns.clone() {
        filter_ops.columns = Some(columns);
    }

    filter_ops.page = query.page;
    filter_ops.page_size = query.page_size;
    filter_ops.delimiter = query.delimiter.clone();
    filter_ops.take = query.take.clone();
    filter_ops.filter = query.filter.clone();
    filter_ops.aggregate = query.aggregate.clone();
    filter_ops.sort_by = query.sort_by.clone();
    filter_ops.should_randomize = query.randomize.unwrap_or(false);
    filter_ops.should_reverse = query.reverse.unwrap_or(false);

    filter_ops.clone()
}
