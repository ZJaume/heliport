use std::path::Path;
use std::thread;

use heli_otr::Model;


fn main() {
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

    let probs = char_model.dic.get("ación ").unwrap();
    println!("{probs:?}");
}
