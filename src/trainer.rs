use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};
use counter::Counter;
use lazy_static::lazy_static;
use log::{debug, info, warn};
use rayon::prelude::*;
use regex::Regex;
use shingles::AsShingles;
use strum::IntoEnumIterator;

use crate::utils::RE_NON_ALPHA;

use heliport_model::{Lang, OrderNgram};

lazy_static! {
    static ref RE_LANG_NAME: Regex =
        Regex::new(r"(\w{3,7}).train$").expect("Error compiling lang name from file regex");
}

// Count n-gram frequency of a given n-gram order in the text contained in the file
fn count_ngrams(input_file_path: &Path, order: OrderNgram) -> Result<Counter<String>> {
    let input_file = BufReader::new(File::open(input_file_path)?);
    let mut counts = Counter::new();

    // Read training file line by line and accumulate ngram counts
    for line_res in input_file.lines() {
        let line = line_res?;
        // Replace punctuation by spaces
        let replaced = RE_NON_ALPHA.replace_all(&line, " ");

        // iterate over words
        for word in replaced.split_whitespace() {
            // if current order is word, just count the words
            // otherwise put the space boundaries in the word
            // and generate all possible ngrams of the current order
            // and count them
            if order == OrderNgram::Word {
                if let Some(entry) = counts.get_mut(word) {
                    *entry += 1;
                } else {
                    counts.insert(String::from(word), 1);
                }
            } else {
                let wordspace = format!(" {word} ");
                // order can be cast to integer because the internal representations
                // have the same number (word is 0, unigram is 1 and so on)
                for gram in wordspace.as_shingles(order as usize) {
                    if let Some(entry) = counts.get_mut(gram) {
                        *entry += 1;
                    } else {
                        counts.insert(String::from(gram), 1);
                    }
                }
            }
        }
    }

    Ok(counts)
}

// Count n-gram frequency of all n-gram orders for a given lanuage
pub fn count_all_ngrams(input_file_path: &Path, output_dir: &Path, top_k: usize) -> Result<()> {
    // use the lang prefix in the input file as language code
    let string_file_name = input_file_path.to_string_lossy();
    let lang_string = RE_LANG_NAME
        .captures(&string_file_name)
        .context("Could not parse language name from input_file")?
        .get(1)
        .with_context(|| "Could not get first capture group from lang name regex")?
        .as_str();
    // Check that the language exists
    // warn if does not exist
    if Lang::from_str(&lang_string).is_err() {
        warn!("Language code '{lang_string}' does not exist. Please add it if you want the model to be used");
    }
    info!("Training '{lang_string}'");

    // Run training for each nggram order in parallel
    let ngram_orders: Vec<_> = OrderNgram::iter().collect();
    let results: Vec<Result<_>> = ngram_orders
        .into_par_iter()
        .map(|order| -> Result<()> {
            // Obtain nggram frequencies
            let counts = count_ngrams(input_file_path, order)?;
            // create output file with the language code and ngram order as name
            let output_file = File::create(output_dir.join(format!(
                "{}.{}.model",
                lang_string,
                order.to_string()
            )))
            .with_context(|| "Could not create file")?;
            let mut output_file = BufWriter::new(output_file);
            let total = counts.total::<usize>();
            debug!(
                "Total: {} top-10: {:?}",
                total,
                counts.k_most_common_ordered(10)
            );

            // Write the top-k most frequent n-grams with their frequencies and the total count
            writeln!(&mut output_file, "{}", total)?;
            for (ngram, count) in counts.k_most_common_ordered(top_k) {
                writeln!(&mut output_file, "{ngram}\t{count}")?;
            }
            Ok(())
        })
        .collect();

    for r in results {
        let _ = r?;
    }

    info!("Finished '{lang_string}'");
    Ok(())
}
