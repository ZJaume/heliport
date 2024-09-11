use std::process::{exit, Command};
use std::fs;

use log::{info, debug, error};
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;
use tokio::signal::unix;
use futures_util::StreamExt;
use tempfile::NamedTempFile;
use reqwest;

// Run a listener for cancel signals, if received terminate
// if a filename is provided, delete it
async fn run_cancel_handler(filename: Option<String>) {
    tokio::spawn(async move {
        let mut sigint = unix::signal(unix::SignalKind::interrupt()).unwrap();
        let mut sigterm = unix::signal(unix::SignalKind::terminate()).unwrap();
        let mut sigalrm = unix::signal(unix::SignalKind::alarm()).unwrap();
        let mut sighup = unix::signal(unix::SignalKind::hangup()).unwrap();
        loop {
            let kind;
            tokio::select! {
                _ = sigint.recv() => { kind = "SIGINT" },
                _ = sigterm.recv() => { kind = "SIGTERM" },
                _ = sigalrm.recv() => { kind = "SIGALRM" },
                _ = sighup.recv() => { kind = "SIGHUP" },
                else => break,
            }
            error!("Received {}, exiting", kind);
            if let Some(f) = filename {
                // panic if cannot be deleted?
                debug!("Cleaning temp: {}", f);
                fs::remove_file(&f).unwrap();
            }
            exit(1);
        }
    });
}

// Download a file to a path
async fn download_file_async(url: &str, filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Downloading file from '{url}'");
    // Create a download stream
    let response = reqwest::get(url).await?;
    let status = response.status();
    debug!("Response status: {}", status);
    if !status.is_success() {
        error!("Could not download file, HTTP status code: {status}");
        exit(1);
    }

    let mut response_stream = response.bytes_stream();
    let mut outfile = tokio::fs::File::create(filepath).await?;

    debug!("Writing file to '{filepath}'");
    // asyncronously write to the file every piece of bytes that come from the stream
    while let Some(bytes) = response_stream.next().await {
        outfile.write_all(&bytes?).await?;
    }

    Ok(())
}

// Download a .tgz file and extract it, async version
async fn download_file_and_extract_async(url: &str, extractpath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let binding = NamedTempFile::new()?.into_temp_path();
    let temp_path = binding
        .to_str()
        .ok_or("Error converting tempfile name to string")?;
    run_cancel_handler(Some(String::from(temp_path))).await;
    download_file_async(url, &temp_path).await?;

    let mut command = Command::new("/bin/tar");
    command.args(["xvfm", temp_path, "-C", extractpath, "--strip-components", "1"]);
    debug!("Running command {:?}", command.get_args());
    let comm_output = command.output()?;
    debug!("Command status: {:?}", comm_output.status);
    if !comm_output.status.success() {
        return Err(format!("Command failed during execution: {}",
                    std::str::from_utf8(&comm_output.stderr)?).into())
    }
    debug!("Command stderr: {}", std::str::from_utf8(&comm_output.stderr).unwrap());
    debug!("Command stdout: {}", std::str::from_utf8(&comm_output.stdout).unwrap());
    Ok(())
}

// Download a .tgz file and extract it, call async version and block on it
pub fn download_file_and_extract(url: &str, extractpath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    runtime.block_on(download_file_and_extract_async(url, extractpath))
}

// Trait that extracts the contained ok value or aborts if error
// sending the error message to the log
pub trait Abort<T> {
    fn or_abort(self, exit_code: i32) -> T;
}


impl<T, E: std::fmt::Display> Abort<T> for Result<T, E>
{
    fn or_abort(self, exit_code: i32) -> T {
        match self {
            Ok(v) => v,
            Err(e) => { error!("{e}"); exit(exit_code); },
        }
    }
}
