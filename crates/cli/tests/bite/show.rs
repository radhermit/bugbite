use std::fs;

use predicates::prelude::*;
use tempfile::tempdir;

use crate::command::cmd;

#[test]
fn services() {
    cmd("bite show services")
        .assert()
        .stdout(predicate::str::starts_with("Service: "))
        .stderr("")
        .success();
}

#[test]
fn connections() {
    cmd("bite show connections")
        .assert()
        .stdout(predicate::str::contains("gentoo"))
        .stderr("")
        .success();
}

#[test]
fn custom_config() {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_str().unwrap();
    let dir = dir.path().join(".config");
    let xdg_path = dir.to_str().unwrap();
    let dir = dir.join("bugbite/services");
    let dir_path = dir.to_str().unwrap();
    fs::create_dir_all(dir_path).unwrap();
    let file = dir.join("config");
    let file_path = file.to_str().unwrap();
    let config = indoc::indoc! {r#"
        type = "bugzilla"
        name = "bugzilla-test"
        base = "http://127.0.0.1:8080/"
    "#};
    fs::write(file_path, config).unwrap();

    // no custom config
    cmd("bite show connections")
        .assert()
        .stdout(predicate::str::contains("bugzilla-test").not())
        .stderr("")
        .success();

    // file target
    cmd("bite show connections")
        .env("BUGBITE_CONFIG", file_path)
        .assert()
        .stdout(predicate::str::contains("bugzilla-test"))
        .stderr("")
        .success();

    // dir target
    cmd("bite show connections")
        .env("BUGBITE_CONFIG", dir_path)
        .assert()
        .stdout(predicate::str::contains("bugzilla-test"))
        .stderr("")
        .success();

    if cfg!(target_os = "linux") {
        // $HOME dir
        cmd("bite show connections")
            .env_remove("BUGBITE_CONFIG")
            .env_remove("XDG_CONFIG_HOME")
            .env("HOME", home_path)
            .assert()
            .stdout(predicate::str::contains("bugzilla-test"))
            .stderr("")
            .success();

        // xdg config dir
        cmd("bite show connections")
            .env_remove("BUGBITE_CONFIG")
            .env("XDG_CONFIG_HOME", xdg_path)
            .assert()
            .stdout(predicate::str::contains("bugzilla-test"))
            .stderr("")
            .success();
    }
}
