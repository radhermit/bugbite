use pyo3::prelude::*;

mod error;

/// Python library for bugbite.
#[pymodule]
#[pyo3(name = "bugbite")]
fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("BugbiteError", py.get_type_bound::<error::BugbiteError>())?;
    Ok(())
}
