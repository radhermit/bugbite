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
        .stdout(predicate::str::is_empty().not())
        .stderr("")
        .success();
}

#[test]
fn connections_with_services() {
    // invalid
    cmd("bite show connections invalid")
        .assert()
        .stdout("")
        .stderr(predicate::str::is_empty().not())
        .failure()
        .code(2);

    // valid
    cmd("bite show connections bugzilla redmine")
        .assert()
        .stdout(predicate::str::is_empty().not())
        .stderr("")
        .success();
}

#[test]
fn custom_config() {
    let dir = tempdir().unwrap();
    let home_path = dir.path().to_str().unwrap();
    let dir = dir.path().join(".config");
    let xdg_path = dir.to_str().unwrap();
    let config_dir = dir.join("bugbite");
    let config_dir_path = config_dir.to_str().unwrap();
    let services_dir = config_dir.join("services");
    fs::create_dir_all(&services_dir).unwrap();
    let file = services_dir.join("config");
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

    // dir target
    cmd("bite show connections")
        .env("BUGBITE_CONFIG_DIR", config_dir_path)
        .assert()
        .stdout(predicate::str::contains("bugzilla-test"))
        .stderr("")
        .success();

    if cfg!(target_os = "linux") {
        // $HOME dir
        cmd("bite show connections")
            .env_remove("BUGBITE_CONFIG_DIR")
            .env_remove("XDG_CONFIG_HOME")
            .env("HOME", home_path)
            .assert()
            .stdout(predicate::str::contains("bugzilla-test"))
            .stderr("")
            .success();

        // xdg config dir
        cmd("bite show connections")
            .env_remove("BUGBITE_CONFIG_DIR")
            .env("XDG_CONFIG_HOME", xdg_path)
            .assert()
            .stdout(predicate::str::contains("bugzilla-test"))
            .stderr("")
            .success();
    }
}
