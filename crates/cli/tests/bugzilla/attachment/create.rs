use std::fs;

use camino_tempfile::NamedUtf8TempFile;
use predicates::prelude::*;

use crate::command::cmd;

use super::get_existing_bug;

#[tokio::test]
async fn single_attachment_to_single_bug() -> anyhow::Result<()> {
    let id = get_existing_bug().await?;

    let file = NamedUtf8TempFile::new()?;
    fs::write(&file, "test")?;
    let path = file.path();

    cmd!("bite bugzilla attachment create {id} -v")
        .arg(path)
        .assert()
        .stdout(predicate::str::contains(format!(
            "{path}: attached to bug(s): {id}"
        )))
        .stderr("")
        .success();

    Ok(())
}
