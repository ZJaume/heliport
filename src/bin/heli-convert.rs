use std::path::Path;

use heli_otr::{Model, ModelType};


fn main() {
    let modelpath = Path::new("./LanguageModels");
    let wordmodel = Model::from_text(&modelpath, ModelType::Word);
    let path = Path::new("wordict.ser");
    wordmodel.save(path);

    let charmodel = Model::from_text(&modelpath, ModelType::Char);
    let path = Path::new("gramdict.ser");
    charmodel.save(path);
}
