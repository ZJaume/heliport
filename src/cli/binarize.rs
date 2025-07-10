use std::path::PathBuf;
use std::process::exit;

use anyhow::Result;
use clap::Args;
use log::{error, warn};

#[cfg(feature = "python")]
use crate::python::module_path;
use crate::utils::Abort;
use heliport_model::{binarize, OrderNgram};

#[derive(Args, Clone)]
pub struct BinarizeCmd {
    #[arg(help = "Input directory where ngram frequency files are located")]
    input_dir: Option<PathBuf>,
    #[arg(help = "Output directory to place the binary files")]
    output_dir: Option<PathBuf>,
    #[arg(
        short,
        long,
        help = "Force overwrite of output files if they already exist"
    )]
    force: bool,
    #[arg(help = "Do not be strict when loading confidence thresholds (do not fail if one language is missing)",
          short = 's',
          long)]
    not_strict: bool,
}

impl BinarizeCmd {
    pub fn cli(self) -> Result<()> {
        let model_path = self.input_dir.unwrap_or(PathBuf::from("LanguageModels"));

        #[cfg(feature = "python")]
        let save_path = self.output_dir.unwrap_or(module_path().unwrap());
        #[cfg(not(feature = "python"))]
        let save_path = self
            .output_dir
            .expect("Python feature is disabled. Input and output dirs must be provided");

        // Fail and warn the use if there is already a model
        if !self.force
            && save_path
                .join(format!("{}.bin", OrderNgram::Word.to_string()))
                .exists()
        {
            warn!(
                "Binarized models are now included in the PyPi package, \
            there is no need to binarize the model unless you are training a new one"
            );
            error!("Output model already exists, use '-f' to force overwrite");
            exit(1);
        }

        binarize(&save_path, &model_path, !self.not_strict).or_abort(1);
        Ok(())
    }
}
