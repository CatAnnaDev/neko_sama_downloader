use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use reqwest::Client;

use crate::{info, utils_data};

#[derive(Clone)]
pub struct AllPath {
    pub exe_path: PathBuf,
    pub ublock_destination: PathBuf,
    pub extract_path: PathBuf,
    pub tmp_dl: PathBuf,
    pub chrome_path: PathBuf,
    pub ffmpeg_path: PathBuf,
    pub u_block_path: PathBuf,
}

pub fn check() -> Result<AllPath, Box<dyn Error>> {
    let binding = env::current_exe()?;
    let exe_path = binding.parent().unwrap();

    let ublock_destination = exe_path.join(PathBuf::from("utils/uBlock-Origin.crx"));

    let extract_path = exe_path.join(PathBuf::from("utils/"));
    let tmp_dl = exe_path.join(PathBuf::from("tmp/"));

    utils_data::remove_dir_contents(&tmp_dl);

    // chrome driver
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let chrome_path = extract_path.join(PathBuf::from("chromedriver"));
    #[cfg(target_os = "windows")]
    let chrome_path = extract_path.join(PathBuf::from("chromedriver.exe"));

    // ffmpeg
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let ffmpeg_path = extract_path.join(PathBuf::from("ffmpeg"));
    #[cfg(target_os = "windows")]
    let ffmpeg_path = extract_path.join(PathBuf::from("ffmpeg.exe"));

    // ublock
    let u_block_path = extract_path.join(PathBuf::from("uBlock-Origin.crx"));

    Ok(AllPath {
        exe_path: binding,
        ublock_destination,
        extract_path,
        tmp_dl,
        chrome_path,
        ffmpeg_path,
        u_block_path,
    })
}

pub async fn download(url: &str, destination: &PathBuf) -> Result<(), Box<dyn Error>> {
    info!("Download: {url}");
    let response = Client::new().get(url).send().await?;
    let archive_bytes = response.bytes().await?.to_vec();

    let mut archive_file = File::create(destination)?;
    archive_file.write_all(&archive_bytes)?;
    Ok(())
}
