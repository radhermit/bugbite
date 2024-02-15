// Return Option<Ordering> if the arguments or expression are not equal.
macro_rules! async_block {
    ($fn:expr) => {
        tokio::task::block_in_place(move || {
            tokio::runtime::Handle::current().block_on(async { $fn.await })
        })
    };
}
pub(crate) use async_block;
