use clap::ArgMatches;
use liboxen::opts::RestoreOpts;
use liboxen::util;
use std::path::{Path, PathBuf};

use crate::dispatch;
use liboxen::constants::{DEFAULT_BRANCH_NAME, DEFAULT_REMOTE_NAME};

pub fn init(sub_matches: &ArgMatches) {
    let path = sub_matches.value_of("PATH").unwrap_or(".");

    match dispatch::init(path) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

pub fn config(sub_matches: &ArgMatches) {
    if let Some(auth) = sub_matches.values_of("auth-token") {
        if let [host, token] = auth.collect::<Vec<_>>()[..] {
            match dispatch::set_auth_token(host, token) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("{}", err)
                }
            }
        } else {
            eprintln!("invalid arguments for --auth");
        }
    }

    if let Some(name) = sub_matches.value_of("name") {
        match dispatch::set_user_name(name) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    }

    if let Some(email) = sub_matches.value_of("email") {
        match dispatch::set_user_email(email) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    }
}

pub async fn create_remote(sub_matches: &ArgMatches) {
    let namespace = sub_matches.value_of("NAMESPACE").expect("required");
    let name = sub_matches.value_of("NAME").expect("required");
    let host = sub_matches.value_of("HOST").expect("required");

    match dispatch::create_remote(namespace, name, host).await {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

pub fn remote(sub_matches: &ArgMatches) {
    if let Some(subcommand) = sub_matches.subcommand() {
        match subcommand {
            ("add", sub_matches) => {
                let name = sub_matches.value_of("NAME").expect("required");
                let url = sub_matches.value_of("URL").expect("required");

                match dispatch::add_remote(name, url) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{}", err)
                    }
                }
            }
            ("remove", sub_matches) => {
                let name = sub_matches.value_of("NAME").expect("required");

                match dispatch::remove_remote(name) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{}", err)
                    }
                }
            }
            (command, _) => {
                eprintln!("Invalid subcommand: {}", command)
            }
        }
    } else if sub_matches.is_present("verbose") {
        dispatch::list_remotes_verbose().expect("Unable to list remotes.");
    } else {
        dispatch::list_remotes().expect("Unable to list remotes.");
    }
}

pub fn status(sub_matches: &ArgMatches) {
    let skip: usize = sub_matches
        .value_of("skip")
        .unwrap_or("0")
        .parse::<usize>()
        .expect("Skip must be a valid integer.");
    let limit: usize = sub_matches
        .value_of("limit")
        .unwrap_or("10")
        .parse::<usize>()
        .expect("Limit must be a valid integer.");
    let print_all = sub_matches.is_present("print_all");

    match dispatch::status(skip, limit, print_all) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err);
        }
    }
}

