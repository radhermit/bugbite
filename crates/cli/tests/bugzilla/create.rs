use camino_tempfile::tempdir;

use crate::command::cmd;

use super::get_existing_bug;

#[tokio::test]
async fn from_bug() -> anyhow::Result<()> {
    let id = get_existing_bug().await?;

    cmd!("bite bugzilla create --from-bug {id} -S summary -D description")
        .assert()
        .success();

    Ok(())
}

#[tokio::test]
async fn from_template() -> anyhow::Result<()> {
    let id = get_existing_bug().await?;
    let dir = tempdir()?;
    let path = dir.path().join("template");

    // create template from bug
    cmd!("bite bugzilla create --from-bug {id} --to {path} --dry-run")
        .assert()
        .stdout("")
        .stderr("")
        .success();

    // use template to create bug
    cmd!("bite bugzilla create --from {path} -S summary -D description")
        .assert()
        .success();

    Ok(())
}
