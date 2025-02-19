use std::path::{PathBuf};
use std::process::exit;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Args;
use log::{info, error};
use rayon::prelude::*;

use crate::utils::Abort;
use crate::trainer::count_all_ngrams;

#[derive(Args, Clone)]
pub struct CreateModelCmd {
    #[arg(help="Output directory to save the ngram frequency files")]
    output_dir: PathBuf,
    #[arg(help="Directory where input text files are located")]
    input_files: Vec<PathBuf>,
    #[arg(short = 'k', long, default_value_t = 10000, help="Truncate at top-k most frequent n-grams")]
    topk: usize,
}

impl CreateModelCmd {
    pub fn cli(self) -> Result<()> {
        info!("Starting");
        let now = Instant::now();

        if !self.output_dir.exists() {
            error!("Output directory '{}' does not exist, please create it", self.output_dir.display());
            exit(1);
        }

        info!("Saving top {} most frequent n-grams", self.topk);

        // Train each file/language in parallel
        // use panic_fuse to fail early if one of the jobs fail
        self.input_files
            .into_par_iter()
            .panic_fuse()
            .for_each(|lang_file| {
                count_all_ngrams(&lang_file, &self.output_dir, self.topk)
                    .with_context(|| format!("Error with file '{}'", lang_file.display()))
                    .or_abort(1);
            });

        info!("Finished");
        info!("Elapsed time: {:.2?}", now.elapsed());
        Ok(())
    }
}
