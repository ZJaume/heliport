mod identify;
#[cfg(feature = "download")]
mod download;
mod binarize;
mod create_models;

use anyhow::Result;
use clap::{Subcommand, Parser};
use log::{debug};
use env_logger::Env;
use std::ffi::OsString;

#[cfg(feature = "python")]
use crate::python::module_path;
#[cfg(feature = "download")]
use self::download::DownloadCmd;
use self::binarize::BinarizeCmd;
use self::identify::IdentifyCmd;
use self::create_models::CreateModelCmd;

#[derive(Parser, Clone)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, help="Do not print log messages")]
    quiet: bool,
}

#[derive(Subcommand, Clone)]
enum Commands {
    #[cfg(feature = "download")]
    #[command(about="Download heliport model from GitHub")]
    #[cfg(feature = "download")]
    Download(DownloadCmd),
    #[command(about="Binarize heliport model")]
    Binarize(BinarizeCmd),
    #[command(about="Identify languages of input text", visible_alias="detect")]
    Identify(IdentifyCmd),
    #[command(about="Create heliport models")]
    CreateModel(CreateModelCmd),
}


pub fn cli_run<I, T>(os_args: I) -> Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
{
    let args = Cli::parse_from(os_args);
    #[cfg(feature = "python")]
    debug!("Module path found at: {}", module_path().expect("Could not found module path").display());

    if !args.quiet {
        env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    } else {
        env_logger::Builder::from_env(Env::default().default_filter_or("error")).init();
    }

    match args.command {
        #[cfg(feature = "download")]
        Commands::Download(cmd) => { cmd.cli() },
        Commands::Binarize(cmd) => { cmd.cli() },
        Commands::Identify(cmd) => { cmd.cli() },
        Commands::CreateModel(cmd) => { cmd.cli() },
    }
}
