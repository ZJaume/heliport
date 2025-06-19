use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::BuildHasherDefault;
use std::io::{self, Read, Write};
use std::ops::Index;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::thread;

use anyhow::{bail, Context, Result};
use bitcode;
use log::{debug, info, warn};
use rayon::prelude::*;
use strum::{Display, EnumCount, IntoEnumIterator};
use strum_macros::EnumIter;

use wyhash2::WyHash;
type MyHasher = BuildHasherDefault<WyHash>;

use crate::lang::{Lang, LangBitmap, LangScores};

#[derive(
    bitcode::Encode, bitcode::Decode, EnumIter, Display, EnumCount, Debug, PartialEq, Clone, Copy,
)]
#[strum(serialize_all = "lowercase")]
pub enum OrderNgram {
    Word,
    Unigram,
    Bigram,
    Trigram,
    Quadgram,
    Quingram,
    Hexagram,
}

#[derive(bitcode::Encode, bitcode::Decode, Debug, PartialEq)]
pub struct ModelNgram {
    pub dic: HashMap<String, Vec<(Lang, f32)>, MyHasher>,
    pub model_type: OrderNgram,
}

impl ModelNgram {
    // The following values are the ones used in Jauhiainen et al. 2017.
    pub const MAX_USED: f64 = 0.0000005;

    pub fn contains(&self, key: &str) -> bool {
        self.dic.contains_key(key)
    }

    pub fn from_text(
        model_dir: &Path,
        model_type: OrderNgram,
        langs: Option<Vec<Lang>>,
    ) -> Result<Self> {
        if let Some(l) = langs {
            Self::from_text_langs(model_dir, model_type, l)
        } else {
            Self::from_text_all(model_dir, model_type)
        }
    }

    /// Load the model from plain text for a subset of languages
    pub fn from_text_langs(
        model_dir: &Path,
        model_type: OrderNgram,
        langs: Vec<Lang>,
    ) -> Result<Self> {
        let mut model = ModelNgram {
            dic: HashMap::default(),
            model_type: model_type.clone(),
        };

        for lang in langs {
            let lang_repr = lang.to_string().to_lowercase();
            let type_repr = model_type.to_string();
            model.read_model(
                &model_dir.join(format!("{lang_repr}.{type_repr}.model")),
                &lang,
            )?;
        }

        Ok(model)
    }

    /// Load the model from plain text for all languages
    pub fn from_text_all(model_dir: &Path, model_type: OrderNgram) -> Result<Self> {
        let mut model = ModelNgram {
            dic: HashMap::default(),
            model_type: model_type.clone(),
        };
        let model_repr = model_type.to_string();

        // Open languagelist for this model
        let lang_list = fs::read_to_string(model_dir.join("languagelist"))
            .with_context(|| format!("Could not find '{}/languagelist'", model_dir.display()))?;
        let lang_list: HashSet<&str> = lang_list.split('\n').collect();

        // Load each type of language model
        for lang in Lang::iter() {
            if lang.is_special() {
                continue;
            }
            let lang_repr = lang.to_string().to_lowercase();
            // Models may not have all the language codes supported by the library
            if !lang_list.contains(&lang_repr[..]) {
                warn!("{model_repr}: Language '{lang_repr}' not found in languagelist, omitting");
                continue;
            }

            let type_repr = model_type.to_string();
            model.read_model(
                &model_dir.join(format!("{lang_repr}.{type_repr}.model")),
                &lang,
            )?;
        }

        // we give language_list here, otherwise cannot call mutable borrow 'model.read_model' above
        Ok(model)
    }

    /// Parse the ngram file, compute probabilities and insert into the model
    fn read_model(&mut self, p: &Path, langcode: &Lang) -> Result<()> {
        // Read the language model file to a string all at once
        let modelfile =
            fs::read_to_string(p).with_context(|| format!("Error reading file: {p:?}"))?;

        let mut temp_dict: HashMap<_, _, MyHasher> = HashMap::default();
        let mut num_features = 0_u64;
        let mut amount: u64;
        let mut langamount = 0_u64;

        debug!("Reading '{}'", p.display());

        // parse the language model file
        for (i, line) in modelfile.lines().enumerate() {
            // parse number of entries
            if i == 0 {
                num_features = line
                    .parse()
                    .with_context(|| format!("Error parsing line {i} in file {p:?}"))?;
                continue;
            }

            // parse an entry with token and frequency
            let parts: Vec<&str> = line.split("\t").collect();
            amount = parts[1]
                .parse()
                .with_context(|| format!("Error parsing line {i} in file {p:?}"))?;
            // insert into the map
            if (amount as f64 / num_features as f64) > Self::MAX_USED {
                temp_dict.insert(String::from(parts[0]), amount);
                langamount += amount;
            } else {
                debug!("Lang {langcode} break in |{}| {}", parts[0], parts[1]);
                break;
            }
        }

        // Insert into the Model
        // compute probability for each entry
        // if gram exists, insert the entry into that gram BTree, identified by lang and prob
        // if not, create a new BTree and insert it
        let mut prob;
        for (gram, amount) in temp_dict {
            prob = -(amount as f32 / langamount as f32).log10();
            if let Some(inner_vec) = self.dic.get_mut(&gram) {
                inner_vec.push((langcode.clone(), prob));
            } else {
                let mut inner_vec = Vec::new();
                inner_vec.push((langcode.clone(), prob));
                self.dic.insert(gram, inner_vec);
            }
        }

        Ok(())
    }

