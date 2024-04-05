use pyo3::prelude::*;
use pyo3::wrap_pymodule;

mod config;
mod error;

pub(crate) use self::error::Error;

/// Python library for bugbite.
#[pymodule]
#[pyo3(name = "bugbite")]
fn module(py: Python, m: &PyModule) -> PyResult<()> {
    // register submodules so `from pkgcraft.eapi import Eapi` works as expected
    m.add_wrapped(wrap_pymodule!(config::module))?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("pkgcraft.config", m.getattr("config")?)?;

    m.add("BugbiteError", py.get_type::<error::BugbiteError>())?;
    Ok(())
}
