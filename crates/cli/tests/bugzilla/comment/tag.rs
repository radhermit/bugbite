use bugbite::test::assert_ordered_eq;
use bugbite::traits::RequestSend;
use camino_tempfile::tempdir;
use indexmap::IndexSet;

use crate::command::cmd;

use super::*;

#[tokio::test]
async fn single_bug() -> anyhow::Result<()> {
    let id = create_bug("comment-create").await?;

    // create comments
    for value in ["a", "b", "c"] {
        cmd!("bite bugzilla update -c {value} {id}")
            .assert()
            .success();
    }

    // add tags
    cmd!("bite bugzilla comment tag -t +foo,+bar {id}")
        .assert()
        .success();

    let comments = SERVICE.comment_get([id]).send().await?;
    let tags: IndexSet<_> = comments.iter().flatten().flat_map(|x| &x.tags).collect();
    // bugzilla lexically orders tags
    assert_ordered_eq!(tags, ["bar", "foo"]);

    // add and remove tags
    cmd!("bite bugzilla comment tag -t=-foo,+blah {id}")
        .assert()
        .success();

    let comments = SERVICE.comment_get([id]).send().await?;
    let tags: IndexSet<_> = comments.iter().flatten().flat_map(|x| &x.tags).collect();
    assert_ordered_eq!(tags, ["bar", "blah"]);

    // set all tags
    cmd!("bite bugzilla comment tag -t spam {id}")
        .assert()
        .success();

    let comments = SERVICE.comment_get([id]).send().await?;
    let tags: IndexSet<_> = comments.iter().flatten().flat_map(|x| &x.tags).collect();
    assert_ordered_eq!(tags, ["spam"]);

    // untag all comments
    cmd!("bite bugzilla comment tag {id} -u").assert().success();

    let comments = SERVICE.comment_get([id]).send().await?;
    assert!(comments.iter().flatten().all(|x| x.tags.is_empty()));

    Ok(())
}

#[tokio::test]
async fn from_template() -> anyhow::Result<()> {
    let id = create_bug("comment-create").await?;
    let dir = tempdir().unwrap();
    let path = dir.path().join("template");

    // add tags
    cmd!("bite bugzilla comment tag -t +foo,+bar {id}")
        .assert()
        .success();

    let comments = SERVICE.comment_get([id]).send().await?;
    let tags: IndexSet<_> = comments.iter().flatten().flat_map(|x| &x.tags).collect();
    // bugzilla lexically orders tags
    assert_ordered_eq!(tags, ["bar", "foo"]);

    // create template
    cmd!("bite bugzilla comment tag {id} -t spam --to {path} --dry-run")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // use template
    cmd!("bite bugzilla comment tag {id} --from {path}")
        .assert()
        .success();

    let comments = SERVICE.comment_get([id]).send().await?;
    assert!(comments.iter().flatten().all(|x| x.tags.is_empty()));

    Ok(())
}
