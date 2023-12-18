use std::{env, error::Error, path::{Path, PathBuf}, process::{exit, Command}, time::Instant, {fs, io},sync::mpsc,time::Duration};
use thirtyfour::{common::capabilities::chrome::ChromeCapabilities, WebDriver};
use indicatif::{ProgressBar, ProgressStyle};
use crate::thread_pool::ThreadPool;
use reqwest::Client;
use regex::Regex;

mod vlc_playlist_builder;
mod html_parser;
mod thread_pool;
mod utils_check;
mod web;
mod log_color;

#[cfg(target_os = "macos")]
#[cfg(target_arch = "x86_64")]
static DRIVER_PATH: &str =
    "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/mac-x64/chromedriver-mac-x64.zip";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "arm")]
static DRIVER_PATH: &str =
    "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/mac-arm64/chromedriver-mac-arm64.zip";

#[cfg(target_os = "linux")]
static DRIVER_PATH: &str =
    "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/linux64/chromedriver-linux64.zip";

#[cfg(target_os = "windows")]
static DRIVER_PATH: &str =
    "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/win64/chromedriver-win64.zip";

static UBLOCK_PATH: &str =
    "https://github.com/PsykoDev/neko_sama_downloader/raw/main/utils/uBlock-Origin.crx";

// 120.0.6099.110

// https://googlechromelabs.github.io/chrome-for-testing/known-good-versions-with-downloads.json

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {

    let binding = env::current_exe()?;
    let exe_path = binding.parent().unwrap();

    let ffmpeg_url = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-git-essentials.7z";

    let chrome_destination = exe_path.join(PathBuf::from("utils/chrome-win64.zip"));
    let ffmpeg_destination = exe_path.join(PathBuf::from("utils/ffmpeg-git-essentials.7z"));
    let ublock_destination = exe_path.join(PathBuf::from("utils/uBlock-Origin.crx"));
    let extract_path =       exe_path.join(PathBuf::from("utils/"));
    let tmp_dl =             exe_path.join(PathBuf::from("tmp/"));
    let chrome_path =        extract_path.join(PathBuf::from("chromedriver.exe"));
    let u_block_path =       extract_path.join(PathBuf::from("uBlock-Origin.crx"));
    let ffmpeg_path =        extract_path.join(PathBuf::from("ffmpeg.exe"));

    let mut chrome_check = false;
    let mut ffmpeg_check = false;
    let mut ublock_check = false;

    let args: Vec<_> = env::args().collect::<_>();
    let url_test = args.iter().nth(1).expect("usage: ./anime_dl \"https://neko-sama.fr/anime/info/5821-sword-art-online_vf\"");
    let thread = args.iter().nth(2).unwrap_or(&String::from("1")).parse::<usize>().unwrap();


    if url_test.is_empty() {
        warn!("usage: ./anime_dl \"https://neko-sama.fr/anime/info/5821-sword-art-online_vf\"");
        exit(0);
    }else if !url_test.contains("https://neko-sama.fr/") {
        warn!("ONLY https://neko-sama.fr/ work actually")
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

        info!("chromedriver is present\t ? {chrome_check}");
        info!("ffmpeg is present\t ? {ffmpeg_check}");
        info!("uBlock Origin is present ? {ublock_check}");

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
            start(&url_test, exe_path, &tmp_dl, &chrome_path, &u_block_path, &ffmpeg_path, thread).await?;
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

async fn start(url_test: &String, exe_path: &Path, tmp_dl: &PathBuf, chrome: &PathBuf, ublock: &PathBuf, ffmpeg: &PathBuf, thread: usize) -> Result<(), Box<dyn Error>> {

    let pool = ThreadPool::new(thread);

    let client = Client::builder().build()?;

    let _ = Command::new(chrome).arg("--port=4444").spawn()?;

    let before = Instant::now();

    let mut save_path = String::new();

    let base_url = "https://neko-sama.fr";

    let mut prefs = ChromeCapabilities::new();
    prefs
        .add_extension(ublock)
        .expect("can't install ublock origin");

    let driver = WebDriver::new("http://localhost:4444", prefs).await?;
    driver.minimize_window().await?;

    driver.set_page_load_timeout(Duration::from_secs(20)).await?;

    driver.goto(url_test).await?;

    info!("Scan Main Page");

    let episode_url = scan_main_page(&mut save_path, &driver, url_test, base_url, tmp_dl).await?;

    info!("total found: {}", &episode_url.len());

    let _ = get_real_video_link(&episode_url, &driver, &client, &tmp_dl).await?;

    info!("Start Processing with {} threads", thread);

    let progress_bar = ProgressBar::new(episode_url.len() as u64);
    progress_bar.enable_steady_tick(Duration::from_secs(1));

    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:60.cyan/blue} {pos}/{len} ({eta})")?
            .progress_chars("#>-"),
    );

    let (tx, rx) = mpsc::channel();

    let _: Vec<_>  = fs::read_dir(tmp_dl)?
        .filter_map(|entry| {

            let tx = tx.clone();
            let ffmpeg = ffmpeg.clone();
            let entry = entry.ok();
            let file_path = entry?.path();

            if file_path.is_file() {
                let output_path = Path::new(tmp_dl).join(file_path.file_name()?);

                let name = format!(
                    "{}\\{}\\{}",
                    exe_path.display(),
                    save_path,
                    edit_for_windows_compatibility(&file_path.file_name().unwrap().to_str().unwrap().replace(".m3u8",".mp4")));

                Some(pool.execute(move || {
                    tx.send(web::download_build_video(
                        &output_path.to_str().unwrap(),
                        name,
                        &ffmpeg
                    )).unwrap_or(())
                }))
            } else {
                None
            }
        })
        .collect();

    drop(tx);

    for _ in rx.iter().take(episode_url.len()) {
        progress_bar.inc(1);
    }

    progress_bar.finish();
    driver.close_window().await?;
    info!("Clean tmp dir!");
    remove_dir_contents(tmp_dl)?;
    info!("drop pool");
    drop(pool);
    let seconds = before.elapsed().as_secs() % 60;
    let minutes = (before.elapsed().as_secs() / 60) % 60;
    let hours = (before.elapsed().as_secs() / 60) / 60;

    let time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

    info!("Done in: {} for {} episodes",time,episode_url.len());

    Ok(())
}