    // Create a new struct reading from a binary file
    pub fn from_bin(p: &Path) -> Result<Self> {
        let mut file = File::open(p)
            .with_context(|| format!("Could not open model file '{}'", p.display()))?;
        let mut content = Vec::new();
        let _ = file
            .read_to_end(&mut content)
            .with_context(|| format!("Error during reading file '{}'", p.display()))?;

        // should find a way to propagate possible bitcode errors?
        Ok(bitcode::decode(&content).with_context(|| "Could not deserialize model")?)
    }

    // Save the struct in binary format
    // take ownership of the struct
    pub fn save(self, p: &Path) -> Result<()> {
        // Create file
        let mut file = File::create(p)
            .with_context(|| format!("Could not open file for saving model: {}", p.display()))?;

        let serialized = bitcode::encode(&self);
        // Write serialized bytes to the compressor
        file.write_all(&serialized)
            .with_context(|| format!("Error during writing file '{}'", p.display()))
    }
}

pub struct Model {
    inner: [ModelNgram; OrderNgram::COUNT],
    pub confidence: LangScores,
}

impl Model {
    pub const CONFIDENCE_FILE: &'static str = "confidenceThresholds";

    // Load confidence thresholds
    pub fn load_confidence(conf_file_path: &Path, strict: bool) -> Result<LangScores> {
        let mut confidence = LangScores::new();
        let confidence_file = fs::read_to_string(conf_file_path)
            .with_context(|| "Could not open confidenceThreshold file")?;
        let mut loaded_langs = LangBitmap::new();

        for (i, line) in confidence_file.trim_end().split('\n').enumerate() {
            let parts: Vec<&str> = line.trim_end().split('\t').collect();
            // Check that the number of fields are correct and the language exists
            if parts.len() != 2 {
                bail!(
                    "Could not parse confidence files, expected fields 2, obtained {} in line {i}",
                    parts.len()
                );
            }
            let lang = Lang::from_str(parts[0]).with_context(|| {
                format!(
                    "Loading confidence file, lang '{}' does not exist",
                    parts[0]
                )
            })?;
            let prob = f32::from_str(parts[1]).with_context(|| {
                format!(
                    "Loading confidence file: could not parse float '{}'",
                    parts[1]
                )
            })?;

            loaded_langs.set(&lang, true);
            confidence.insert(lang, prob);
        }
        confidence.insert(Lang::und, 0.0);
        confidence.insert(Lang::zxx, 0.0);

        // Check all languages after collapsing have thresholds
        for lang in Lang::iter() {
            let lang_col = lang.collapse();
            if lang_col.is_special() {
                continue;
            }
            if strict && !loaded_langs.get(&lang_col) {
                bail!(
                    "Language '{}' confidence threshold not found '{}' file",
                    lang_col,
                    Self::CONFIDENCE_FILE
                );
            }
        }
        debug!("{:?}", loaded_langs);

        Ok(confidence)
    }

