use std::path::PathBuf;
use std::env;

use pyo3::prelude::*;

#[cfg(feature = "cli")]
use crate::cli::cli_run;
use crate::identifier::Identifier;

// Call python interpreter and obtain python path of our module
pub fn module_path() -> PyResult<PathBuf> {
    let mut path = PathBuf::new();
    Python::with_gil(|py| {
        // Instead of hardcoding the module name, obtain it from the crate name at compile time
        let module = PyModule::import(py, env!("CARGO_PKG_NAME"))?;
        let paths: Vec<String> = module
            .getattr("__path__")?
            .extract()?;
        // __path__ attribute returns a list of paths, return first
        path.push(&paths[0]);
        Ok(path)
    })
}

/// Bindings to Python
/// //TODO support returning both lang+score
/// //TODO support parallel identification
/// //TODO support loading relevant languages from text
#[pymethods]
impl Identifier {
    #[new]
    #[pyo3(signature = (ignore_confidence = false))]
    fn py_new(ignore_confidence: bool) -> PyResult<Self> {
        let modulepath = module_path().expect("Error loading python module path");
        let mut identifier = Identifier::load(&modulepath, None)?;
        if ignore_confidence {
            identifier.disable_confidence();
        }
        Ok(identifier)
    }

    #[pyo3(name = "identify")]
    fn py_identify(&mut self, text: &str) -> String {
        self.identify(text).0.to_string()
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
    m.add_class::<Identifier>()?;
    // m.add_class::<PyLang>()?;

    Ok(())
}
