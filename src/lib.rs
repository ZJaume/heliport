use std::io::{self, BufRead};
use std::path::Path;
use std::env;

use pyo3::prelude::*;
use log::{info, debug};
use env_logger::Env;
use strum::IntoEnumIterator;
use target;

use crate::languagemodel::{Model, ModelType};
use crate::identifier::Identifier;
use crate::utils::Abort;


pub mod languagemodel;
pub mod identifier;
pub mod lang;
mod utils;

// Call python interpreter and obtain python path of our module
pub fn module_path() -> PyResult<String> {
    let mut path = String::new();
    Python::with_gil(|py| {
        // Instead of hardcoding the module name, obtain it from the crate name at compile time
        let module = PyModule::import_bound(py, env!("CARGO_PKG_NAME"))?;
        let paths: Vec<&str> = module
            .getattr("__path__")?
            .extract()?;
        // __path__ attribute returns a list of paths, return first
        path.push_str(paths[0]);
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
        let identifier = Identifier::load(&modulepath)?;

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


#[pyfunction]
pub fn cli_run() -> PyResult<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let modulepath = module_path().expect("Error loading python module path");
    let mut identifier = Identifier::load(&modulepath).or_abort(1);

    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        println!("{}", identifier.identify(&line.unwrap()).0);
    }
    Ok(())
}

#[pyfunction]
pub fn cli_download() -> PyResult<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let modulepath = module_path().expect("Error loading python module path");
    let url = format!(
        "https://github.com/ZJaume/{}/releases/download/v{}/models-{}-{}.tgz",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        target::os(),
        target::arch());

    utils::download_file_and_extract(&url, &modulepath).unwrap();
    info!("Finished");

    Ok(())
}

#[pyfunction]
pub fn cli_compile() -> PyResult<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let modulepath = module_path().expect("Error loading python module path");
    debug!("Module path found: {}", modulepath);
    let modelpath = Path::new("./LanguageModels");

    for model_type in ModelType::iter() {
        let type_repr = model_type.to_string();
        info!("Loading {type_repr} model");
        let model = Model::from_text(modelpath, model_type);
        let savepath = format!("{modulepath}/{type_repr}.bin");
        info!("Saving {type_repr} model");
        model.save(Path::new(&savepath))?;
    }
    info!("Saved models at '{}'", modulepath);
    info!("Finished");

    Ok(())
}

#[pymodule]
fn heliport(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(cli_run))?;
    m.add_wrapped(wrap_pyfunction!(cli_compile))?;
    m.add_wrapped(wrap_pyfunction!(cli_download))?;
    m.add_class::<PyIdentifier>()?;
    // m.add_class::<PyLang>()?;

    Ok(())
}
