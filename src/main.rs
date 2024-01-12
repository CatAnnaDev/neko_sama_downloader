#![feature(pattern)]
#![feature(fs_try_exists)]
#![feature(stmt_expr_attributes)]

use std::{error::Error, fs, time::Instant, io::Write, thread};
use clap::Parser;
use requestty::{OnEsc, prompt_one, Question};
use crate::chrome_spawn::spawn_chrome;
use crate::search::ProcessingUrl;

mod cmd_line_parser;
mod html_parser;
mod log_color;
mod process_part1;
mod search;
mod static_data;
mod thread_pool;
mod utils_check;
mod utils_data;
mod vlc_playlist_builder;
mod web;
mod chrome_spawn;

enum Scan {
    Download,
    Search
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // print!("\x1B[2J\x1B[1;1H");
    let mut new_args = cmd_line_parser::Args::parse();

    header!(r#"
  _   _      _                   _ _
 | \ | | ___| | _____         __| | |
 |  \| |/ _ \ |/ / _ \ _____ / _` | |
 | |\  |  __/   < (_) |_____| (_| | |
 |_| \_|\___|_|\_\___/       \__,_|_|
                          by PsykoDev
"#);

    if new_args.url_or_search_word.is_empty() {
        warn!("prefers use ./anime_dl -h\n");
        let questions = Question::input("keyword")
            .message("Enter url to direct download or keyword to search: ")
            .build();
        let reply = prompt_one(questions)?;
        new_args.url_or_search_word = reply.as_string().unwrap().trim().to_string();
    }

    info!(
    "Config:\n\
    Url or Search:\t{}\n\
    Language:\t{}\n\
    Threads:\t{}\n\
    Vlc playlist:\t{}\n\
    Ignore Alert:\t{}\n\
    Minimized:\t{}\n\
    Debug:\t\t{}",
        new_args.url_or_search_word,
        new_args.language,
        new_args.thread,
        new_args.vlc_playlist,
        new_args.ignore_alert_missing_episode,
        new_args.minimized_chrome,
        new_args.debug,

    );

    let path = utils_check::check()?;

    let mut chrome_check = false;
    let mut ffmpeg_check = false;
    let mut ublock_check = false;

    let mut thread = new_args.thread as usize;
    let max_thread = thread::available_parallelism()?.get() * 4;
    if thread > max_thread {
        warn!("Max thread for your cpu is between 1 and {}", max_thread);
        thread = max_thread;
    }

    let mut processing_url = vec![];
    let mut _scan = Scan::Search;

    if new_args.url_or_search_word.starts_with("https://neko-sama.fr/") { _scan = Scan::Download; }
    else { _scan = Scan::Search; }

    match _scan {
        Scan::Search => {
            let find = search::search_over_json(
                &new_args.url_or_search_word,
                &new_args.language,
                &new_args.debug,
            )
            .await?;

            let multi_select = Question::multi_select("Season")
                .message("What seasons do you want?")
                .choices(find.iter().map(|s| format!("{} ({})\n[{}]", s.name, s.ep, s.genre)).collect::<Vec<String>>())
                .on_esc(OnEsc::Terminate)
                .page_size(20)
                .should_loop(false)
                .build();

            let answer = prompt_one(multi_select)?;
            let matching_processing_urls: Vec<_> =
                answer
                    .try_into_list_items()
                    .unwrap()
                    .iter()
                    .filter_map(|number|  find.get(number.index).cloned() )
                    .collect();

            processing_url.extend(matching_processing_urls);

        }
        Scan::Download => {
            processing_url.extend(vec![ProcessingUrl {
                name: "".to_string(),
                ep: "".to_string(),
                url: new_args.url_or_search_word.clone(),
                genre: "".to_string(),
            }]);
        }
    }

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

                #[cfg(any(
                    target_os = "macos",
                    target_os = "linux"
                ))]
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
        utils_check::download(static_data::UBLOCK_PATH, &path.ublock_destination)
            .await
            .expect("Erreur lors du téléchargement de uBlock Origin.");
    }

    match ffmpeg_check && chrome_check && ublock_check {
        true => {
            let global_time = Instant::now();
            if new_args.debug {
                debug!("spawn chrome process");
            }
            spawn_chrome(&path.chrome_path);
            for x in processing_url {
                info!("Process: {}", x.url);
                process_part1::start(
                    &x.url,
                    &path.exe_path.parent().unwrap(),
                    &path.tmp_dl,
                    &path.u_block_path,
                    &path.ffmpeg_path,
                    thread,
                    &new_args
                ).await?;
            }

            info!(
                "Global time: {}",
                utils_data::time_to_human_time(global_time)
            );
        }
        false => {
            if !ffmpeg_check && chrome_check {
                error!(
                    "Please download then extract {} ffmpeg here:\n{}",
                    path.ffmpeg_path.display(),
                    static_data::FFMPEG_PATH
                );
            } else if !chrome_check && ffmpeg_check {
                error!(
                    "Please download chrome wed driver then extract {} in utils folder here:\n{}",
                    path.chrome_path.display(),
                    static_data::DRIVER_PATH
                );
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
            }
        }
    }

    utils_data::kill_process()?;
    Ok(())
}

fn _pick_season_list(input: &str, processing_url: Vec<ProcessingUrl>) -> Result<Vec<ProcessingUrl>, Box<dyn Error>> {
    let numbers: Vec<usize> = input.split(|c: char| !c.is_digit(10)).filter_map(|s| s.parse().ok()).collect();
    Ok(numbers.iter().filter_map(|&number| { processing_url.get(number - 1).map(|url| url.clone()) }).collect())
}
