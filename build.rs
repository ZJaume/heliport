use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use env_logger::Env;

use heliport_model::binarize;

fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

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
    println!(concat!(
        "cargo:rerun-if-changed=",
        env!("CARGO_MANIFEST_DIR"),
        "/build.rs")
    );
    // Re-run build script if heliort-model has been recompiled
    // I guess setting the path to lib.rs would be enough, as its build artifact would change 
    // if any of the source files have been recompiled
    println!(concat!(
        "cargo:rerun-if-changed=",
        env!("CARGO_MANIFEST_DIR"),
        "/heliport-model/src/lib.rs")
    );

    binarize(&save_path, &model_path)
}
