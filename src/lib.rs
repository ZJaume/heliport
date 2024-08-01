use std::io::{self, BufRead};
use std::sync::Arc;
use std::path::Path;
use std::thread;
use std::env;

use pyo3::prelude::*;
use log::{info, debug};
use env_logger::Env;

use crate::languagemodel::{Model, ModelType};
use crate::identifier::Identifier;


pub mod languagemodel;
pub mod identifier;
pub mod lang;
mod utils;

const WORDMODEL_FILE: &str = "wordmodel.bin";
const CHARMODEL_FILE: &str = "charmodel.bin";

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

pub fn load_models(modelpath: &str) -> (Model, Model) {
    let grampath = format!("{modelpath}/{CHARMODEL_FILE}");
    let char_handle = thread::spawn(move || {
        let path = Path::new(&grampath);
        Model::from_bin(path)
    });

    let wordpath = format!("{modelpath}/{WORDMODEL_FILE}");
    let word_handle = thread::spawn(move || {
        let path = Path::new(&wordpath);
        Model::from_bin(path)
    });
    let charmodel = char_handle.join().unwrap();
    let wordmodel = word_handle.join().unwrap();

    (charmodel, wordmodel)
}

/// Bindings to Python
#[pyclass(name = "Identifier")]
pub struct PyIdentifier {
    inner: Identifier,
}

#[pymethods]
impl PyIdentifier {
    #[new]
    fn new() -> Self {
        let modulepath = module_path().expect("Error loading python module path");
        let (charmodel, wordmodel) = load_models(&modulepath);
        let identifier = Identifier::new(
            Arc::new(charmodel),
            Arc::new(wordmodel),
        );

        Self {
            inner: identifier,
        }
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
    let (charmodel, wordmodel) = load_models(&modulepath);
    let mut identifier = Identifier::new(
            Arc::new(charmodel),
            Arc::new(wordmodel),
    );

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
        "https://github.com/ZJaume/heli-otr/releases/download/v{}",
        env!("CARGO_PKG_VERSION"));

    utils::download_file(
        &format!("{url}/{WORDMODEL_FILE}"),
        &format!("{modulepath}/{WORDMODEL_FILE}")
    ).unwrap();
    utils::download_file(
        &format!("{url}/{CHARMODEL_FILE}"),
        &format!("{modulepath}/{CHARMODEL_FILE}")
    ).unwrap();
    info!("Finished");

    Ok(())
}

#[pyfunction]
pub fn cli_convert() -> PyResult<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let modulepath = module_path().expect("Error loading python module path");
    debug!("Module path found: {}", modulepath);
    let modelpath = Path::new("./LanguageModels");

    info!("Loading wordmodel");
    let wordmodel = Model::from_text(modelpath, ModelType::Word);
    let savepath = format!("{modulepath}/wordmodel.bin");
    info!("Saving wordmodel");
    wordmodel.save(Path::new(&savepath));

    info!("Loading charmodel");
    let charmodel = Model::from_text(modelpath, ModelType::Char);
    let savepath = format!("{modulepath}/charmodel.bin");
    info!("Saving charmodel");
    charmodel.save(Path::new(&savepath));

    Ok(())
}

#[pymodule]
fn heli_otr(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(cli_run))?;
    m.add_wrapped(wrap_pyfunction!(cli_convert))?;
    m.add_wrapped(wrap_pyfunction!(cli_download))?;
    m.add_class::<PyIdentifier>()?;
    // m.add_class::<PyLang>()?;

    Ok(())
}