    pub fn load(
        modelpath: &Path,
        strict: bool,
        from_text: bool,
        langs: Option<Vec<Lang>>,
    ) -> Result<Self> {
        debug!("Loading model from '{}", modelpath.display());
        // Run a separated thread to load each model
        let mut handles: Vec<thread::JoinHandle<_>> = Vec::new();
        for model_type in OrderNgram::iter() {
            let type_repr = model_type.to_string();

            if from_text || langs.is_some() {
                // Load model from text
                let modelpath_copy = PathBuf::from(modelpath);
                let langs_copy = langs.clone();
                handles.push(thread::spawn(move || {
                    let model = ModelNgram::from_text(&modelpath_copy, model_type, langs_copy)?;
                    Ok(model)
                }));
            } else {
                // Load model binary
                let filename = modelpath.join(format!("{type_repr}.bin"));
                // If a model binary does not exist, fail early
                if !filename.exists() {
                    let message = format!("Model file '{}' could not be found", filename.display());
                    for h in handles {
                        //TODO figure out how to propagate this
                        let _ = h.join().unwrap()?;
                    }
                    return Err(io::Error::new(io::ErrorKind::NotFound, message).into());
                }
                handles.push(thread::spawn(move || {
                    let model = ModelNgram::from_bin(&filename)?;
                    // check model type is correct
                    assert!(model.model_type == model_type);
                    Ok::<ModelNgram, anyhow::Error>(model)
                }));
            }
        }
        let confidence_scores =
            Self::load_confidence(&modelpath.join(Self::CONFIDENCE_FILE), strict)?;

        Ok(Self {
            // remove first position because after removal, the vec is reindexed
            inner: [
                handles.remove(0).join().unwrap()?,
                handles.remove(0).join().unwrap()?,
                handles.remove(0).join().unwrap()?,
                handles.remove(0).join().unwrap()?,
                handles.remove(0).join().unwrap()?,
                handles.remove(0).join().unwrap()?,
                handles.remove(0).join().unwrap()?,
            ],
            confidence: confidence_scores,
        })
    }
}

// to avoid calling inner value
impl Index<usize> for Model {
    type Output = ModelNgram;

    fn index(&self, num: usize) -> &Self::Output {
        &self.inner[num]
    }
}

/// Binarize models and save in a path
pub fn binarize(save_path: &Path, model_path: &Path, strict: bool) -> Result<()> {
    let orders: Vec<_> = OrderNgram::iter().collect();

    let results: Vec<Result<_>> = orders
        .par_iter()
        .panic_fuse()
        .map(|model_type| -> Result<()> {
            let type_repr = model_type.to_string();
            info!("{type_repr}: loading text model");
            let model = ModelNgram::from_text(&model_path, model_type.clone(), None)?;
            let size = model.dic.len();
            let filename = save_path.join(format!("{type_repr}.bin"));
            info!("{type_repr}: saving binarized model with {size} entries");
            model.save(Path::new(&filename))
        })
        .collect();

    // If there is one error, propagate
    for r in results {
        let _ = r?;
    }

    info!("Copying confidence thresholds file");
    let conf_file_in = model_path.join(Model::CONFIDENCE_FILE);
    let conf_file_out = save_path.join(Model::CONFIDENCE_FILE);
    // Check conf file is ok by loading it
    let _ = Model::load_confidence(&conf_file_in, strict)?;
    fs::copy(conf_file_in, conf_file_out)?;

    info!("Saved models at '{}'", save_path.display());
    info!("Finished");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::NamedTempFile;

    #[test]
    fn test_langs() {
        let tempf = NamedTempFile::new().unwrap();
        let temppath = tempf.into_temp_path();
        let modelpath = Path::new("./LanguageModels");

        let model = ModelNgram::from_text(&modelpath, OrderNgram::Quingram, None).unwrap();
        // let path = Path::new("gramdict.ser");
        model.save(&temppath).unwrap();
        let model = ModelNgram::from_bin(&temppath).unwrap();
        temppath.close().unwrap();

        let mut expected = Vec::new();
        expected.push((Lang::ayr, 4.2863530f32));
        expected.push((Lang::cat, 3.3738296f32));
        expected.push((Lang::epo, 4.5279417f32));
        expected.push((Lang::ext, 2.5946038f32));
        expected.push((Lang::gla, 4.7052390f32));
        expected.push((Lang::glg, 2.3186955f32));
        expected.push((Lang::grn, 3.1885893f32));
        expected.push((Lang::kac, 5.5482570f32));
        expected.push((Lang::lmo, 5.2805230f32));
        expected.push((Lang::nhn, 5.0725970f32));
        expected.push((Lang::que, 3.8049161f32));
        expected.push((Lang::spa, 2.3922930f32));
        expected.push((Lang::vol, 5.1173210f32));

        let mut probs = model
            .dic
            .get("ación")
            .expect("Could not found the ngram in the model")
            .clone();
        // round to less decimals to be a lit permissive
        // as there are differences between java and rust
        let round_to = 10000.0;
        for i in expected.iter_mut() {
            i.1 = (i.1 * round_to).round() / round_to;
        }
        for i in probs.iter_mut() {
            i.1 = (i.1 * round_to).round() / round_to;
        }
        assert_eq!(&probs, &expected);
    }
}
