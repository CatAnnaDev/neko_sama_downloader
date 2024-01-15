use std::{env, fs};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;

use reqwest::Client;

use crate::{error, info, static_data, utils_data};
use crate::search::ProcessingUrl;

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

pub async fn confirm() -> Result<AllPath, Box<dyn Error>> {
    let path = check()?;

    let mut chrome_check = false;
    let mut ffmpeg_check = false;
    let mut ublock_check = false;

    fs::create_dir_all(&path.extract_path)?;

    for entry in fs::read_dir(&path.extract_path)? {
        if let Ok(x) = entry {
            if let Some(file_name) = x.file_name().to_str() {
                #[cfg(target_os = "windows")]
                if file_name.ends_with(".exe") {
                    if file_name.contains("chromedriver") {
                        chrome_check = true;
                    }
                    if file_name.contains("ffmpeg") {
                        ffmpeg_check = true;
                    }
                }

                #[cfg(any(target_os = "macos", target_os = "linux"))]
                if file_name.ends_with("") {
                    if file_name.contains("chromedriver") {
                        chrome_check = true;
                    }

                    ffmpeg_check = true;
                }

                if file_name.ends_with(".crx") {
                    if file_name.contains("uBlock-Origin") {
                        ublock_check = true;
                    }
                }
            }
        }
    }

    if !ublock_check {
        download(static_data::UBLOCK_PATH, &path.ublock_destination).await.expect("Erreur lors du téléchargement de uBlock Origin.");
    }

    match ffmpeg_check && chrome_check && ublock_check {
        true => {
            Ok(path)
        }
        false => {
            if !ffmpeg_check && chrome_check {
                error!(
                    "Please download then extract {} ffmpeg here:\n{}",
                    path.ffmpeg_path.display(),
                    static_data::FFMPEG_PATH
                );
                exit(0);
            } else if !chrome_check && ffmpeg_check {
                error!(
                    "Please download chrome wed driver then extract {} in utils folder here:\n{}",
                    path.chrome_path.display(),
                    static_data::DRIVER_PATH
                );
                exit(0);

            } else {
                error!(
                    "Please download chrome wed driver then extract {} in utils folder here:\n{}",
                    path.chrome_path.display(),
                    static_data::DRIVER_PATH
                );
                println!();
                error!(
                    "Please download then extract {} ffmpeg here:\n{}",
                    path.ffmpeg_path.display(),
                    static_data::FFMPEG_PATH
                );
                exit(0);

            }
        }
    }
}

pub async fn download(url: &str, destination: &PathBuf) -> Result<(), Box<dyn Error>> {
    info!("Download: {url}");
    let response = Client::new().get(url).send().await?;
    let archive_bytes = response.bytes().await?.to_vec();
    let mut archive_file = File::create(destination)?;
    archive_file.write_all(&archive_bytes)?;
    Ok(())
}

fn _pick_season_list(input: &str, processing_url: Vec<ProcessingUrl>, ) -> Result<Vec<ProcessingUrl>, Box<dyn Error>> {
    let numbers: Vec<usize> = input
        .split(|c: char| !c.is_digit(10))
        .filter_map(|s| s.parse().ok())
        .collect();
    Ok(numbers
        .iter()
        .filter_map(|&number| processing_url.get(number - 1)
            .map(|url| url.clone()))
        .collect())
}
