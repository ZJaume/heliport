use std::path::{Path, PathBuf};
use std::fs;
use std::process::exit;

use clap::Args;
use log::{error, warn, info};
use pyo3::prelude::*;
use strum::IntoEnumIterator;

use heliport_model::languagemodel::{Model, ModelNgram, OrderNgram};
use crate::utils::Abort;
use crate::python::module_path;

#[derive(Args, Clone)]
pub struct BinarizeCmd {
    #[arg(help="Input directory where ngram frequency files are located")]
    input_dir: Option<PathBuf>,
    #[arg(help="Output directory to place the binary files")]
    output_dir: Option<PathBuf>,
    #[arg(short, long, help="Force overwrite of output files if they already exist")]
    force: bool,
}

impl BinarizeCmd {
    pub fn cli(self) -> PyResult<()> {
        let model_path = self.input_dir.unwrap_or(PathBuf::from("./LanguageModels"));
        let save_path = self.output_dir.unwrap_or(module_path().unwrap());

        // Fail and warn the use if there is already a model
        if !self.force &&
            save_path.join(
                format!("{}.bin", OrderNgram::Word.to_string())
                ).exists()
        {
            warn!("Binarized models are now included in the PyPi package, \
            there is no need to binarize the model unless you are training a new one"
                );
            error!("Output model already exists, use '-f' to force overwrite");
            exit(1);
        }

        for model_type in OrderNgram::iter() {
            let type_repr = model_type.to_string();
            info!("Loading {type_repr} model");
            let model = ModelNgram::from_text(&model_path, model_type, None)
                .or_abort(1);
            let size = model.dic.len();
            info!("Created {size} entries");
            let filename = save_path.join(format!("{type_repr}.bin"));
            info!("Saving {type_repr} model");
            model.save(Path::new(&filename)).or_abort(1);
        }
        info!("Copying confidence thresholds file");
        fs::copy(
            model_path.join(Model::CONFIDENCE_FILE),
            save_path.join(Model::CONFIDENCE_FILE),
        ).or_abort(1);

        info!("Saved models at '{}'", save_path.display());
        info!("Finished");

        Ok(())
    }
}


