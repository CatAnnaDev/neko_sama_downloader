use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use reqwest::Client;

use crate::info;

pub async fn download(url: &str, destination: &PathBuf) -> Result<(), Box<dyn Error>> {
    info!("Download: {url}");
    let response = Client::new().get(url).send().await?;
    let archive_bytes = response.bytes().await?.to_vec();

    let mut archive_file = File::create(destination)?;
    archive_file.write_all(&archive_bytes)?;
    Ok(())
}
