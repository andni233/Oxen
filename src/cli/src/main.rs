use clap::{arg, Command};

pub mod dispatch;

fn main() {
    // Here is another example with set of commands
    // https://github.com/rust-in-action/code/blob/1st-edition/ch9/ch9-clock1/src/main.rs
    let matches = Command::new("oxen")
        .version("0.0.1")
        .about("Data management toolchain")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .allow_invalid_utf8_for_external_subcommands(true)
        .subcommand(
            Command::new("init")
                .about("Initializes a local repository")
                .arg(arg!(<PATH> "The directory to establish the repo in"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("set-remote")
                .about("Sets remote url for repository")
                .arg(arg!(<URL> "The remote url"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("status")
                .about("See at what files are ready to be added or committed")
        )
        .subcommand(
            Command::new("log")
                .about("See log of commits")
        )
        .subcommand(
            Command::new("add")
                .about("Adds the specified files or directories")
                .arg(arg!(<PATH> ... "The files or directory to add"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("ls")
                .about("Lists the directories within a repo")
                .arg_required_else_help(true)
                .arg(arg!(<OBJECT> "Run ls locally or remote (remote, local)")),
        )
        .subcommand(
            Command::new("clone")
                .about("Clone a repository by its URL")
                .arg_required_else_help(true)
                .arg(arg!(<URL> "URL of the repository you want to clone")),
        )
        .subcommand(
            Command::new("push")
                .about("Push the current branch up to the remote repository")
        )
        .subcommand(
            Command::new("pull")
                .about("Pull the files up from a remote branch")
                // .arg(arg!(<REMOTE_OR_BRANCH> "Name of remote or branch to pull from"))
                // .arg(arg!(<BRANCH> "Name of branch to pull from")),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("init", sub_matches)) => {
            let path = sub_matches.value_of("PATH").expect("required");

            match dispatch::init(path) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("{}", err)
                }
            }
        }
        Some(("set-remote", sub_matches)) => {
            let url = sub_matches.value_of("URL").expect("required");

            match dispatch::set_remote(url) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("{}", err)
                }
            }
        }
        Some(("status", _sub_matches)) => match dispatch::status() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        },
        Some(("log", _sub_matches)) => match dispatch::log_commits() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        },
        Some(("add", sub_matches)) => {
            let path = sub_matches.value_of("PATH").expect("required");

            match dispatch::add(path) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("{}", err)
                }
            }
        }
        Some(("push", _sub_matches)) => match dispatch::push() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        },
        Some(("pull", _sub_matches)) => match dispatch::pull() {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err)
            }
        },
        Some(("ls", sub_matches)) => {
            let object_type = sub_matches.value_of("OBJECT").unwrap_or_default();
            let result = match object_type {
                "remote" => dispatch::list_datasets(),
                _ => {
                    println!("Unknown object type: {}", object_type);
                    Ok(())
                }
            };
            match result {
                Ok(_) => {}
                Err(err) => {
                    println!("Err: {}", err)
                }
            }
        }
        Some(("clone", sub_matches)) => {
            let url = sub_matches.value_of("URL").expect("required");
            match dispatch::clone(url) {
                Ok(_) => {}
                Err(err) => {
                    println!("Err: {}", err)
                }
            }
        }
        // TODO: Get these in the help command instead of just falling back
        Some((ext, sub_matches)) => {
            let args = sub_matches
                .values_of_os("")
                .unwrap_or_default()
                .collect::<Vec<_>>();

            match ext {
                "login" => dispatch::login(),
                "commit" => dispatch::commit(args),
                "create" => dispatch::create(args),
                _ => {
                    println!("Unknown command {}", ext);
                    Ok(())
                }
            }
            .unwrap_or_default()
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachabe!()
    }
}