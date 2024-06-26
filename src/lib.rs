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


// Call python interpreter and obtain python path of our module
pub fn pythonpath() -> PyResult<String> {
    let mut path = String::new();
    Python::with_gil(|py| {
        // Instead of hardcoding the module name, obtain it from the crate name at compile time
        let module = PyModule::import(py, env!("CARGO_PKG_NAME"))?;
        let paths: Vec<&str> = module
            .getattr("__path__")?
            .extract()?;
        // __path__ attribute returns a list of paths, return first
        path.push_str(paths[0]);
        Ok(path)
    })
}

pub fn load_models(modelpath: &str) -> (Model, Model) {
    let grampath = format!("{modelpath}/charmodel.bin");
    let char_handle = thread::spawn(move || {
        let path = Path::new(&grampath);
        Model::from_bin(path)
    });

    let wordpath = format!("{modelpath}/wordmodel.bin");
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
        let modulepath = pythonpath().expect("Error loading python module path");
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
    let modulepath = pythonpath().expect("Error loading python module path");
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
pub fn cli_convert() -> PyResult<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let modulepath = pythonpath().expect("Error loading python module path");
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
fn heli_otr(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(cli_run))?;
    m.add_wrapped(wrap_pyfunction!(cli_convert))?;
    m.add_class::<PyIdentifier>()?;
    // m.add_class::<PyLang>()?;

    Ok(())
}
