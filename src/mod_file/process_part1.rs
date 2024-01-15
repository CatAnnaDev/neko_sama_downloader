use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::exit;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use thirtyfour::{ChromeCapabilities, ChromiumLikeCapabilities, WebDriver};

use crate::{
    debug,
    error,
    info,
    mod_file::{
        html_parser,
        html_parser::get_base_name_direct_url
    },
    mod_file::{
        utils_data,
        utils_data::ask_something
    },
    mod_file::cmd_line_parser::Args,
    mod_file::thread_pool::ThreadPool,
    mod_file::utils_check::AllPath,
    mod_file::vlc_playlist_builder,
    mod_file::web,
    warn,
};

pub async fn start(url_test: &str, path: &AllPath, mut thread: usize, args: &Args) -> Result<(), Box<dyn Error>> {
    let client = Client::builder().build()?;

    let before = Instant::now();

    let mut save_path = String::new();

    let base_url = "https://neko-sama.fr";

    if args.debug {
        debug!("add ublock origin");
    }
    let mut prefs = ChromeCapabilities::new();
    prefs
        .add_extension(&*path.u_block_path)
        .expect("can't install ublock origin");
    prefs.set_ignore_certificate_errors()?;

    if args.debug {
        debug!("connect to chrome driver");
    }
    let driver = WebDriver::new("http://localhost:6969", prefs).await?;
    if args.minimized_chrome {
        driver.minimize_window().await?;
    }
    driver
        .set_page_load_timeout(Duration::from_secs(20))
        .await?;

    driver.goto(url_test).await?;

    info!("Scan Main Page");

    let (good, error) = scan_main_page(
        &mut save_path,
        &driver,
        url_test,
        base_url,
        path,
        &client,
        &args,
    )
        .await?;

    info!("total found: {}", good);

    if good == 0 {
        driver.quit().await?;
        return Ok(());
    }

    if error > 0 && args.ignore_alert_missing_episode {
        if let Ok(e) =
            ask_something("Continue with missing episode(s) ? 'Y' continue, 'n' to cancel : ")
        {
            if e.as_bool().unwrap() {
                info!("Okay continue")
            } else {
                exit(130);
            }
        }
    }

    if good == 0 {
        error!("Nothing found or url down");
        exit(130);
    }

    // kill chromedriver
    if args.debug {
        debug!("chromedriver close_window");
    }
    if let Ok(_) = driver.close_window().await {}
    if args.debug {
        debug!("chromedriver quit");
    }
    if let Ok(_) = driver.quit().await {}
    if args.debug {
        debug!("chromedriver kill process");
    }

    if thread > good as usize {
        warn!("update thread count from {thread} to {good}");
        thread = good as usize;
    }

    let (tx, rx) = mpsc::channel();

    let mut pool = ThreadPool::new(thread, good as usize);

    let mut save_path_vlc = vec![];

    let mut m3u8_path_folder: Vec<_> = fs::read_dir(&path.tmp_dl)?
        .filter_map(|entry| {
            let save = &mut save_path_vlc;

            let entry = entry.ok();
            let file_path = entry?.path();

            if file_path.is_file() {
                let output_path = Path::new(&path.tmp_dl).join(file_path.file_name()?);
                let name =
                    path.exe_path.parent().unwrap()
                        .join(&save_path)
                        .join(utils_data::edit_for_windows_compatibility(
                            &file_path
                                .file_name()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .replace(".m3u8", ".mp4")
                                .replace(" ", "_"),
                        ));
                save.push((name.clone(), &save_path));
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
        let ffmpeg = path.ffmpeg_path.clone();
        let debug = args.debug.clone();
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

    if good >= 2 && args.vlc_playlist {
        info!("Build vlc playlist");
        utils_data::custom_sort_vlc(&mut save_path_vlc);
        vlc_playlist_builder::new(save_path_vlc)?;
    }

    let seconds = before.elapsed().as_secs() % 60;
    let minutes = (before.elapsed().as_secs() / 60) % 60;
    let hours = (before.elapsed().as_secs() / 60) / 60;

    let time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

    info!("Clean tmp dir!");
    utils_data::remove_dir_contents(&path.tmp_dl);

    info!(
        "Done in: {} for {} episodes and {} error",
        time, good, error
    );
    Ok(())
}

pub async fn scan_main_page(save_path: &mut String, drivers: &WebDriver, url_test: &str, base_url: &str, path: &AllPath, client: &Client, args: &Args) -> Result<(u16, u16), Box<dyn Error>> {
    fs::create_dir_all(&path.tmp_dl)?;
    let mut _path = String::new();
    if !url_test.contains("/episode/") {
        _path = format!(
            "Anime_Download/{}/{}",
            args.language.to_uppercase(),
            &utils_data::edit_for_windows_compatibility(
                &drivers
                    .title()
                    .await?
                    .replace(" - Neko Sama", "")
                    .replace(" ", "_")
            )
        );
    } else {
        _path = format!(
            "Anime_Download/{}/{}",
            args.language.to_uppercase(),
            &utils_data::edit_for_windows_compatibility(
                &get_base_name_direct_url(&drivers)
                    .await
                    .replace(" - Neko Sama", "")
                    .replace(" ", "_")
            )
        );
    }
    save_path.push_str(_path.as_str());

    let season_path = path.tmp_dl.parent().unwrap().join(save_path);
    if args.ignore_alert_missing_episode {
        if fs::try_exists(season_path.clone()).unwrap() {
            warn!("Path already exist\n{}", season_path.display());
            if let Ok(e) = ask_something("Delete this path (Y) or ignore and continue (N):") {
                if e.as_bool().unwrap() {
                    println!("{}", season_path.display());
                    fs::remove_dir_all(season_path.clone())?;
                } else {
                    info!("Okay path ignored")
                }
            }
        }
    }

    fs::create_dir_all(season_path)?;
    Ok(
        html_parser::recursive_find_url(&drivers, url_test, base_url, args, &client, path)
            .await?,
    )
}