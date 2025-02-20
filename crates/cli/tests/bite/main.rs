use std::sync::LazyLock;
use std::{env, fs};

use bugbite::test::{build_path, TestServer};
use camino::Utf8PathBuf;
use indexmap::IndexSet;
use itertools::Itertools;
use predicates::prelude::*;
use tempfile::tempdir;

use command::cmd;

mod command;
mod service;
mod show;

pub(crate) static TEST_DATA_PATH: LazyLock<Utf8PathBuf> =
    LazyLock::new(|| build_path!(env!("CARGO_MANIFEST_DIR"), "testdata"));

async fn start_server() -> TestServer {
    let server = TestServer::new().await;
    unsafe { env::set_var("BUGBITE_CONNECTION", server.uri()) };
    server
}

async fn start_server_with_auth() -> TestServer {
    unsafe {
        env::set_var("BUGBITE_USER", "bugbite@bugbite.test");
        env::set_var("BUGBITE_PASS", "bugbite");
        env::set_var("BUGBITE_KEY", "bugbite");
    }

    start_server().await
}

/// Initialization for all test executables.
#[ctor::ctor]
fn initialize() {
    unsafe {
        // avoid spawning a real browser or editor by default
        env::set_var("EDITOR", "true");
        env::set_var("BROWSER", "true");

        // wipe bugbite-related environment variables
        for (key, _value) in env::vars() {
            if key.starts_with("BUGBITE_") {
                env::remove_var(key);
            }
        }
    }
}

#[test]
fn help() {
    for opt in ["-h", "--help"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::is_empty().not())
            .stderr("")
            .success();
    }
}

#[test]
fn version() {
    let version = env!("CARGO_PKG_VERSION");
    for opt in ["-V", "--version"] {
        cmd("bite")
            .arg(opt)
            .assert()
            .stdout(predicate::str::diff(format!("bite {version}")).trim())
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn doc() {
    let server = start_server_with_auth().await;
    let doc_dir = build_path!(env!("CARGO_MANIFEST_DIR"), "doc");
    let data_dir = TEST_DATA_PATH.join("bugbite");

    for entry in doc_dir.read_dir_utf8().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|x| x == "adoc").unwrap_or_default() {
            let name = entry.file_name();
            let stem = entry.path().file_stem().unwrap();
            let cmd_args = stem.split('-').collect::<Vec<_>>();
            let cmd_str = cmd_args.iter().join(" ");
            let cmd_dir = data_dir.join(cmd_args.iter().skip(1).join("/"));
            let data = cmd_dir.join("valid");

            // skip commands without default, valid data response
            if !data.exists() {
                continue;
            }

            server.reset().await;
            server.respond(200, data).await;

            // use a temporary directory for current dir to drop any outputted files
            let tmp_dir = tempdir().unwrap();
            env::set_current_dir(&tmp_dir).unwrap();

            let doc = fs::read_to_string(path).unwrap();
            // flag docs that support request templating
            let templates = doc.contains("template-options.adoc[]");

            for (lineno, line) in doc.lines().enumerate().filter(|(_, x)| x.starts_with(' ')) {
                for s in line.trim().split(" | ").filter(|x| x.starts_with(&cmd_str)) {
                    let args = shlex::split(s).unwrap();
                    let args: IndexSet<_> = args.iter().map(|x| x.as_str()).collect();

                    // skip commands reading from stdin
                    if args.contains("-") {
                        continue;
                    }

                    // skip commands lacking service subcommands
                    if args.len() < 3 {
                        continue;
                    }

                    // skip commands with custom service options
                    if args[2].starts_with('-') {
                        continue;
                    }

                    let cmd_str = args.iter().join(" ");
                    let cmd_result = cmd(cmd_str).assert();
                    let output = cmd_result.get_output();
                    let stderr = std::str::from_utf8(&output.stderr).unwrap().trim();
                    // ignore command errors requiring multiple responses
                    let invalid_response = stderr.starts_with("Error: invalid service response: ");
                    // ignore command errors trying to read files
                    let missing_file = stderr.ends_with(": No such file or directory (os error 2)");
                    if !output.status.success() && (!invalid_response && !missing_file) {
                        panic!(
                            "failed running: {s}\nfile: {name}, line {}\nstderr: {stderr}",
                            lineno + 1
                        );
                    }

                    // test templates for working commands
                    if output.status.success()
                        && templates
                        && !(args.contains("--to") || args.contains("--from"))
                    {
                        // save template
                        let mut save_args = args.clone();
                        save_args.extend(["-n", "--to", "template"]);
                        let cmd_str = save_args.iter().join(" ");
                        if cmd(cmd_str).assert().try_success().is_err() {
                            panic!(
                                "failed saving template: {s}\nfile: {name}, line {}",
                                lineno + 1
                            );
                        }

                        // load template
                        let mut load_args = args.clone();
                        load_args.extend(["--from", "template"]);
                        let cmd_str = load_args.iter().join(" ");
                        if cmd(cmd_str).assert().try_success().is_err() {
                            panic!(
                                "failed loading template: {s}\nfile: {name}, line {}",
                                lineno + 1
                            );
                        }
                    }
                }
            }
        }
    }
}
