use std::process::{exit, Command};
use std::fs;

use log::{info, warn, debug, error};
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;
use tokio::signal::unix;
use futures_util::StreamExt;
use tempfile::NamedTempFile;
use anyhow::{bail, Context, Result};
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
                if fs::remove_file(&f).is_err(){
                    warn!("Could not remove temporary file: {f}");
                }
            }
            exit(1);
        }
    });
}

// Download a file to a path
async fn download_file_async(url: &str, filepath: &str) -> Result<()> {
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
async fn download_file_and_extract_async(url: &str, extractpath: &str) -> Result<()> {
    let binding = NamedTempFile::new()?.into_temp_path();
    let temp_path = binding
        .to_str()
        .context("Error converting tempfile name to string")?;
    run_cancel_handler(Some(String::from(temp_path))).await;
    download_file_async(url, &temp_path).await?;

    let mut command = Command::new("/bin/tar");
    command.args(["xvfm", temp_path, "-C", extractpath, "--strip-components", "1"]);
    debug!("Running command {:?}", command.get_args());
    let comm_output = command.output()?;
    debug!("Command status: {:?}", comm_output.status);
    // If the command fails, return an error, containing command stderr output
    if !comm_output.status.success() {
        let stderr_out = String::from_utf8_lossy(&comm_output.stderr);
        bail!("Command failed during execution: {stderr_out}");
    }
    debug!("Command stderr: {}", std::str::from_utf8(&comm_output.stderr)?);
    debug!("Command stdout: {}", std::str::from_utf8(&comm_output.stdout)?);
    Ok(())
}

// Download a .tgz file and extract it, call async version and block on it
pub fn download_file_and_extract(url: &str, extractpath: &str) -> Result<()> {
    let runtime = Runtime::new()?;
    runtime.block_on(download_file_and_extract_async(url, extractpath))
}


