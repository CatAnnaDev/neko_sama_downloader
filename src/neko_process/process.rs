use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    process::exit,
    time::Instant,
};
use chromiumoxide::Page;

use crate::{AllPath, error, info, MainArg, warn};
use crate::cmd_arg::cmd_line_parser::Args;
use crate::neko_process::html_parser;
use crate::utils::utils_data;
use crate::utils::utils_data::ask_something;
use crate::vlc::vlc_playlist_builder;

pub async fn scan_main(driver: &Page, url_test: &str, main_arg: &MainArg)
    -> Result<Vec<String>, Box<dyn Error>> {
    info!("Scan Main Page");

    // found all urls
    let all_url_found = html_parser::recursive_find_url(&driver, url_test, main_arg).await?;

    Ok(all_url_found)
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


pub fn build_vec_m3u8_folder_path(path: &AllPath, save_path: String)
    -> Result<(Vec<(PathBuf, PathBuf)>, Vec<(PathBuf, String)>), Box<dyn Error>> {
    let mut save_path_vlc = vec![];

    let m3u8_path_folder: Vec<_> = fs::read_dir(&path.m3u8_tmp)?
        .filter_map(|entry| {

            let entry = entry.ok();
            let file_path = entry?.path();
            if file_path.is_file() {
                let output_path = Path::new(&path.m3u8_tmp).join(file_path.file_name()?);
                let name = path
                    .tmp_path
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
                save_path_vlc.push((name.clone(), save_path.clone()));
                Some((output_path, name))
            } else {
                None
            }
        })
        .collect();
    Ok((m3u8_path_folder, save_path_vlc))
}

pub fn build_vlc_playlist(mut save_path_vlc: Vec<(PathBuf, String)>)
    -> Result<(), Box<dyn Error>> {
    info!("Build vlc playlist");
    utils_data::custom_sort_vlc(&mut save_path_vlc);
    vlc_playlist_builder::new(save_path_vlc)?;
    Ok(())
}

pub async fn build_path_to_save_final_video(save_path: &mut String, page: &Page, url_test: &str, main_arg: &MainArg)
    -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(&main_arg.path.tmp_path)?;

    let name = get_name_based_on_url(url_test, &main_arg.new_args, &page).await?;

    save_path.push_str(name.as_str());

    let season_path = main_arg.path.tmp_path.join(save_path);
    if main_arg.new_args.ignore_alert_missing_episode {
        if let Ok(pa) = tokio::fs::try_exists(&season_path).await {
            if pa {
                warn!("Path already exist\n{}", season_path.display());
                if let Ok(e) = ask_something("Delete this path (Y) or ignore and continue (N):") {
                    if e.as_bool().unwrap() {
                        println!("{}", season_path.display());
                        fs::remove_dir_all(&season_path)?;
                    } else {
                        info!("Okay path ignored")
                    }
                }
            }
        }
    }
    fs::create_dir_all(&season_path)?;
    Ok(())
}

async fn get_base_name_direct_url(page: &Page) // TODO change this
    -> String {
    page.get_title().await.unwrap().unwrap()
}

async fn get_name_based_on_url(url_test: &str, args: &Args, page: &Page)
    -> Result<String, Box<dyn Error>> {
    let path = if !url_test.contains("/episode/") {
        format!(
            "Anime_Download/{}/{}",
            args.language.to_uppercase(),
            &utils_data::edit_for_windows_compatibility(
                &page.get_title().await.unwrap().unwrap()
                    .replace(" - Neko Sama", "")
                    .replace(" ", "_")
            )
        )
    } else {
        format!(
            "Anime_Download/{}/{}",
            args.language.to_uppercase(),
            &utils_data::edit_for_windows_compatibility(
                &get_base_name_direct_url(&page)
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
    utils_data::remove_dir_contents(&path.m3u8_tmp);
    info!(
        "Done in: {} for {} episodes and {} error",
        utils_data::time_to_human_time(before),
        good,
        if error >= 1 { format!("\x1B[31m{error}") } else { error.to_string() }
    );
}
