use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::env;

use clap::{Parser, Subcommand, Args};
use itertools::Itertools;
use pyo3::prelude::*;
use log::{info, debug};
use env_logger::Env;
use strum::IntoEnumIterator;
use target;

use crate::languagemodel::{ModelNgram, OrderNgram};
use crate::identifier::Identifier;
use crate::utils::Abort;
use crate::python::module_path;
use crate::download;

#[derive(Parser, Clone)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    #[command(about="Download heliport model from GitHub")]
    Download(DownloadCmd),
    #[command(about="Binarize heliport model")]
    Binarize(BinarizeCmd),
    #[command(about="Identify languages of input text", visible_alias="detect")]
    Identify(IdentifyCmd),
}

#[derive(Args, Clone)]
struct BinarizeCmd {
    #[arg(help="Input directory where ngram frequency files are located")]
    input_dir: Option<PathBuf>,
    #[arg(help="Output directory to place the binary files")]
    output_dir: Option<PathBuf>,
}

impl BinarizeCmd {
    fn cli(self) -> PyResult<()> {
        let model_path = self.input_dir.unwrap_or(PathBuf::from("./LanguageModels"));
        let save_path = self.output_dir.unwrap_or(module_path().unwrap());

        for model_type in OrderNgram::iter() {
            let type_repr = model_type.to_string();
            info!("Loading {type_repr} model");
            let model = ModelNgram::from_text(&model_path, model_type)
                .or_abort(1);
            let size = model.dic.len();
            info!("Created {size} entries");
            let filename = save_path.join(format!("{type_repr}.bin"));
            info!("Saving {type_repr} model");
            model.save(Path::new(&filename)).or_abort(1);
        }
        info!("Saved models at '{}'", save_path.display());
        info!("Finished");

        Ok(())
    }
}

#[derive(Args, Clone)]
struct DownloadCmd {
    #[arg(help="Path to download the model, defaults to the module path")]
    path: Option<PathBuf>,
}

impl DownloadCmd {
    fn cli(self) -> PyResult<()> {
        let download_path = self.path.unwrap_or(module_path().unwrap());

        let url = format!(
            "https://github.com/ZJaume/{}/releases/download/v{}/models-{}-{}.tgz",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            target::os(),
            target::arch());

        download::download_file_and_extract(&url, download_path.to_str().unwrap()).unwrap();
        info!("Finished");

        Ok(())
    }
}

#[derive(Args, Clone)]
struct IdentifyCmd {
    #[arg(help="Number of parallel threads to use", short='j', long, default_value_t=0)]
    threads: usize,
    #[arg(
        short,
        long,
        default_value_t=100000,
        help="Number of text segments to pre-load for parallel processing")]
    batch_size: usize,
}

impl IdentifyCmd {
    fn cli(self) -> PyResult<()> {
        let identifier = Identifier::load(&module_path().unwrap().to_str().unwrap())
            .or_abort(1);

        let stdin = io::stdin().lock();
        if self.threads == 0 {
            return self.run_single(stdin, identifier)
        } else {
            self.run_parallel(stdin, identifier)
        }
    }

    // Run using the parallel identification method
    // read in batches
    fn run_parallel<F>(self, reader: F, identifier: Identifier) -> PyResult<()>
        where F: BufRead
    {
        // Initialize global thread pool with the number of threads
        // provided by the user
        rayon::ThreadPoolBuilder::new()
            .num_threads(self.threads)
            .build_global()
            .or_abort(1);

        // Initialize the reader iterator in batches
        let batches = reader
            .lines()
            .chunks(self.batch_size);

        // Process each batch in parallel
        for batch_result in &batches {
            let batch: Vec<_> = batch_result
                .map(|line| {
                    line.or_abort(1)
                })
                .collect();
            for b in identifier.par_identify(batch) {
                println!("{}", b.0);
            }
        }
        Ok(())
    }

    // Run using the single-threaded indetification method
    fn run_single<F>(self, reader: F, mut identifier: Identifier) -> PyResult<()>
        where F: BufRead
    {
        // Process line by line
        for line in reader.lines() {
            println!("{}", identifier.identify(&line?).0);
        }
        Ok(())
    }
}

#[pyfunction]
pub fn cli_run() -> PyResult<()> {
    // parse the cli arguments, skip the first one that is the path to the Python entry point
    let os_args = std::env::args_os().skip(1);
    let args = Cli::parse_from(os_args);
    debug!("Module path found at: {}", module_path().expect("Could not found module path").display());
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    match args.command {
        Commands::Download(cmd) => { cmd.cli() },
        Commands::Binarize(cmd) => { cmd.cli() },
        Commands::Identify(cmd) => { cmd.cli() },
    }
}
