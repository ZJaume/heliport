use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;

use heliport_model::languagemodel::binarize;

fn main() -> Result<()> {
    let mut model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    model_path.push("LanguageModels");

    let platlib_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/heliport.data/platlib/heliport",
    );
    fs::create_dir_all(&platlib_path)?;
    let save_path = PathBuf::from(&platlib_path);

    // Re-run build script if language models has changed
    println!(
        "cargo:rerun-if-changed={}",
        model_path.display()
    );
    println!("cargo:rerun-if-changed=build.rs");

    binarize(&save_path, &model_path)
}
