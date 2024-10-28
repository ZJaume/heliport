use std::env;
use std::fs::copy;
use std::path::PathBuf;

use anyhow::Result;
use log::{info};
use strum::IntoEnumIterator;

use heliport_model::languagemodel::{Model, ModelNgram, OrderNgram};

fn main() -> Result<(), std::io::Error> {
    let mut model_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    model_path.push("LanguageModels");
    let save_path = PathBuf::from(&env::var("OUT_DIR").unwrap());

    // Re-run build script if language models has changed
    println!(
        "cargo:rerun-if-changed={}",
        model_path.display()
    );

    //TODO parallelize
    for model_type in OrderNgram::iter() {
        let type_repr = model_type.to_string();
        info!("Loading {type_repr} model");
        let model = ModelNgram::from_text(&model_path, model_type, None)
            .unwrap();
        let size = model.dic.len();
        info!("Created {size} entries");
        let filename = save_path.join(format!("{type_repr}.bin"));
        info!("Saving {type_repr} model");
        model.save(&filename).unwrap();
    }
    info!("Copying confidence thresholds file");
    copy(
        model_path.join(Model::CONFIDENCE_FILE),
        save_path.join(Model::CONFIDENCE_FILE),
    ).unwrap();

    info!("Finished");

    Ok(())
}
