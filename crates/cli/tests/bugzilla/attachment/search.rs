use std::time::Duration;
use std::{fs, thread};

use bugbite::service::bugzilla::attachment::create::Attachment;
use bugbite::traits::RequestSend;
use camino_tempfile::NamedUtf8TempFile;
use chrono::{SecondsFormat, prelude::*};
use itertools::Itertools;
use predicates::prelude::*;
use uuid::Uuid;

use crate::command::cmd;

use super::{SERVICE, create_bug};

#[tokio::test]
async fn filters() -> anyhow::Result<()> {
    let bug_id = create_bug("attachment-search").await?;

    // create files for attachments
    let file1 = NamedUtf8TempFile::new()?;
    let file2 = NamedUtf8TempFile::new()?;
    fs::write(&file1, "test1")?;
    fs::write(&file2, "test2")?;
    let path1 = file1.path();
    let path2 = file2.path();

    let attachment1 = Attachment::new(path1).name(Some("test1"));
    let attachment2 = Attachment::new(path2).name(Some("test2"));

    let (id1, id2) = SERVICE
        .attachment_create([bug_id])
        .attachments([attachment1, attachment2])
        .send()
        .await?
        .into_iter()
        .flatten()
        .collect_tuple()
        .unwrap();

    // search by bug ID
    cmd!("bite bugzilla attachment search --id {bug_id}")
        .assert()
        .stdout(predicate::str::contains(format!("{id1}: ")))
        .stdout(predicate::str::contains(format!("{id2}: ")))
        .stderr("")
        .success();

    // search by filename
    for opt in ["-f", "--filename"] {
        cmd!("bite bugzilla attachment search {opt} test1")
            .assert()
            .stdout(predicate::str::contains(format!("{id1}: ")))
            .stdout(predicate::str::contains(format!("{id2}: ")).not())
            .stderr("")
            .success();
    }

    // delay at least a second so updated attachment uses a different timestamp
    thread::sleep(Duration::from_secs(1));
    // get current time rounded to seconds
    let updated = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    // update attachment
    let uuid = Uuid::new_v4();
    let (updated_id,) = SERVICE
        .attachment_update([id1])
        .comment(uuid)
        .description(uuid)
        .mime_type("text/plain")
        .name(uuid)
        .patch(true)
        .private(true)
        .send()
        .await?
        .into_iter()
        .collect_tuple()
        .unwrap();
    assert_eq!(updated_id, id1);

    // search by description
    for opt in ["-d", "--description"] {
        cmd!("bite bugzilla attachment search {opt} {uuid}")
            .assert()
            .stdout(predicate::str::contains(format!("{id1}: ")))
            .stdout(predicate::str::contains(format!("{id2}: ")).not())
            .stderr("")
            .success();
    }

    // search by filename
    for opt in ["-f", "--filename"] {
        cmd!("bite bugzilla attachment search {opt} {uuid}")
            .assert()
            .stdout(predicate::str::contains(format!("{id1}: ")))
            .stdout(predicate::str::contains(format!("{id2}: ")).not())
            .stderr("")
            .success();
    }

    // search by update time
    for opt in ["-u", "--updated"] {
        cmd!("bite bugzilla attachment search {opt} {updated}..")
            .assert()
            .stdout(predicate::str::contains(format!("{id1}: ")))
            .stdout(predicate::str::contains(format!("{id2}: ")).not())
            .stderr("")
            .success();
    }

    Ok(())
}
