use std::error::Error;
use std::fs;
use std::io::{stdin, stdout, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use thirtyfour::{ChromeCapabilities, ChromiumLikeCapabilities, WebDriver};

use crate::thread_pool::ThreadPool;
use crate::{debug, error, html_parser, info, utils_data, vlc_playlist_builder, warn, web};

pub async fn start(
    url_test: &str,
    exe_path: &Path,
    tmp_dl: &PathBuf,
    ublock: &PathBuf,
    ffmpeg: &PathBuf,
    mut thread: usize,
    debug: &bool,
    vlc_playlist: &bool,
    ignore_alert: &bool,
    minimized: &bool,
    langue: &String,
) -> Result<(), Box<dyn Error>> {
    let client = Client::builder().build()?;

    let before = Instant::now();

    let mut save_path = String::new();

    let base_url = "https://neko-sama.fr";

    if *debug {
        debug!("add ublock origin");
    }
    let mut prefs = ChromeCapabilities::new();
    prefs.add_extension(ublock).expect("can't install ublock origin");
    prefs.set_ignore_certificate_errors()?;

    if *debug {
        debug!("connect to chrome driver");
    }
    let driver = WebDriver::new("http://localhost:6969", prefs).await?;
    if *minimized {
        driver.minimize_window().await?;
    }
    driver.set_page_load_timeout(Duration::from_secs(20)).await?;

    driver.goto(url_test).await?;

    info!("Scan Main Page");

    let (good, error) = scan_main_page(&mut save_path, &driver, url_test, base_url, tmp_dl, debug, &client, &ignore_alert, langue).await?;

    info!("total found: {}", good);

    if good == 0 {
        driver.quit().await?;
        return Ok(());
    }

    if error > 0 && *ignore_alert {
        let mut s = String::new();
        print!("Continue with missing episode(s) ? 'Y' continue, 'n' to cancel : ");

        let _ = stdout().flush();
        stdin().read_line(&mut s).expect("Did not enter a correct string");
        if s.trim() == "n" {
            exit(130);
        }
    }

    if good == 0 {
        error!("Nothing found or url down");
        exit(130);
    }

    // kill chromedriver
    if *debug {
        debug!("chromedriver close_window");
    }
    if let Ok(_) = driver.close_window().await{}
    if *debug {
        debug!("chromedriver quit");
    }
    if let Ok(_) = driver.quit().await{}
    if *debug {
        debug!("chromedriver kill process");
    }

    if thread > good as usize {
        warn!("update thread count from {thread} to {good}");
        thread = good as usize;
    }

    let (tx, rx) = mpsc::channel();

    let mut pool = ThreadPool::new(thread, good as usize);

    let mut save_path_vlc = vec![];

    let mut m3u8_path_folder: Vec<_> = fs::read_dir(tmp_dl)?
        .filter_map(|entry| {
            let save = &mut save_path_vlc;

            let entry = entry.ok();
            let file_path = entry?.path();

            if file_path.is_file() {
                let output_path = Path::new(tmp_dl).join(file_path.file_name()?);

                let name =
                    exe_path
                        .join(&save_path)
                        .join(utils_data::edit_for_windows_compatibility(
                            &file_path
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .replace(".m3u8", ".mp4").replace(" ", "_"),
                        ));

                let _ = &mut save.push((name.clone(), &save_path));

                Some((output_path, name))
            } else {
                None
            }
        })
        .collect();

    utils_data::custom_sort(&mut m3u8_path_folder);

    info!("Start Processing with {} threads", thread);

    let progress_bar = ProgressBar::new(good as u64);
    progress_bar.enable_steady_tick(Duration::from_secs(1));

    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:60.cyan/blue} {pos}/{len} ({eta})")?
            .progress_chars("$>-"),
    );

    for (output_path, name) in m3u8_path_folder {
        let tx = tx.clone();
        let ffmpeg = ffmpeg.clone();
        let debug = debug.clone();
        pool.execute(move || {
            tx.send(web::download_build_video(
                &output_path.to_str().unwrap(),
                name.to_str().unwrap(),
                &ffmpeg,
                &debug,
            ))
            .unwrap_or(())
        })
    }

    drop(tx);

    for _ in rx.iter().take(good as usize) {
        progress_bar.inc(1);
    }

    progress_bar.finish();

    if good >= 2 && *vlc_playlist {
        info!("Build vlc playlist");
        utils_data::custom_sort_vlc(&mut save_path_vlc);
        vlc_playlist_builder::new(save_path_vlc)?;
    }

    let seconds = before.elapsed().as_secs() % 60;
    let minutes = (before.elapsed().as_secs() / 60) % 60;
    let hours = (before.elapsed().as_secs() / 60) / 60;

    let time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

    info!("Clean tmp dir!");
    utils_data::remove_dir_contents(tmp_dl);

    info!(
        "Done in: {} for {} episodes and {} error",
        time, good, error
    );
    Ok(())
}

pub async fn scan_main_page(
    save_path: &mut String,
    drivers: &WebDriver,
    url_test: &str,
    base_url: &str,
    tmp_dl: &PathBuf,
    debug: &bool,
    client: &Client,
    ignore_warn: &bool,
    langue: &String
) -> Result<(u16, u16), Box<dyn Error>> {

    fs::create_dir_all(tmp_dl)?;
    let path= format!("Anime_Download/{}/{}", langue.to_uppercase(), &utils_data::edit_for_windows_compatibility(&drivers.title().await?.replace(" - Neko Sama", "").replace(" ", "_")));
    save_path.push_str(path.as_str());

    let season_path = tmp_dl.parent().unwrap().join(save_path);
    if *ignore_warn{
        if fs::try_exists(season_path.clone()).unwrap(){
            warn!("Path already exist\n{}", season_path.display());
            let mut s = String::new();
            print!("Do you want delete this path press Y, or N to ignore and continue: ");
            let _ = stdout().flush();
            stdin().read_line(&mut s).expect("Did not enter a correct string");
            if s.to_lowercase().trim() == "y"{
                fs::remove_dir_all(season_path.clone())?;
            }else {
                info!("Okay path ignored")
            };
        }
    }

    fs::create_dir_all(season_path)?;
    Ok(html_parser::recursive_find_url(&drivers, url_test, base_url, debug, &client, &tmp_dl).await?)
}