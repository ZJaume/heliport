use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::fs::{self, File};

use bincode::{config, Decode, Encode};
use log::{debug};

#[derive(Encode, Decode, PartialEq, Debug)]
pub enum ModelType {
    Word,
    Char
}

#[derive(Encode, Decode, PartialEq, Debug)]
pub struct Model {
    pub dic: BTreeMap<String, BTreeMap<String, f32>>,
    language_list: Vec<String>,
    model_type: ModelType,
}

impl Model {
    // The following values are the ones used in Jauhiainen et al. 2017.
    const MAX_USED : f64 = 0.0000005;


    pub fn from_text(lang_list_path: &Path, model_dir: &Path,
                     model_type: ModelType) -> Self {
        let language_list: Vec<String> = fs::read_to_string(lang_list_path).
            unwrap().lines().map(|s| s.to_string()).collect();
        let mut model = Model {
            dic: BTreeMap::new(),
            language_list: Vec::new(),
            model_type: model_type
        };

        // Load each type of language model
        for lang in &language_list {
            if let ModelType::Char = model.model_type {
                model.read_model(&model_dir.join(format!("{lang}.LowGramModel1")), lang);
                model.read_model(&model_dir.join(format!("{lang}.LowGramModel2")), lang);
                model.read_model(&model_dir.join(format!("{lang}.LowGramModel3")), lang);
                model.read_model(&model_dir.join(format!("{lang}.LowGramModel4")), lang);
                model.read_model(&model_dir.join(format!("{lang}.LowGramModel5")), lang);
                model.read_model(&model_dir.join(format!("{lang}.LowGramModel6")), lang);
            } else {
                model.read_model(&model_dir.join(format!("{lang}.LowWordModel")), lang);
            }
        }

        // we give language_list here, otherwise cannot call mutable borrow 'model.read_model' above
        model.language_list = language_list;
        model
    }

    fn read_model(&mut self, p: &Path, langcode: &String) {
        // Read the language model file to a string all at once
        let modelfile = fs::read_to_string(p).expect(
            format!("Error reading file: {p:?}").as_str());

        let mut temp_dict = HashMap::new();
        let mut num_features = 0_usize;
        let mut amount: usize;
        let mut langamount = 0_usize;

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
        for (mut gram, amount) in temp_dict {
            prob = -(amount as f32 / langamount as f32).log10();
            if p.ends_with("LowWordModel") {
                gram = format!(" {gram} ");
            }
            if self.dic.contains_key(&gram) {
                let mut inner_map = self.dic.get_mut(&gram).unwrap();
                inner_map.insert(langcode.clone(), prob);
            } else {
                let inner_map = BTreeMap::from([(langcode.clone(), prob)]);
                self.dic.insert(gram, inner_map);
            }
        }
    }

    // Create a new struct reading from a binary file
    pub fn from_bin(p: &Path) -> Self {
        let config = config::standard();
        let mut file = File::open(p).expect(
            format!("Error cannot open file {p:?}").as_str());
        bincode::decode_from_std_read(&mut file, config)
            .expect("Error decoding from binary file")
    }

    // Save the truct in binary format, then destroy it
    pub fn save(self, p: &Path) {
        let config = config::standard();
        let mut file = File::create(p).expect(
            format!("Error cannot write to file {p:?}").as_str());
        let _ = bincode::encode_into_std_write(self, &mut file, config)
            .expect("Error encoding model to binary file");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;


    #[test]
    fn test_langs() {
        let langlistpath = Path::new("./languagelist");
        let modelpath = Path::new("./LanguageModels");
        let wordmodel = Model::from_text(&langlistpath, &modelpath, ModelType::Word);
        let path = Path::new("wordict.ser");
        wordmodel.save(path);

        let charmodel = Model::from_text(&langlistpath, &modelpath, ModelType::Char);
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

        let word_model = word_handle.join().unwrap();
        let char_model = char_handle.join().unwrap();

        let expected = BTreeMap::from([
            ("cat".to_string(), 3.4450269f32),
            ("epo".to_string(), 4.5279417f32),
            ("ext".to_string(), 2.5946937f32),
            ("gla".to_string(), 4.7058706f32),
            ("glg".to_string(), 2.3187783f32),
            ("grn".to_string(), 2.9653773f32),
            ("nhn".to_string(), 4.774119f32),
            ("que".to_string(), 3.8074818f32),
            ("spa".to_string(), 2.480955f32),
        ]);

        let probs = char_model.dic.get("ación").unwrap();
        assert_eq!(probs, &expected);
    }
}
