use std::io::{self, BufRead, BufReader, Write, BufWriter};
use std::fs::{copy, File};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::env;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, Args};
use itertools::Itertools;
use pyo3::prelude::*;
use log::{info, debug};
use env_logger::Env;
use strum::IntoEnumIterator;
use target;

use crate::languagemodel::{Model, ModelNgram, OrderNgram};
use crate::lang::Lang;
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
        info!("Copying confidence thresholds file");
        copy(
            model_path.join(Model::CONFIDENCE_FILE),
            save_path.join(Model::CONFIDENCE_FILE),
        ).or_abort(1);

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
    #[arg(help="Number of parallel threads to use.\n0 means no multi-threading\n1 means running the identification in a separated thread\n>1 run multithreading",
          short='j',
          long,
          default_value_t=0)]
    threads: usize,
    #[arg(
        short,
        long,
        default_value_t=100000,
        help="Number of text segments to pre-load for parallel processing")]
    batch_size: usize,

    #[arg(help="Input file, default: stdin", )]
    input_file: Option<PathBuf>,
    #[arg(help="Output file, default: stdout", )]
    output_file: Option<PathBuf>,

    #[arg(short, long, help="Model directory containing binarized model or plain text model. Default is Python module path or './LanguageModels' if relevant languages are requested")]
    model_dir: Option<PathBuf>,
    #[arg(long,
          short = 'l',
          value_delimiter=',',
          help="Load only relevant languages. Specify a comma-separated list of language codes. Needs plain text model directory")]
    relevant_langs: Option<Vec<String>>,
}

fn open_reader(p: &Path) -> Result<Box<dyn BufRead>> {
    let file = File::open(&p)
        .with_context(|| format!("Error opening input file {} for reading", p.display()))?;
    Ok(Box::new(BufReader::new(file)))
}

fn open_writer(p: &Path) -> Result<Box<dyn Write>> {
    let file = File::create(&p)
        .with_context(|| format!("Error opening input file {} for writing", p.display()))?;
    Ok(Box::new(BufWriter::new(file)))
}

// Parse a list of language code strings to Lang enum
fn parse_langs(langs_text: &Vec<String>) -> Result<Vec<Lang>> {
    let mut langs = Vec::new();
    for l in langs_text {
        langs.push(Lang::from_str(&l.to_lowercase())
                   .with_context(|| format!("Language code '{l}' does not exist"))?);
    }
    Ok(langs)
}

impl IdentifyCmd {
    fn cli(self) -> PyResult<()> {
        // If provided, parse the list of relevant languages
        let mut relevant_langs = None;
        if let Some(r) = &self.relevant_langs {
            relevant_langs = Some(parse_langs(&r).or_abort(1));
        }

        // Obtain model directory
        let model_dir;
        if let Some(m) = &self.model_dir {
            // Use provided model dir
            model_dir = m.clone();
        } else {
            // If user does not provide model dir and relevant languages
            // are requested, default to .LanguageModels in the repo
            // otherwise use python module path
            if relevant_langs.is_some() {
                model_dir = PathBuf::from("./LanguageModels");
            } else {
                model_dir = module_path().unwrap();
            }
        }

        let (input_file, output_file);
        if let Some(p) = &self.input_file {
            input_file = open_reader(&p).or_abort(1);
        } else {
            input_file = Box::new(io::stdin().lock());
        }
        if let Some(p) = &self.output_file {
            output_file = open_writer(&p).or_abort(1);
        } else {
            output_file = Box::new(io::stdout().lock());
        }

        // Load identifier
        let identifier = Identifier::load(&model_dir, relevant_langs)
            .or_abort(1);

        // do not run on separated threads if multithreading is not requested
        if self.threads == 0 {
            self.run_single(identifier, input_file, output_file).or_abort(1);
        } else {
            self.run_parallel(identifier, input_file, output_file).or_abort(1);
        }
        Ok(())
    }


    // Run using the parallel identification method
    // read in batches
    fn run_parallel<R, W>(self, identifier: Identifier, reader: R, mut writer: W) -> Result<()>
        where R: BufRead,
              W: Write,
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
                writeln!(writer, "{}", b.0)?;
            }
        }
        Ok(())
    }

    // Run using the single-threaded indetification method
    fn run_single<R, W>(self, mut identifier: Identifier, reader: R, mut writer: W) -> Result<()>
        where R: BufRead,
              W: Write,
    {
        // Process line by line
        for line in reader.lines() {
            writeln!(writer, "{}", identifier.identify(&line?).0)?;
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
