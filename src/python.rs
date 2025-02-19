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

#[cfg(feature = "cli")]
#[pyfunction]
#[pyo3(name = "cli_run")]
pub fn py_cli_run() -> PyResult<()> {
    // skip the first argument that is the path to the Python entry point
    let os_args = env::args_os().skip(1);
    cli_run(os_args)?;
    Ok(())
}

/// Bindings to Python
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

    /// Identify the language of a string
    #[pyo3(name = "identify")]
    fn py_identify(&mut self, text: &str) -> String {
        self.identify(text).0.to_string()
    }

    /// Identify the language of a string and return the language and score.
    /// This score is the confidence score (difference with the 2nd best)
    /// or the raw score if ignore_confidence is enabled.
    #[pyo3(name = "identify_with_score")]
    fn py_identify_with_score(&mut self, text: &str) -> (String, f32) {
        let pred = self.identify(text);
        (pred.0.to_string(), pred.1)
    }

    /// Identify the top-k most probable languages of a string and return the languages and scores.
    /// This score is the confidence score (difference with the 2nd best)
    /// or the raw score if ignore_confidence is enabled.
    #[pyo3(name = "identify_topk_with_score")]
    fn py_identify_topk_with_score(&mut self, text: &str, k: usize) -> Vec<(String, f32)> {
        let preds = self.identify_topk(text, k);
        let mut out = Vec::<_>::with_capacity(preds.len());
        for (pred, conf) in preds {
            out.push((pred.to_string(), conf));
        }
        out
    }

    /// Parallelized version of `identify` function for a list of strings.
    /// To change the number of threads set `RAYON_NUM_THREADS` environment variable.
    #[pyo3(name = "par_identify")]
    fn py_par_identify(&mut self, texts: Vec<String>) -> Vec<String> {
        let preds = self.par_identify(texts);
        let mut preds_out = Vec::with_capacity(preds.len());
        for pred in preds {
            preds_out.push(pred.0.to_string());
        }
        preds_out
    }

    /// Parallelized version of `identify_with_score` function for a list of strings.
    /// To change the number of threads set `RAYON_NUM_THREADS` environment variable.
    #[pyo3(name = "par_identify_with_score")]
    fn py_par_identify_with_score(&mut self, texts: Vec<String>) -> Vec<(String, f32)> {
        let preds = self.par_identify(texts);
        let mut preds_out = Vec::with_capacity(preds.len());
        for pred in preds {
            preds_out.push((pred.0.to_string(), pred.1));
        }
        preds_out
    }
}

// #[pyclass(name = "Lang")]
// pub struct PyLang {
//     inner: Lang,
// }

#[pymodule]
fn heliport(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    #[cfg(feature = "cli")]
    m.add_wrapped(wrap_pyfunction!(py_cli_run))?;
    m.add_class::<Identifier>()?;
    // m.add_class::<PyLang>()?;

    Ok(())
}
