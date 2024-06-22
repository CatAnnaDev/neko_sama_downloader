use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::exit,
    time::{Duration, Instant},
};

use thirtyfour::{ChromeCapabilities, ChromiumLikeCapabilities, WebDriver};

use crate::{debug, error, info, MainArg, warn};
use crate::cmd_arg::cmd_line_parser::Args;
use crate::neko_process::{html_parser, html_parser::get_base_name_direct_url};
use crate::neko_process::html_parser::enter_iframe_wait_jwplayer;
use crate::utils::utils_check::AllPath;
use crate::utils::utils_data;
use crate::utils::utils_data::ask_something;
use crate::vlc::vlc_playlist_builder;

pub async fn scan_main(driver: &WebDriver, url_test: &str, main_arg: &MainArg)
    -> Result<(String, usize, usize), Box<dyn Error>> {
    info!("Scan Main Page");
    
    // found all urls 
    let all_url_found = html_parser::recursive_find_url(&driver, url_test, main_arg).await?;

    let mut save_path = String::new();
    // make final path to save
    build_path_to_save_final_video(&mut save_path, &driver, url_test, main_arg).await?;
    
    // iter overs all urls found 
    let (good, error) = enter_iframe_wait_jwplayer(&driver, all_url_found, main_arg).await?;

    info!("total found: {}", good);
        
    Ok((save_path, good, error))
}

pub fn prevent_case_nothing_found_or_error(good: usize, error: usize, args: &MainArg) {
    if error > 0 && args.new_args.ignore_alert_missing_episode {
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

pub async fn shutdown_chrome(args: &MainArg, driver: &WebDriver) {
    // kill chromedriver
    if args.new_args.debug {
        debug!("chromedriver close_window");
    }
    if let Ok(_) = <WebDriver as Clone>::clone(&driver).close_window().await {}
    if args.new_args.debug {
        debug!("chromedriver quit");
    }
    if let Ok(_) = <WebDriver as Clone>::clone(&driver).quit().await {}
    if args.new_args.debug {
        debug!("chromedriver kill process");
    }
}

pub fn add_ublock(args: &MainArg)
    -> Result<ChromeCapabilities, Box<dyn Error>> {
    if args.new_args.debug {
        debug!("add ublock origin");
    }
    let mut prefs = ChromeCapabilities::new();
    prefs
        .add_extension(&*args.path.u_block_path)
        .expect("can't install ublock origin");
    prefs.set_ignore_certificate_errors()?;
    Ok(prefs)
}

pub(crate) fn build_vec_m3u8_folder_path(path: &AllPath, save_path: String, )
    -> Result<(Vec<(PathBuf, PathBuf)>, Vec<(PathBuf, String)>), Box<dyn Error>> {
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

pub fn build_vlc_playlist(mut save_path_vlc: Vec<(PathBuf, String)>, )
    -> Result<(), Box<dyn Error>> {
    info!("Build vlc playlist");
    utils_data::custom_sort_vlc(&mut save_path_vlc);
    vlc_playlist_builder::new(save_path_vlc)?;
    Ok(())
}

pub async fn connect_to_chrome_driver(args: &MainArg, prefs: ChromeCapabilities, url_test: &str, )
    -> Result<WebDriver, Box<dyn Error>> {

    if args.new_args.debug {
        debug!("connect to chrome driver");
    }

    let driver = WebDriver::new("http://localhost:6969", prefs).await?;

    if args.new_args.minimized_chrome {
        driver.minimize_window().await?;
    }

    driver.set_page_load_timeout(Duration::from_secs(20)).await?;
    driver.goto(url_test).await?;

    Ok(driver)
}

async fn build_path_to_save_final_video(save_path: &mut String, drivers: &WebDriver, url_test: &str, main_arg: &MainArg)
    -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&main_arg.path.tmp_dl)?;

    let name = get_name_based_on_url(url_test, &main_arg.new_args, &drivers).await?;

    save_path.push_str(name.as_str());

    let season_path = main_arg.path.tmp_dl.parent().unwrap().join(save_path);
    if main_arg.new_args.ignore_alert_missing_episode {
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
    Ok(())
}

async fn get_name_based_on_url(url_test: &str, args: &Args, drivers: &WebDriver, )
    -> Result<String, Box<dyn Error>> {

    let path = if !url_test.contains("/episode/") {
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
    Ok(path)
}

pub fn end_print(before: Instant, path: &AllPath, good: usize, error: usize) {
    info!("Clean tmp dir!");
    utils_data::remove_dir_contents(&path.tmp_dl);
    info!(
        "Done in: {} for {} episodes and {} error",
        utils_data::time_to_human_time(before),
        good,
        if error >= 1 { format!("\x1B[31m{error}") } else { error.to_string() }
    );
}
