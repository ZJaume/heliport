use anyhow::Result;
use heliport::cli::cli_run;
use std::env;

fn main() -> Result<()> {
    cli_run(env::args_os())
}
