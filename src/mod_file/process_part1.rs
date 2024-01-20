use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::exit,
    sync::mpsc,
    time::{Duration, Instant},
};

use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use thirtyfour::{ChromeCapabilities, ChromiumLikeCapabilities, WebDriver};

use crate::{debug, error, info, warn};
use crate::mod_file::{
    {html_parser, html_parser::get_base_name_direct_url},
    {utils_data, utils_data::ask_something},
    cmd_line_parser::Args,
    thread_pool::ThreadPool, utils_check::AllPath,
    vlc_playlist_builder,
    web,
};

pub async fn start(url_test: &str, path: &AllPath, mut thread: usize, args: &Args, driver: WebDriver, ) -> Result<(), Box<dyn Error>> {
    let client = Client::builder().build()?;
    let before = Instant::now();

    let (save_path, good, error) = scan_main(&driver, url_test, path, &client, args).await?;

    prevent_case_nothing_found_or_error(good, error, args);

    shutdown_chrome(args, &driver).await;

    if thread > good as usize {
        warn!("update thread count from {thread} to {good}");
        thread = good as usize;
    }

    let (mut vec_m3u8_path_folder, vec_save_path_vlc) =
        build_vec_m3u8_folder_path(path, save_path)?;

    utils_data::custom_sort(&mut vec_m3u8_path_folder);

    info!("Start Processing with {} threads", thread);

    let progress_bar = ProgressBar::new(good as u64);
    progress_bar.enable_steady_tick(Duration::from_secs(1));

    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:60.cyan/blue} {pos}/{len} ({eta})")?
            .progress_chars("$>-"),
    );

    let (tx, rx) = mpsc::channel();
    let mut pool = ThreadPool::new(thread, good as usize);
    for (output_path, name) in vec_m3u8_path_folder {
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

    build_vlc_playlist(good, args, vec_save_path_vlc)?;

    end_print(before, path, good, error);

    Ok(())
}

pub fn add_ublock(args: &Args, path: &AllPath) -> Result<ChromeCapabilities, Box<dyn Error>> {
    if args.debug {
        debug!("add ublock origin");
    }
    let mut prefs = ChromeCapabilities::new();
    prefs
        .add_extension(&*path.u_block_path)
        .expect("can't install ublock origin");
    prefs.set_ignore_certificate_errors()?;
    Ok(prefs)
}

pub async fn connect_to_chrome_driver(args: &Args, prefs: ChromeCapabilities, url_test: &str, ) -> Result<WebDriver, Box<dyn Error>> {
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

    Ok(driver)
}

async fn scan_main(driver: &WebDriver,url_test: &str,path: &AllPath,client: &Client,args: &Args,) -> Result<(String, u16, u16), Box<dyn Error>> {
    info!("Scan Main Page");
    let mut save_path = String::new();

    let (good, error) =
        build_path_to_save_final_video(&mut save_path, &driver, url_test, path, &client, &args)
            .await?;

    info!("total found: {}", good);

    Ok((save_path, good, error))
}

fn prevent_case_nothing_found_or_error(good: u16, error: u16, args: &Args) {
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
}

async fn shutdown_chrome(args: &Args, driver: &WebDriver) {
    // kill chromedriver
    if args.debug {
        debug!("chromedriver close_window");
    }
    if let Ok(_) = <WebDriver as Clone>::clone(&driver).close_window().await {}
    if args.debug {
        debug!("chromedriver quit");
    }
    if let Ok(_) = <WebDriver as Clone>::clone(&driver).quit().await {}
    if args.debug {
        debug!("chromedriver kill process");
    }
}

fn build_vec_m3u8_folder_path(path: &AllPath, save_path: String, ) -> Result<(Vec<(PathBuf, PathBuf)>, Vec<(PathBuf, String)>), Box<dyn Error>> {
    let mut save_path_vlc = vec![];

    let m3u8_path_folder: Vec<_> = fs::read_dir(&path.tmp_dl)?
        .filter_map(|entry| {
            let save = &mut save_path_vlc;

            let entry = entry.ok();
            let file_path = entry?.path();

            if file_path.is_file() {
                let output_path = Path::new(&path.tmp_dl).join(file_path.file_name()?);
                let name = path
                    .exe_path
                    .parent()
                    .unwrap()
                    .join(save_path.clone())
                    .join(utils_data::edit_for_windows_compatibility(
                        &file_path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .replace(".m3u8", ".mp4")
                            .replace(" ", "_"),
                    ));
                save.push((name.clone(), save_path.clone()));
                Some((output_path, name))
            } else {
                None
            }
        })
        .collect();

    Ok((m3u8_path_folder, save_path_vlc))
}

async fn build_path_to_save_final_video(save_path: &mut String, drivers: &WebDriver, url_test: &str, path: &AllPath, client: &Client, args: &Args, ) -> Result<(u16, u16), Box<dyn Error>> {
    fs::create_dir_all(&path.tmp_dl)?;

    let mut _name = get_name_based_on_url(url_test, args, &drivers).await?;
    save_path.push_str(_name.as_str());

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
    Ok(html_parser::recursive_find_url(&drivers, url_test, args, &client, path).await?)
}

async fn get_name_based_on_url(url_test: &str, args: &Args, drivers: &WebDriver, ) -> Result<String, Box<dyn Error>> {
    let _path = if !url_test.contains("/episode/") {
        format!(
            "Anime_Download/{}/{}",
            args.language.to_uppercase(),
            &utils_data::edit_for_windows_compatibility(
                &drivers
                    .title()
                    .await?
                    .replace(" - Neko Sama", "")
                    .replace(" ", "_")
            )
        )
    } else {
        format!(
            "Anime_Download/{}/{}",
            args.language.to_uppercase(),
            &utils_data::edit_for_windows_compatibility(
                &get_base_name_direct_url(&drivers)
                    .await
                    .replace(" - Neko Sama", "")
                    .replace(" ", "_")
            )
        )
    };
    Ok(_path)
}

fn build_vlc_playlist(good: u16, args: &Args, mut save_path_vlc: Vec<(PathBuf, String)>, ) -> Result<(), Box<dyn Error>> {
    if good >= 2 && args.vlc_playlist {
        info!("Build vlc playlist");
        utils_data::custom_sort_vlc(&mut save_path_vlc);
        vlc_playlist_builder::new(save_path_vlc)?;
    }
    Ok(())
}

fn end_print(before: Instant, path: &AllPath, good: u16, error: u16) {
    info!("Clean tmp dir!");
    utils_data::remove_dir_contents(&path.tmp_dl);
    info!(
        "Done in: {} for {} episodes and {} error",
        utils_data::time_to_human_time(before),
        good,
        error
    );
}
