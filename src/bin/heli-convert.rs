use std::path::Path;

use heli_rust::{Model, ModelType};


fn main() {
    let langlistpath = Path::new("../heli-ots/languagelist");
    let modelpath = Path::new("../heli-ots/LanguageModels");
    let wordmodel = Model::from_text(&langlistpath, &modelpath, ModelType::Word);
    let path = Path::new("wordict.ser");
    wordmodel.save(path);

    let charmodel = Model::from_text(&langlistpath, &modelpath, ModelType::Char);
    let path = Path::new("gramdict.ser");
    charmodel.save(path);
}
