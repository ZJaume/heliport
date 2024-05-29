use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::io::{Write, Read};
use std::path::Path;
use std::fs::{self, File};

use rkyv::ser::{Serializer, serializers::AllocSerializer};
use rkyv::{self, Archive, Deserialize, Serialize};
use log::{debug};

use wyhash2::WyHash;
type MyHasher = BuildHasherDefault<WyHash>;

use crate::lang::Lang;

pub mod identifier;
pub mod lang;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub enum ModelType {
    Word,
    Char
}


#[derive(Archive, Deserialize, Serialize, Debug, )]
#[archive_attr(derive(Debug))]
pub struct Model {
    pub dic: HashMap<String, HashMap<Lang, f32, MyHasher>, MyHasher>,
    model_type: ModelType,
}

impl Model {
    // The following values are the ones used in Jauhiainen et al. 2017.
    const MAX_USED : f64 = 0.0000005;

    pub fn contains(&self, key: &str) -> bool {
        self.dic.contains_key(key)
    }

    pub fn from_text(model_dir: &Path, model_type: ModelType) -> Self {
        let mut model = Model {
            dic: HashMap::default(),
            model_type: model_type
        };

        // Load each type of language model
        for lang in Lang::iter() {
            let lang_repr = lang.to_string().to_lowercase();
            if let ModelType::Char = model.model_type {
                model.read_model(&model_dir.join(format!("{lang_repr}.LowGramModel1")), &lang);
                model.read_model(&model_dir.join(format!("{lang_repr}.LowGramModel2")), &lang);
                model.read_model(&model_dir.join(format!("{lang_repr}.LowGramModel3")), &lang);
                model.read_model(&model_dir.join(format!("{lang_repr}.LowGramModel4")), &lang);
                model.read_model(&model_dir.join(format!("{lang_repr}.LowGramModel5")), &lang);
                model.read_model(&model_dir.join(format!("{lang_repr}.LowGramModel6")), &lang);
            } else {
                model.read_model(&model_dir.join(format!("{lang_repr}.LowWordModel")), &lang);
            }
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
        for (mut gram, amount) in temp_dict {
            prob = -(amount as f32 / langamount as f32).log10();
            if self.model_type == ModelType::Word {
                gram = format!(" {gram} ");
            }
            if self.dic.contains_key(&gram) {
                let inner_map = self.dic.get_mut(&gram).unwrap();
                inner_map.insert(langcode.clone(), prob);
            } else {
                let mut inner_map = HashMap::default();
                inner_map.insert(langcode.clone(), prob);
                self.dic.insert(gram, inner_map);
            }
        }
    }

    // Create a new struct reading from a binary file
    pub fn from_bin(p: &Path) -> Self {
        let mut file = File::open(p).expect(
            format!("Error cannot open file {p:?}").as_str());
        let mut content = Vec::new();
        let _ = file.read_to_end(&mut content).unwrap();

        let archived = unsafe { rkyv::archived_root::<Self>(&content[..]) };
        archived.deserialize(&mut rkyv::Infallible).unwrap()
    }

    // Save the struct in binary format
    // take ownership of the struct
    pub fn save(self, p: &Path) {
        // Create file
        let mut file = File::create(p).expect(
            format!("Error cannot write to file {p:?}").as_str());

        // Serialize in rkyv zero-copy serialization binary format
        let mut serializer = AllocSerializer::<1024>::default();
        serializer.serialize_value(&self).unwrap();
        let serialized = serializer.into_serializer().into_inner();
        // Write serialized bytes to the compressor
        file.write_all(&serialized).expect("Error writing serialized model");
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

        let charmodel = Model::from_text(&modelpath, ModelType::Char);
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