async fn scan_main_page(save_path: &mut String, driver: &WebDriver, url_test: &str, base_url: &str, tmp_dl: &PathBuf) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    fs::create_dir_all(tmp_dl)?;

    save_path.push_str(
        &edit_for_windows_compatibility(
            &driver.title().await?.replace(" - Neko Sama", "")
        )
    );

    fs::create_dir_all(tmp_dl.parent().unwrap().join(save_path))?;

    Ok(html_parser::recursive_find_url(&driver, url_test, base_url).await?)
}

async fn get_real_video_link(episode_url: &Vec<(String, String)>, driver: &WebDriver, client: &Client, tmp_dl: &PathBuf) -> Result<(), Box<dyn Error>> {

    for (name, url) in episode_url {
        if url.starts_with("http") {

            driver.goto(&url).await?;
            info!("Get m3u8 for: {}", name);

            if let Ok(script) = driver.execute(r#"jwplayer().play(); let ret = jwplayer().getPlaylistItem(); return ret;"#, vec![],).await
            {
                if let Some(url) = script.json()["file"].as_str() {
                    html_parser::fetch_url(url, &name.trim().replace(":", ""), &tmp_dl, &client).await?;
                }

            }else {
                error!("Can't get .m3u8 {url}")
            }
        }else {
            error!("Error with: {name} url: {url}");
        }
    }

    Ok(())

}

fn edit_for_windows_compatibility(name: &str) -> String {
    let regex = Regex::new(r#"[\\/?%*:|"<>]+"#).unwrap();
    regex.replace_all(name, "").to_string()
}
fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    fs::remove_dir_all(path)?;
    Ok(())
}
