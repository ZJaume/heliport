use std::io::{self, BufRead, BufReader, Write, BufWriter};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Args;
use itertools::Itertools;
use log::{info, debug};
use pyo3::prelude::*;

use heliport_model::Lang;
use crate::identifier::Identifier;
use crate::utils::Abort;
use crate::python::module_path;

#[derive(Args, Clone, Debug)]
pub struct IdentifyCmd {
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

    #[arg(short = 'c', long, help="Ignore confidence thresholds. Predictions under the thresholds will not be labeled as 'und'")]
    ignore_confidence: bool,
    #[arg(short = 's', long, help="Print confidence score (higher is better) or raw score (higher is better) in case '-c' is provided")]
    print_scores: bool,

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
    pub fn cli(self) -> PyResult<()> {
        info!("Starting");
        let now = Instant::now();

        // If provided, parse the list of relevant languages
        let mut relevant_langs = None;
        if let Some(r) = &self.relevant_langs {
            relevant_langs = Some(parse_langs(&r).or_abort(1));
            info!("Using relevant langs: {:?}", relevant_langs.as_ref().unwrap());
        }
        debug!("{:?}", self);

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

        info!("Loading model");
        // Load identifier
        let mut identifier = Identifier::load(&model_dir, relevant_langs)
            .or_abort(1);
        if self.ignore_confidence {
            info!("Disabled confidence thresholds");
            identifier.disable_confidence();
        }

        // do not run on separated threads if multithreading is not requested
        if self.threads == 0 {
            info!("Running single-threaded");
            self.run_single(identifier, input_file, output_file).or_abort(1);
        } else {
            info!("Running with {} threads", self.threads);
            self.run_parallel(identifier, input_file, output_file).or_abort(1);
        }

        info!("Finished");
        info!("Elapsed time: {:.2?}", now.elapsed());
        Ok(())
    }


    // Run using the parallel identification method
    // read in batches
    fn run_parallel<'a, R, W>(self, identifier: Identifier, reader: R, mut writer: W) -> Result<()>
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
            for pred in identifier.par_identify(batch) {
                self.print_result(&mut writer, &pred).or_abort(1);
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
        for line_res in reader.lines() {
            let line = line_res?;
            let pred = identifier.identify(&line);
            self.print_result(&mut writer, &pred)?;
        }
        Ok(())
    }

    fn print_result<W>(&self, writer: &mut W, pred: &(Lang, Option<f32>)) -> io::Result<()>
        where W: Write,
    {
        if self.print_scores {
            writeln!(writer, "{}\t{:.4}", pred.0, pred.1.unwrap())
        } else {
            writeln!(writer, "{}", pred.0)
        }
    }
}