pub fn log() {
    match dispatch::log_commits() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

fn parse_df_sub_matches(sub_matches: &ArgMatches) -> liboxen::df::DFOpts {
    let vstack: Option<Vec<PathBuf>> = if let Some(vstack) = sub_matches.values_of("vstack") {
        let values: Vec<PathBuf> = vstack.map(std::path::PathBuf::from).collect();
        Some(values)
    } else {
        None
    };

    liboxen::df::DFOpts {
        output: sub_matches.value_of("output").map(std::path::PathBuf::from),
        slice: sub_matches.value_of("slice").map(String::from),
        take: sub_matches.value_of("take").map(String::from),
        columns: sub_matches.value_of("columns").map(String::from),
        filter: sub_matches.value_of("filter").map(String::from),
        aggregate: sub_matches.value_of("aggregate").map(String::from),
        col_at: sub_matches.value_of("col_at").map(String::from),
        vstack,
        add_col: sub_matches.value_of("add_col").map(String::from),
        add_row: sub_matches.value_of("add_row").map(String::from),
        sort_by: sub_matches.value_of("sort").map(String::from),
        unique: sub_matches.value_of("unique").map(String::from),
        should_randomize: sub_matches.is_present("randomize"),
        should_reverse: sub_matches.is_present("reverse"),
    }
}

pub fn df(sub_matches: &ArgMatches) {
    let path = sub_matches.value_of("DF_SPEC").expect("required");
    if sub_matches.is_present("schema") {
        match dispatch::df_schema(path) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    } else {
        let opts = parse_df_sub_matches(sub_matches);

        match dispatch::df(path, opts) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    }
}

pub fn schemas(sub_matches: &ArgMatches) {
    if let Some(subcommand) = sub_matches.subcommand() {
        match subcommand {
            ("list", _sub_matches) => match dispatch::schema_list() {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("{}", err)
                }
            },
            ("show", sub_matches) => {
                let val = sub_matches.value_of("NAME_OR_HASH").expect("required");
                let verbose = true; // do not print schema
                match dispatch::schema_show(val, verbose) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{}", err)
                    }
                }
            }
            ("name", sub_matches) => {
                let hash = sub_matches.value_of("HASH").expect("required");
                let val = sub_matches.value_of("NAME").expect("required");
                match dispatch::schema_name(hash, val) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{}", err)
                    }
                }
            }
            ("create_index", sub_matches) => {
                let schema = sub_matches
                    .value_of("SCHEMA")
                    .expect("Must supply a schema ref");
                let field = sub_matches.value_of("field").expect("Must supply a field");
                match dispatch::schema_create_index(schema, field) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{}", err)
                    }
                }
            }
            ("indices", sub_matches) => {
                let schema = sub_matches
                    .value_of("SCHEMA")
                    .expect("Must supply a schema ref");

                match dispatch::schema_list_indices(schema) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{}", err)
                    }
                }
            }
            ("query", sub_matches) => {
                let schema = sub_matches.value_of("SCHEMA").expect("required");
                let field = sub_matches.value_of("field").expect("required");
                let query = sub_matches.value_of("query").expect("required");
                match dispatch::schema_query_index(schema, field, query) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("{}", err)
                    }
                }
            }
            ("df", sub_matches) => {
                let schema = sub_matches.value_of("DF_SPEC").expect("required");
                let verbose = false; // do not print schema
                match dispatch::schema_show(schema, verbose) {
                    Ok((repo, Some(schema))) => {
                        let path = liboxen::util::fs::schema_df_path(&repo, &schema);
                        let opts = parse_df_sub_matches(sub_matches);

                        match dispatch::df(path, opts) {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("{}", err)
                            }
                        }
                    }
                    Ok((_, None)) => {
                        eprintln!("Could not find schema {}", schema);
                    }
                    Err(err) => {
                        eprintln!("{}", err)
                    }
                }
            }
            (cmd, _) => {
                eprintln!("Unknown subcommand {}", cmd)
            }
        }
    } else {
        match dispatch::schema_list() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    }
}

