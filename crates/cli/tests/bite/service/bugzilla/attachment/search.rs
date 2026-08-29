use camino_tempfile::tempdir;

use super::*;

#[test]
fn aliases() {
    for subcmd in ["s", "search"] {
        for opt in ["-h", "--help"] {
            cmd!("bite bugzilla attachment {subcmd} {opt}")
                .assert()
                .stdout(predicate::str::is_empty().not())
                .stderr("")
                .success();
        }
    }
}

#[tokio::test]
async fn nonexistent_bug() {
    let server = start_server().await;

    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    for opt in ["-i", "--id"] {
        cmd!("bite bugzilla attachment search")
            .args([opt, "1"])
            .assert()
            .stdout("")
            .stderr("")
            .success();
    }
}

#[tokio::test]
async fn template() {
    let server = start_server().await;

    // output template to stdout
    cmd!("bite bugzilla attachment search -c 1d -n")
        .args(["--to", "-"])
        .assert()
        .stdout(predicate::str::diff("created = \"1d\"").trim())
        .stderr("")
        .success();

    let dir = tempdir().unwrap();
    let path = dir.path().join("template");
    let path = path.as_str();

    // save template to a specific path
    cmd!("bite bugzilla attachment search -c 1d -n")
        .args(["--to", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();

    server
        .respond(200, TEST_DATA.join("search/nonexistent.json"))
        .await;

    // load template
    cmd!("bite bugzilla attachment search")
        .args(["--from", path])
        .assert()
        .stdout("")
        .stderr("")
        .success();
}
