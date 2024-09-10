/// Create a blocking, streaming iterator implementation for a given class and object.
#[macro_export]
macro_rules! stream_iterator {
    ($class:ty, $object:ty) => {
        #[pymethods]
        impl $class {
            fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
                slf
            }

            fn __next__(&mut self) -> Option<PyResult<$object>> {
                use ::futures_util::TryStreamExt;
                $crate::utils::tokio().block_on(async {
                    match self.0.try_next().await {
                        Err(e) => Some(Err(Error(e).into())),
                        Ok(v) => v.map(|x| Ok(x.into())),
                    }
                })
            }
        }
    };
}
pub(crate) use stream_iterator;
