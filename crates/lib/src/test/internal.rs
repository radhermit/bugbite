/// Test encoding a request and collecting the stream of resulting items.
#[macro_export]
macro_rules! stream {
    ($req:expr) => {
        let items = futures_util::TryStreamExt::try_collect::<Vec<_>>($req.stream())
            .await
            .unwrap();
        assert!(items.is_empty());
    };
}
pub(crate) use stream;
