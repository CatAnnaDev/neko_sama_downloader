use crate::thread_pool::ThreadPool;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::Client;
use std::sync::mpsc;
use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    process::{exit, Command},
    time::Instant,
    {fs, io},
};
use std::time::Duration;
use thirtyfour::{common::capabilities::chrome::ChromeCapabilities, WebDriver};

mod html_parser;
mod thread_pool;
mod utils_check;
mod vlc_playlist_builder;
mod web;

const TMP_DL: &str = "./tmp";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "x86_64")]
static DRIVER_PATH: &str = "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/mac-x64/chromedriver-mac-x64.zip";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "arm")]
static DRIVER_PATH: &str = "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/mac-arm64/chromedriver-mac-arm64.zip";

#[cfg(target_os = "linux")]
static DRIVER_PATH: &str = "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/linux64/chromedriver-linux64.zip";

#[cfg(target_os = "windows")]
static DRIVER_PATH: &str = "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/win64/chromedriver-win64.zip";

static UBLOCK_PATH: &str =
    "https://github.com/PsykoDev/neko_sama_downloader/raw/main/utils/uBlock-Origin.crx";

// 120.0.6099.110

// https://googlechromelabs.github.io/chrome-for-testing/known-good-versions-with-downloads.json

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let ffmpeg_url = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-git-essentials.7z";

    let chrome_destination = PathBuf::from("./utils/chrome-win64.zip");
    let ffmpeg_destination = PathBuf::from("./utils/ffmpeg-git-essentials.7z");
    let ublock_destination = PathBuf::from("./utils/uBlock-Origin.crx");
    let extract_path = PathBuf::from("./utils");

    let mut chrome_check = false;
    let mut ffmpeg_check = false;
    let mut ublock_check = false;

    let url_test = env::args().collect::<Vec<_>>();

    if url_test.len() != 2 {
        println!("usage: ./anime_dl \"https://neko-sama.fr/anime/info/5821-sword-art-online_vf\"");
        //url_test.push("https://neko-sama.fr/anime/info/3458-hagane-no-renkinjutsushi-fullmetal-alchemist_vostfr".to_string());
        exit(0);
    }

    fs::create_dir_all(&extract_path)?;

    loop {
        for entry in fs::read_dir(&extract_path)? {
            if let Ok(x) = entry {
                if x.file_name().to_str().unwrap().ends_with(".exe") {
                    if x.file_name().to_str().unwrap().contains("chromedriver") {
                        chrome_check = true;
                    }
                    if x.file_name().to_str().unwrap().contains("ffmpeg") {
                        ffmpeg_check = true;
                    }
                }
                if x.file_name().to_str().unwrap().ends_with(".crx") {
                    if x.file_name().to_str().unwrap().contains("uBlock-Origin") {
                        ublock_check = true;
                    }
                }
            }
        }

        println!("chromedriver is present ? {chrome_check}");
        println!("ffmpeg is present ? {ffmpeg_check}");
        println!("uBlock Origin is present ? {ublock_check}");

        if !ublock_check {
            utils_check::download_and_extract_archive(
                UBLOCK_PATH,
                &ublock_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de uBlock Origin.");
        }

        if ffmpeg_check && chrome_check && ublock_check {
            start(&url_test).await?;
            break;
        } else if !ffmpeg_check && chrome_check {
            utils_check::download_and_extract_archive(
                ffmpeg_url,
                &ffmpeg_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de FFmpeg.");
        } else if !chrome_check && ffmpeg_check {
            utils_check::download_and_extract_archive(
                DRIVER_PATH,
                &chrome_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de Chrome.");
        } else {
            utils_check::download_and_extract_archive(
                DRIVER_PATH,
                &chrome_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de Chrome.");
            utils_check::download_and_extract_archive(
                ffmpeg_url,
                &ffmpeg_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de FFmpeg.");
        }
    }
    Ok(())
}

async fn start(url_test: &Vec<String>) -> Result<(), Box<dyn Error>> {

    let pool = ThreadPool::new(1); // 20 threads for 1Gb/s fiber

    let client = Client::builder().build()?;

    let _ = Command::new("./utils/chromedriver.exe").arg("--port=4444").spawn()?;

    let before = Instant::now();
    let mut save_path = String::new();
    let base_url = "https://neko-sama.fr";
    let mut prefs = ChromeCapabilities::new();
    prefs
        .add_extension(Path::new(r#"./utils/uBlock-Origin.crx"#))
        .expect("can't install ublock origin");

    let driver = WebDriver::new("http://localhost:4444", prefs).await?;

    driver.set_page_load_timeout(Duration::from_secs(20)).await?;
    driver.set_implicit_wait_timeout(Duration::from_secs(20)).await?;

    if let Some(last) = url_test.last() {
        driver.goto(last).await?;
    }

    println!("Scan Main Page");

    save_path.push_str(
        driver
            .title()
            .await?
            .replace(" - Neko Sama", "")
            .replace(":", "")
            .as_str(),
    );
    fs::create_dir_all(edit_for_windows_compatibility(&save_path.clone()))?;
    fs::create_dir_all(TMP_DL)?;
    let mut episode_url = html_parser::recursive_find_url(&driver, url_test.last().unwrap(), base_url).await?;

    println!("\ntotal found: {}", &episode_url.len());

    for (name, url) in &episode_url {
        if url.starts_with("http") {

            driver.goto(&url).await?;
            println!("Get m3u8 for: {}", name);

            if let Ok(script) = driver
                .execute(
                    r#"jwplayer().play(); let ret = jwplayer().getPlaylistItem(); return ret;"#,
                    vec![],
                )
                .await
            {
                if let Some(url) = script.json()["file"].as_str() {
                    html_parser::fetch_url(url, &name.trim().replace(":", ""), &client).await?;
                }

            }else {
                println!("Can't get .m3u8 {url}")
            }
        }else {
            println!("not http");
        }
    }

    println!("Start Processing");

    let progress_bar = ProgressBar::new(episode_url.len() as u64);

    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:60.cyan/blue} {pos}/{len} ({eta})")?
            .progress_chars("#>-"),
    );

    let (tx, rx) = mpsc::channel();

    let paths = fs::read_dir(TMP_DL)?;

    let _: Vec<_>  = paths
        .filter_map(|entry| {
            let tx = tx.clone();
            let entry = entry.ok();
            let file_path = entry?.path();
            if file_path.is_file() {
                let output_path = Path::new(TMP_DL).join(file_path.file_name()?);
                let name = format!(
                    "./{}/{}.mp4",
                    save_path.clone(),
                    edit_for_windows_compatibility(
                        &file_path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .replace(".m3u8", "")
                    )
                );
                Some(pool.execute(move || {
                    tx.send(web::download_build_video(
                        &output_path.to_str().unwrap(),
                        name,
                    ))
                    .expect("Error TX Send")
                }))
            } else {
                None
            }
        })
        .collect();

    drop(tx);
    progress_bar.inc(0); // force progress bar to appear before 1st finish

    for _ in rx.iter().take(episode_url.len()) {
        progress_bar.inc(1);
    }

    driver.close_window().await?;
    println!("Clean !");
    remove_dir_contents(TMP_DL)?;

    let seconds = before.elapsed().as_secs() % 60;
    let minutes = (before.elapsed().as_secs() / 60) % 60;
    let hours = (before.elapsed().as_secs() / 60) / 60;

    println!(
        "Done in: {:02}:{:02}:{:02}secs for {} episodes",
        hours,
        minutes,
        seconds,
        episode_url.len()
    );

    Ok(())
}

fn edit_for_windows_compatibility(name: &str) -> String {
    let regex = Regex::new(r#"[\\/?%*:|"<>]+"#).unwrap();
    regex.replace_all(name, "_").to_string()
}
fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        if let Ok(x) = entry {
            if x.file_name().to_str().unwrap().ends_with(".m3u8") {
                fs::remove_file(x.path())?;
            }
        }
    }
    Ok(())
}
