use std::process::exit;

use log::{info, debug, error};
use tokio::io::AsyncWriteExt;
use tokio::runtime::Runtime;
use futures_util::StreamExt;
use reqwest;


// Run a tokio task that listens for ctrl+c
async fn run_cancel_handler() {
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(_) => {
                info!("Received Ctrl+C, terminating immediately.");
                exit(1);
            }
            Err(e) => error!("Error listening for SIGINT: {}", e),
        };
    });
}

// Download a file to a path
pub fn download_file(url: &str, filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = Runtime::new()?;
    runtime.block_on(async {
        info!("Downloading file from '{url}'");
        run_cancel_handler().await;
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
    })
}
