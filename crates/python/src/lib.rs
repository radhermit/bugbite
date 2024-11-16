use bugbite::error::python::BugbiteError;
use pyo3::prelude::*;
use pyo3::wrap_pymodule;

mod bugzilla;
mod config;
mod macros;
mod redmine;
mod service;
mod traits;
mod utils;

/// Python library for bugbite.
#[pymodule]
#[pyo3(name = "bugbite")]
fn ext(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // register submodules so `from bugbite.config import Config` works as expected
    let py = m.py();
    m.add_wrapped(wrap_pymodule!(bugzilla::ext))?;
    m.add_wrapped(wrap_pymodule!(config::ext))?;
    m.add_wrapped(wrap_pymodule!(redmine::ext))?;
    m.add_wrapped(wrap_pymodule!(service::ext))?;
    let sys_modules = py.import("sys")?.getattr("modules")?;
    sys_modules.set_item("bugbite.bugzilla", m.getattr("bugzilla")?)?;
    sys_modules.set_item("bugbite.config", m.getattr("config")?)?;
    sys_modules.set_item("bugbite.redmine", m.getattr("redmine")?)?;
    sys_modules.set_item("bugbite.service", m.getattr("service")?)?;

    m.add("BugbiteError", py.get_type::<BugbiteError>())?;
    Ok(())
}
