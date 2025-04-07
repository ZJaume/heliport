use std::env;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use log::info;
use pyo3::prelude::*;
use target;

use crate::download;
use crate::python::module_path;

#[derive(Args, Clone)]
pub struct DownloadCmd {
    #[arg(help = "Path to download the model, defaults to the module path")]
    path: Option<PathBuf>,
}

impl DownloadCmd {
    pub fn cli(self) -> Result<()> {
        let download_path = self.path.unwrap_or(module_path().unwrap());

        let url = format!(
            "https://github.com/ZJaume/{}/releases/download/v{}/models-{}-{}.tgz",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            target::os(),
            target::arch()
        );

        download::download_file_and_extract(&url, download_path.to_str().unwrap()).unwrap();
        info!("Finished");

        Ok(())
    }
}
