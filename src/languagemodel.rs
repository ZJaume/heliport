use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use std::io::{self, Write, Read};
use std::fs::{self, File};
use std::path::Path;
use std::ops::Index;
use std::thread;

use strum::{IntoEnumIterator, Display, EnumCount};
use strum_macros::EnumIter;
use log::{debug, warn};
use bitcode;

use wyhash2::WyHash;
type MyHasher = BuildHasherDefault<WyHash>;

use crate::lang::Lang;

#[derive(bitcode::Encode, bitcode::Decode, EnumIter, Display, EnumCount,
         Debug, PartialEq, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum ModelType {
    Word,
    Unigram,
    Bigram,
    Trigram,
    Quadgram,
    Quingram,
    Hexagram,
}


#[derive(bitcode::Encode, bitcode::Decode, Debug, PartialEq)]
pub struct Model {
    pub dic: HashMap<String, Vec<(Lang, f32)>, MyHasher>,
    pub model_type: ModelType,
}

impl Model {
    // The following values are the ones used in Jauhiainen et al. 2017.
    pub const MAX_USED : f64 = 0.0000005;

    pub fn contains(&self, key: &str) -> bool {
        self.dic.contains_key(key)
    }

    pub fn from_text(model_dir: &Path, model_type: ModelType) -> Self {
        let mut model = Model {
            dic: HashMap::default(),
            model_type: model_type.clone()
        };

        // Open languagelist for this model
        let lang_list = fs::read_to_string(model_dir.join("languagelist"))
            .expect(format!("Could not find '{}/languagelist'", model_dir.display()).as_str());
        let lang_list: HashSet<&str> = lang_list.split('\n').collect();

        // Load each type of language model
        for lang in Lang::iter() {
            if lang == Lang::unk { continue; }
            let lang_repr = lang.to_string().to_lowercase();
            // Models may not have all the language codes supported by the library
            if !lang_list.contains(&lang_repr[..]) {
                warn!("Language '{lang_repr}' not found in languagelist, omitting");
                continue;
            }

            let type_repr = model_type.to_string();
            model.read_model(&model_dir.join(format!("{lang_repr}.{type_repr}.model")), &lang);
        }

        // we give language_list here, otherwise cannot call mutable borrow 'model.read_model' above
        model
    }

    fn read_model(&mut self, p: &Path, langcode: &Lang) {
        // Read the language model file to a string all at once
        let modelfile = fs::read_to_string(p).expect(
            format!("Error reading file: {p:?}").as_str());

        let mut temp_dict: HashMap<_, _, MyHasher> = HashMap::default();
        let mut num_features = 0_u64;
        let mut amount: u64;
        let mut langamount = 0_u64;

        debug!("Reading '{}'", p.display());

        // parse the language model file
        for (i, line) in modelfile.lines().enumerate() {
            // parse number of entries
            if i == 0 {
                num_features = line.parse().expect(
                    format!("Error parsing line {i} in file {p:?}").as_str());
                continue;
            }

            // parse an entry with token and frequency
            let parts: Vec<&str> = line.split("\t").collect();
            amount = parts[1].parse().expect(
                format!("Error parsing line {i} in file {p:?}").as_str());
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
            if self.dic.contains_key(&gram) {
                let inner_vec = self.dic.get_mut(&gram).unwrap();
                inner_vec.push((langcode.clone(), prob));
                inner_vec.shrink_to_fit();
            } else {
                let mut inner_vec = Vec::new();
                inner_vec.push((langcode.clone(), prob));
                self.dic.insert(gram, inner_vec);
            }
        }
    }

    // Create a new struct reading from a binary file
    pub fn from_bin(p: &str) -> io::Result<Self> {
        let mut file = File::open(p)?;
        let mut content = Vec::new();
        let _ = file.read_to_end(&mut content)?;

        // should find a way to propagate possible bitcode errors?
        Ok(bitcode::decode(&content).unwrap())
    }

    // Save the struct in binary format
    // take ownership of the struct
    pub fn save(self, p: &Path) -> io::Result<()> {
        // Create file
        let mut file = File::create(p)?;

        let serialized = bitcode::encode(&self);
        // Write serialized bytes to the compressor
        file.write_all(&serialized)
    }
}

pub struct Models {
    inner: [Model; ModelType::COUNT],
}

impl Models {
    pub fn load(modelpath: &str) -> io::Result<Self> {
        // Run a separated thread to load each model
        let mut handles: Vec<thread::JoinHandle<_>> = Vec::new();
        for model_type in ModelType::iter() {
            let type_repr = model_type.to_string();
            let filename = format!("{modelpath}/{type_repr}.bin");

            // If a model does not exist, fail early
            let path = Path::new(&filename);
            if !path.exists() {
                let message = format!("Model file '{}' could not be found", filename);
                for h in handles {
                    let _ = h.join().unwrap()?;
                }
                return Err(io::Error::new(io::ErrorKind::NotFound, message));
            }
            handles.push(thread::spawn(move || {
                let model = Model::from_bin(&filename)?;
                // check model type is correct
                assert!(model.model_type == model_type);
                Ok::<Model, io::Error>(model)
            }));
        }

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
            ]
        })
    }
}

// to avoid calling inner value
impl Index<usize> for Models {
    type Output = Model;

    fn index(&self, num: usize) -> &Self::Output {
        &self.inner[num]
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::collections::HashMap;


    #[test]
    fn test_langs() {
        let modelpath = Path::new("./LanguageModels");
        let wordmodel = Model::from_text(&modelpath, ModelType::Word);
        let path = Path::new("wordict.ser");
        wordmodel.save(path);

        let charmodel = Model::from_text(&modelpath, ModelType::Quadgram);
        let path = Path::new("gramdict.ser");
        charmodel.save(path);

        let char_handle = thread::spawn(move || {
            let path = Path::new("gramdict.ser");
            Model::from_bin(path)
        });

        let word_handle = thread::spawn(move || {
            let path = Path::new("wordict.ser");
            Model::from_bin(path)
        });

        // let word_model = word_handle.join().unwrap();
        let char_model = char_handle.join().unwrap();

        // failing because original HeLI is using a java float
        // instead of a double for accumulating frequencies
        let mut expected = HashMap::default();
        expected.insert(Lang::Cat, 3.4450269f32);
        expected.insert(Lang::Epo, 4.5279417f32);
        expected.insert(Lang::Ext, 2.5946937f32);
        expected.insert(Lang::Gla, 4.7058706f32);
        expected.insert(Lang::Glg, 2.3187783f32);
        expected.insert(Lang::Grn, 2.9653773f32);
        expected.insert(Lang::Nhn, 4.774119f32);
        expected.insert(Lang::Que, 3.8074818f32);
        expected.insert(Lang::Spa, 2.480955f32);

        let probs = char_model.dic.get("ación").unwrap();
        assert_eq!(probs, &expected);
    }
}
