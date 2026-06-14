/// Test encoding a request and collecting the stream of resulting items.
#[macro_export]
macro_rules! stream_result {
    ($req:expr) => {
        futures_util::TryStreamExt::try_collect::<Vec<_>>($req.stream()).await
    };
}
pub(crate) use stream_result;

/// Verify a stream of results for a request is empty.
#[macro_export]
macro_rules! stream {
    ($req:expr) => {
        let items = $crate::test::stream_result!($req).unwrap();
        assert!(items.is_empty());
    };
}
pub(crate) use stream;