pub fn add(sub_matches: &ArgMatches) {
    let paths: Vec<PathBuf> = sub_matches
        .values_of("files")
        .expect("Must supply files")
        .map(PathBuf::from)
        .collect();

    match dispatch::add(paths) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

pub fn rm(sub_matches: &ArgMatches) {
    let paths: Vec<PathBuf> = sub_matches
        .values_of("files")
        .expect("Must supply files")
        .map(PathBuf::from)
        .collect();

    match dispatch::rm(paths) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

pub fn restore(sub_matches: &ArgMatches) {
    let path = sub_matches.value_of("PATH").expect("required");

    let opts = if let Some(source) = sub_matches.value_of("source") {
        RestoreOpts {
            path: PathBuf::from(path),
            staged: sub_matches.is_present("staged"),
            source_ref: Some(String::from(source)),
        }
    } else {
        RestoreOpts {
            path: PathBuf::from(path),
            staged: sub_matches.is_present("staged"),
            source_ref: None,
        }
    };

    match dispatch::restore(opts) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

pub async fn branch(sub_matches: &ArgMatches) {
    if sub_matches.is_present("all") {
        if let Err(err) = dispatch::list_all_branches().await {
            eprintln!("{}", err)
        }
    } else if let Some(remote_name) = sub_matches.value_of("remote") {
        if let Some(branch_name) = sub_matches.value_of("delete") {
            if let Err(err) = dispatch::delete_remote_branch(remote_name, branch_name).await {
                eprintln!("{}", err)
            }
        } else if let Err(err) = dispatch::list_remote_branches(remote_name).await {
            eprintln!("{}", err)
        }
    } else if let Some(name) = sub_matches.value_of("name") {
        if let Err(err) = dispatch::create_branch(name) {
            eprintln!("{}", err)
        }
    } else if let Some(name) = sub_matches.value_of("delete") {
        if let Err(err) = dispatch::delete_branch(name) {
            eprintln!("{}", err)
        }
    } else if let Some(name) = sub_matches.value_of("force-delete") {
        if let Err(err) = dispatch::force_delete_branch(name) {
            eprintln!("{}", err)
        }
    } else if sub_matches.is_present("show-current") {
        if let Err(err) = dispatch::show_current_branch() {
            eprintln!("{}", err)
        }
    } else if let Err(err) = dispatch::list_branches() {
        eprintln!("{}", err)
    }
}

pub fn checkout(sub_matches: &ArgMatches) {
    if sub_matches.is_present("create") {
        let name = sub_matches.value_of("create").expect("required");
        if let Err(err) = dispatch::create_checkout_branch(name) {
            eprintln!("{}", err)
        }
    } else if sub_matches.is_present("theirs") {
        let name = sub_matches.value_of("name").expect("required");
        if let Err(err) = dispatch::checkout_theirs(name) {
            eprintln!("{}", err)
        }
    } else if sub_matches.is_present("name") {
        let name = sub_matches.value_of("name").expect("required");
        if let Err(err) = dispatch::checkout(name) {
            eprintln!("{}", err)
        }
    } else {
        eprintln!("Err: Usage `oxen checkout <name>`");
    }
}

pub fn merge(sub_matches: &ArgMatches) {
    let branch = sub_matches
        .value_of("BRANCH")
        .unwrap_or(DEFAULT_BRANCH_NAME);
    match dispatch::merge(branch) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

pub async fn push(sub_matches: &ArgMatches) {
    let remote = sub_matches
        .value_of("REMOTE")
        .unwrap_or(DEFAULT_REMOTE_NAME);
    let branch = sub_matches
        .value_of("BRANCH")
        .unwrap_or(DEFAULT_BRANCH_NAME);

    if sub_matches.is_present("delete") {
        println!("Delete remote branch {}/{}", remote, branch);
    } else {
        match dispatch::push(remote, branch).await {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    }
}

pub async fn pull(sub_matches: &ArgMatches) {
    let remote = sub_matches
        .value_of("REMOTE")
        .unwrap_or(DEFAULT_REMOTE_NAME);
    let branch = sub_matches
        .value_of("BRANCH")
        .unwrap_or(DEFAULT_BRANCH_NAME);
    match dispatch::pull(remote, branch).await {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

pub fn diff(sub_matches: &ArgMatches) {
    // First arg is optional
    let file_or_commit_id = sub_matches.value_of("FILE_OR_COMMIT_ID").expect("required");
    let path = sub_matches.value_of("PATH");
    if let Some(path) = path {
        match dispatch::diff(Some(file_or_commit_id), path) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    } else {
        match dispatch::diff(None, file_or_commit_id) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        }
    }
}

pub async fn clone(sub_matches: &ArgMatches) {
    let url = sub_matches.value_of("URL").expect("required");
    match dispatch::clone(url).await {
        Ok(_) => {}
        Err(err) => {
            println!("Err: {}", err)
        }
    }
}

pub fn commit(sub_matches: &ArgMatches) {
    let args = sub_matches
        .values_of_os("")
        .unwrap_or_default()
        .collect::<Vec<_>>();

    match dispatch::commit(args) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{}", err)
        }
    }
}

pub fn kvdb_inspect(sub_matches: &ArgMatches) {
    let path_str = sub_matches.value_of("PATH").expect("required");
    let path = Path::new(path_str);
    match dispatch::inspect(path) {
        Ok(_) => {}
        Err(err) => {
            println!("Err: {}", err)
        }
    }
}

pub fn read_lines(sub_matches: &ArgMatches) {
    let path_str = sub_matches.value_of("PATH").expect("required");
    let start: usize = sub_matches
        .value_of("START")
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap();
    let length: usize = sub_matches
        .value_of("LENGTH")
        .unwrap_or("10")
        .parse::<usize>()
        .unwrap();

    let path = Path::new(path_str);
    let (lines, size) = util::fs::read_lines_paginated_ret_size(path, start, length);
    for line in lines.iter() {
        println!("{}", line);
    }
    println!("Total: {}", size);
}