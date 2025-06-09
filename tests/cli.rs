use assert_cmd::Command;
use predicates::prelude::*;

static LANGUAGEMODELS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/LanguageModels");

// provide the model dir because here we are testing the cargo bin
// and that one fails if model dir is not provided

#[test]
fn test_cli_missing_subcommand() {
    // Running with no subcommand should error and show help
    let mut cmd = Command::cargo_bin("heliport").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
}

#[test]
fn test_cli_invalid_subcommand() {
    // Unknown subcommand should fail
    let mut cmd = Command::cargo_bin("heliport").unwrap();
    cmd.arg("foobar")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn test_cli_identify_invalid_input_file() {
    // Should fail if input file does not exist
    let mut cmd = Command::cargo_bin("heliport").unwrap();
    cmd.args(["identify", "--model-dir", LANGUAGEMODELS, "nonexistent.txt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error opening input file"));
}

#[test]
fn test_cli_identify_invalid_output_file() {
    // Should fail if output file is not writable (e.g., directory)
    let mut cmd = Command::cargo_bin("heliport").unwrap();
    cmd.args([
        "identify",
        "--model-dir",
        LANGUAGEMODELS,
        "-",
        "path/to/nonexistent.txt",
    ])
    .write_stdin("Hello")
    .assert()
    .failure()
    .stderr(
        predicate::str::contains("Error opening input file")
            .or(predicate::str::contains("Is a directory")),
    );
}

#[test]
fn test_cli_identify_invalid_lang_code() {
    // Should fail if an invalid language code is given
    let mut cmd = Command::cargo_bin("heliport").unwrap();
    cmd.args([
        "identify",
        "--relevant-langs",
        "notalang",
        "--model-dir",
        LANGUAGEMODELS,
    ])
    .write_stdin("Hello")
    .assert()
    .failure()
    .stderr(predicate::str::contains("does not exist"));
}

#[test]
fn test_cli_identify_text_model() {
    // Should not fail when loading models from text
    let mut cmd = Command::cargo_bin("heliport").unwrap();
    cmd.args([
        "identify",
        "--model-dir",
        LANGUAGEMODELS,
        "--relevant-langs",
        "eng,spa",
    ])
    .write_stdin("Hello")
    .assert()
    .success();
}
