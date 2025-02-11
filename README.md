# 🐂 Oxen

Create a world where everyone can contribute to an Artificial General Intelligence, starting with the data.

# 🌾 What is Oxen?

Oxen at it's core is a data version control library, written in Rust. It's goals are to be fast, reliable, and easy to use. It's designed to be used in a variety of ways, from a simple command line tool, to a remote server to sync to, to integrations into other ecosystems such as [python](https://github.com/Oxen-AI/oxen-release).

# 📚 Documentation

The documentation for liboxen is automatically generated and uploaded to [docs.rs](https://docs.rs/liboxen/latest/liboxen/).

# 🔨 Build & Run

First, make sure you have latest Rust version installed. You should install the Rust toolchain with rustup: https://www.rust-lang.org/tools/install.

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

If you are a developer and want to learn more about adding code or the overall architecture [start here](docs/dev/AddLibraryCode.md). Otherwise a quick start to make sure everything is working follows.

## Build

```
cargo build
```

If on intel mac, you may need to build with the following

```
$ rustup target install x86_64-apple-darwin
$ cargo build --target x86_64-apple-darwin
```

## Run

Generate a config file and token to give user access to the server

```
./target/debug/oxen-server add-user --email ox@oxen.ai --name Ox --output user_config.toml
```

Copy the config to the default locations

```
mkdir ~/.oxen
```

```
mv user_config.toml ~/.oxen/user_config.toml
```

```
cp ~/.oxen/user_config.toml data/test/config/user_config.toml
```

Set where you want the data to be synced to. The default sync directory is `/tmp/oxen_sync` to change it set the SYNC_DIR environment variable to a path.

```
export SYNC_DIR=/path/to/sync/dir
```

Run the server

```
./target/debug/oxen-server start
```

To run the server with live reload, first install cargo-watch

```
cargo install cargo-watch
```

Then run the server like this

```
cargo watch -- cargo run --bin oxen-server start
```

# Unit & Integration Tests

Make sure your server is running on the default port and host, then run

*Note:* tests open up a lot of file handles, so limit num test threads if running everything.

```
cargo test -- --test-threads=3
```

To run with all debug output and run a specific test

```
env RUST_LOG=warn,liboxen=debug,integration_test=debug cargo test -- --nocapture test_command_push_clone_pull_push
```

To set a different test host you can set the `OXEN_TEST_HOST` environment variable

```
env OXEN_TEST_HOST=0.0.0.0:4000 cargo test
```

# CLI Commands

```
oxen init .
oxen status
oxen add images/
oxen status
oxen commit -m "added images"
oxen push origin main
```

# Oxen Server

## Structure

Directories with repository names to simply sync data to, same internal file structure as your local repo

/tmp/oxen_sync
/repo_name

# APIs

Server defaults to localhost 3000

```
set SERVER 0.0.0.0:3000
```

You can grab your auth token from the config file above (~/.oxen/user_config.toml)

```
set TOKEN <YOUR_TOKEN>
```

## List Repositories

```
curl -H "Authorization: Bearer $TOKEN" "http://$SERVER/api/repos"
```

## Create Repository

```
curl -H "Authorization: Bearer $TOKEN" -X POST -d '{"name": "MyRepo"}' "http://$SERVER/api/repos"
```

# Docker

Create the docker image

```
docker build -t oxen/server:0.6.0 .
```

Run a container on port 3000 with a local filesystem mounted from /var/oxen/data on the host to /var/oxen/data in the container.

```
docker run -d -v /var/oxen/data:/var/oxen/data -p 3000:3001 --name oxen oxen/server:0.6.0
```

Or use docker compose

```
docker-compose up -d reverse-proxy
```

```
docker-compose up -d --scale oxen=4 --no-recreate
```

## Local File Structure

To inspect any of the key value dbs below

```
oxen kvdb-inspect <PATH_TO_DB>
```

```
.oxen/
  HEAD (file that contains name of current "ref")

    ex) heads/main

  refs/ (keeps track of branch heads, remote names and their current commits)
    key,value db of:

    # Local heads
    heads/main -> COMMIT_ID
    heads/feature/add_cats -> COMMIT_ID
    heads/experiment/add_dogs -> COMMIT_ID

    # What has been pushed in these branches
    remotes/experiment/add_dogs -> COMMIT_ID

  staged/ (created from `oxen add <file>` command)
    dirs/ (rocksdb of directory names)
      key: path/to/dir
      value: {  }
    files/ (going to mimic dir structure for fast access to subset)
      path/
        to/
          dir/ (rocks db of files specific to that dir, with relative paths)
            key: filename.jpg
            value: {"hash": "FILE_HASH", "tracking_type": "tabular|regular"} (we generate a file ID and hash for each file that is added)

  history/ (list of commits)
    COMMIT_HASH_1/
      dirs/ (rocks db of dirnames in commit, similar to staged above, but could include computed metadata)
        key: path/to/dir
        value: { "count": 1000, "other_meta_data": ? }
      files/
        path/
          to/
            dir/
              key: filename 
              value: {
                "hash" => "FILE_HASH", (use this to know if a file was different)
                ... other meta data
              }

    COMMIT_HASH_2/
    COMMIT_HASH_3/

  commits/ (created from `oxen commit -m "my message"` command. Also generates history/commit_hash)
    key,value of:

    COMMIT_HASH -> Commit

    A Commit is an object that contains, can use parent for ordering the commit logs
      - Message
      - Parent Commit ID
      - Author
      - Timestamp

  versions/ (copies of original files, versioned with commit ids)
    //
    //       ex) 59E029D4812AEBF0 -> 59/E029D4812AEBF0
    //           72617025710EBB55 -> 72/617025710EBB55
    //
    FILE_HASH_DIRS_1/
      COMMIT_ID_1 (dog_1.jpg)
    FILE_HASH_DIRS_2/
      COMMIT_ID_1 (dog_2.jpg)
```
