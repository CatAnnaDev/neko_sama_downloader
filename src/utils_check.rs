use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use reqwest::Client;
use crate::info;

pub async fn download_and_extract_archive(
    url: &str,
    destination: &PathBuf,
    extract_path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    info!("Download: {url}");
    let response = Client::new().get(url).send().await?;
    let archive_bytes = response.bytes().await?.to_vec();

    let mut archive_file = File::create(destination)?;
    archive_file.write_all(&archive_bytes)?;

    if url.ends_with(".zip") {
        extract_zip(archive_bytes, extract_path).await?;
    }

    if url.ends_with(".7z") {
        extract_7z(destination, extract_path).await?;
    }
    Ok(())
}

pub async fn extract_zip(zip_path: Vec<u8>, extract_path: &Path) -> Result<(), Box<dyn Error>> {
    zip_extract::extract(Cursor::new(zip_path), extract_path, true)?;
    Ok(())
}

pub async fn extract_7z(archive_path: &Path, extract_path: &Path) -> Result<(), Box<dyn Error>> {
    sevenz_rust::decompress_file(archive_path, extract_path).expect("complete");
    for x in fs::read_dir(extract_path)? {
        if let Ok(path) = x {
            if path.path().is_dir() {
                let x = format!("./{}/bin/ffmpeg.exe", path.path().to_str().unwrap());
                fs::rename(x, "./utils/ffmpeg.exe")?;
            }
        }
    }
    Ok(())
}
