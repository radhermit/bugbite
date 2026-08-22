use std::sync::LazyLock;

use tokio::sync::OnceCell;

use crate::service::bugzilla::Bugzilla;
use crate::traits::RequestSend;

pub const BASE: &str = "http://127.0.0.1:8080/";
pub const USER: &str = "bugbite@bugbite.test";
pub const PASSWORD: &str = "bugbite";

pub static SERVICE: LazyLock<Bugzilla> = LazyLock::new(|| {
    Bugzilla::builder(BASE)
        .unwrap()
        .user(USER)
        .password(PASSWORD)
        .build()
        .unwrap()
});

/// ID of an existing bug.
static EXISTING_ID: OnceCell<u64> = OnceCell::const_new();

/// Return an existing bug ID.
pub async fn get_existing_bug() -> crate::Result<u64> {
    EXISTING_ID
        .get_or_try_init(|| async {
            SERVICE
                .create()
                .summary("existing-bug")
                .component("TestComponent")
                .product("TestProduct")
                .description("description")
                .send()
                .await
        })
        .await
        .copied()
}
