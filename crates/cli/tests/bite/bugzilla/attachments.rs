use std::{env, fs};

use bugbite::test::TestServer;
use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;
use crate::macros::build_path;

#[tokio::test]
async fn attachments() {
    let server = TestServer::new().await;
    env::set_var("BUGBITE_BASE", server.uri());
    env::set_var("BUGBITE_SERVICE", "bugzilla-rest-v1");
    let path = build_path!(env!("CARGO_MANIFEST_DIR"), "testdata");

    // listing single attachment via bug ID without data
    server
        .respond(200, "bugzilla/attachments/single-without-data.json")
        .await;
    let expected =
        fs::read_to_string(path.join("bugzilla/attachments/single-without-data.expected")).unwrap();

    for subcmd in ["a", "attachments"] {
        for opts in [vec!["-li"], vec!["-l", "-i"], vec!["--list", "--item-id"]] {
            cmd("bite")
                .arg(subcmd)
                .arg("123")
                .args(opts)
                .assert()
                .stdout(predicate::str::diff(expected.clone()))
                .stderr("")
                .success();
        }
    }

    server.reset().await;

    // viewing plain-text single attachment via bug ID
    server
        .respond(200, "bugzilla/attachments/single-plain-text.json")
        .await;
    let expected =
        fs::read_to_string(path.join("bugzilla/attachments/single-plain-text.expected")).unwrap();

    for subcmd in ["a", "attachments"] {
        for opts in [vec!["-Vi"], vec!["-V", "-i"], vec!["--view", "--item-id"]] {
            cmd("bite")
                .arg(subcmd)
                .arg("123")
                .args(opts)
                .assert()
                .stdout(predicate::str::diff(expected.clone()))
                .stderr("")
                .success();
        }
    }

    server.reset().await;

    // saving plain-text single attachment via bug ID
    server
        .respond(200, "bugzilla/attachments/single-plain-text.json")
        .await;

    let dir = tempdir().unwrap();
    let dir_path = dir.path().to_str().unwrap();
    for subcmd in ["a", "attachments"] {
        for opts in [
            vec!["-d", dir_path, "-i"],
            vec!["--dir", dir_path, "--item-id"],
        ] {
            cmd("bite")
                .arg(subcmd)
                .arg("123")
                .args(opts)
                .assert()
                .stdout(predicate::str::diff(format!(
                    "Saving attachment: {dir_path}/test.txt\n"
                )))
                .stderr("")
                .success();

            // verify file content
            let file = dir.path().join("test.txt");
            let data = fs::read_to_string(&file).unwrap();
            assert_eq!(&data, &expected);
            // remove file to avoid existence errors on loop
            fs::remove_file(&file).unwrap();
        }
    }
}
