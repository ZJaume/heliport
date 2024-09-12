use std::path::PathBuf;
use std::env;

use pyo3::prelude::*;

#[cfg(feature = "cli")]
use crate::cli::cli_run;
use crate::utils::Abort;
use crate::identifier::Identifier;

// Call python interpreter and obtain python path of our module
pub fn module_path() -> PyResult<PathBuf> {
    let mut path = PathBuf::new();
    Python::with_gil(|py| {
        // Instead of hardcoding the module name, obtain it from the crate name at compile time
        let module = PyModule::import_bound(py, env!("CARGO_PKG_NAME"))?;
        let paths: Vec<&str> = module
            .getattr("__path__")?
            .extract()?;
        // __path__ attribute returns a list of paths, return first
        path.push(paths[0]);
        Ok(path)
    })
}

/// Bindings to Python
#[pyclass(name = "Identifier")]
pub struct PyIdentifier {
    inner: Identifier,
}

#[pymethods]
impl PyIdentifier {
    #[new]
    fn new() -> PyResult<Self> {
        let modulepath = module_path().expect("Error loading python module path");
        let identifier = Identifier::load(&modulepath.to_str().unwrap())
            .or_abort(1);

        Ok(Self {
            inner: identifier,
        })
    }

    fn identify(&mut self, text: &str) -> String {
        self.inner.identify(text).0.to_string()
    }
}

// #[pyclass(name = "Lang")]
// pub struct PyLang {
//     inner: Lang,
// }

#[pymodule]
fn heliport(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    #[cfg(feature = "cli")]
    m.add_wrapped(wrap_pyfunction!(cli_run))?;
    m.add_class::<PyIdentifier>()?;
    // m.add_class::<PyLang>()?;

    Ok(())
}
