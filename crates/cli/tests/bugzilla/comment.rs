use predicates::prelude::*;

use crate::command::cmd;

use super::create_bug;

#[tokio::test]
async fn get_single_bug() -> anyhow::Result<()> {
    let id = create_bug("comment-create").await?;

    // create comments
    for value in ["a", "b", "c"] {
        cmd!("bite bugzilla update -c {value} {id}")
            .assert()
            .success();
    }

    // get all comments
    cmd!("bite bugzilla comment get {id}")
        .assert()
        .stdout(predicate::str::is_empty().not())
        .stderr("")
        .success();

    // get comments with attachments
    cmd!("bite bugzilla comment get {id} -a")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    Ok(())
}

#[tokio::test]
async fn get_multiple_bugs() -> anyhow::Result<()> {
    let id1 = create_bug("comment-create-1").await?;
    let id2 = create_bug("comment-create-2").await?;

    // create comments
    for id in [id1, id2] {
        cmd!("bite bugzilla update -c 'bug #{id}' {id}")
            .assert()
            .success();
    }

    // get all comments
    cmd!("bite bugzilla comment get {id1} {id2}")
        .assert()
        .stdout(predicate::str::contains(format!("bug #{id1}")))
        .stdout(predicate::str::contains(format!("bug #{id2}")))
        .stderr("")
        .success();

    Ok(())
}

#[tokio::test]
async fn time_bounds() -> anyhow::Result<()> {
    let id = create_bug("comment-time-bounds").await?;

    // create comment
    cmd!("bite bugzilla update -c comment {id}")
        .assert()
        .success();

    // created after
    cmd!("bite bugzilla comment get {id} -c 2000-01-01")
        .assert()
        .stdout(predicate::str::is_empty().not())
        .stderr("")
        .success();
    cmd!("bite bugzilla comment get {id} --created 9999-01-01")
        .assert()
        .stdout(predicate::str::is_empty())
        .stderr("")
        .success();

    Ok(())
}
