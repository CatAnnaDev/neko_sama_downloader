#![feature(fs_try_exists)]

use std::{
    error::Error,
    str::FromStr,
    sync::mpsc,
    time::{Duration, Instant},
};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use requestty::{Answer, OnEsc, prompt_one, Question};
use reqwest::Client;
use thirtyfour::WebDriver;

use neko_process::process::{self, add_ublock, connect_to_chrome_driver};

use crate::chrome::chrome_spawn::ChromeChild;
use crate::cmd_arg::cmd_line_parser;
use crate::cmd_arg::cmd_line_parser::{Args, Scan};
use crate::neko_process::html_parser::enter_iframe_wait_jwplayer;
use crate::neko_process::process::build_path_to_save_final_video;
use crate::search_engine::search;
use crate::search_engine::search::ProcessingUrl;
use crate::thread::thread_pool;
use crate::thread::thread_pool::ThreadPool;
use crate::utils::{static_data, utils_check, utils_data};
use crate::utils::utils_check::AllPath;
use crate::utils_data::time_to_human_time;
use crate::web_client::web;

mod neko_process;
mod chrome;
mod vlc;
mod utils;
mod thread;
mod cmd_arg;
mod search_engine;
mod web_client;

pub struct MainArg {
    new_args: Args,
    path: AllPath,
    processing_url: Vec<ProcessingUrl>,
    client: Client,
}

#[tokio::main]
async fn main()
    -> Result<(), Box<dyn Error>> {
    let mut new_args = cmd_line_parser::Args::parse();

    header!("{}", static_data::HEADER);
    warn!("Please if you got an Error remember to update Google chrome and chromedriver");
    let mut processing_url = None;
    while processing_url.is_none() {
        let _ = ask_keyword(&mut new_args);
        processing_url = setup_search_or_download(&mut new_args).await?;
        if processing_url.is_none() {
            new_args.url_or_search_word = Scan::Search("".to_owned())
        }
    }

    info!("{}", new_args);

    thread_pool::max_thread_check(&mut new_args);

    let path = utils_check::confirm_chrome_ffmpeg_ublock_presence().await?;

    let client = Client::builder().build()?;

    let mut arg = MainArg{
        new_args,
        path,
        processing_url: processing_url.unwrap(),
        client,
    };

    let _ = iter_over_url_found(&mut arg).await?;

    Ok(())
}

async fn start(url_test: &str, driver: WebDriver, main_arg: &MainArg)
    -> Result<(), Box<dyn Error>> {
    let before = Instant::now();

    let all_url_found = process::scan_main(&driver, url_test, main_arg).await?;

    let mut save_path = String::new();
    // make final path to save
    build_path_to_save_final_video(&mut save_path, &driver, url_test, main_arg).await?;

    // iter overs all urls found
    let (good, error) = enter_iframe_wait_jwplayer(&driver, all_url_found, main_arg).await?;

    info!("total found: {}", good);

    process::prevent_case_nothing_found_or_error(good, error, main_arg);

    process::shutdown_chrome(main_arg, &driver).await;

    let mut new_thread = main_arg.new_args.thread;
    if new_thread > good {
        warn!("update thread count from {new_thread} to {good}");
        new_thread = good;
    }

    let (mut vec_m3u8_path_folder, vec_save_path_vlc) = process::build_vec_m3u8_folder_path(&main_arg.path, save_path)?;

    utils_data::custom_sort(&mut vec_m3u8_path_folder);

    info!("Start Processing with {} threads", new_thread);

    let progress_bar = ProgressBar::new(good as u64);
    progress_bar.enable_steady_tick(Duration::from_secs(1));

    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:60.cyan/blue} {pos}/{len} ({eta})")?
            .progress_chars("$>-"),
    );

    let (tx, rx) = mpsc::channel();
    let mut pool = ThreadPool::new(new_thread, good);
    for (output_path, name) in vec_m3u8_path_folder {
        let tx = tx.clone();
        let ffmpeg = main_arg.path.ffmpeg_path.clone();
        let debug = main_arg.new_args.debug.clone();
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

    for _ in rx.iter().take(good) {
        progress_bar.inc(1);
    }

    progress_bar.finish();

    if good >= 2 && main_arg.new_args.vlc_playlist {
        process::build_vlc_playlist(vec_save_path_vlc)?;
    }

    process::end_print(before, &main_arg.path, good, error);

    Ok(())
}

async fn iter_over_url_found(main_arg: &mut MainArg, )
    -> Result<(), Box<dyn Error>> {

    time_it!("Global time:", {

        if main_arg.new_args.debug {
            debug!("spawn chrome process");
        }

        let mut child = ChromeChild::spawn(&main_arg.path.chrome_path);
        if main_arg.new_args.debug {
            debug!("wait 1sec chrome process spawn correctly");
        }

        tokio::time::sleep(Duration::from_secs(1)).await;

        for (index, x) in main_arg.processing_url.iter().enumerate() {
            header!("Step {} / {}", index + 1, main_arg.processing_url.len());
            info!("Process: {}", x.url);
            start(&x.url, connect_to_chrome_driver(&main_arg, add_ublock(&main_arg)?, &x.url).await?, &main_arg).await?;
        }

        child.chrome.kill()?;
    });

    Ok(())
}

async fn setup_search_or_download(new_args: &mut Args, )
    -> Result<Option<Vec<ProcessingUrl>>, Box<dyn Error>> {
    let processing_url = match new_args.url_or_search_word {
        Scan::Search(ref keyword) => {
            match search::search_over_json(&keyword, &new_args.language, &new_args.debug).await{
                Ok(find) => {
                    if find.len() != 0{
                        build_print_nb_ep_film(&find);
                        let answer = build_question(&find)?;
                        Some(find_real_link_with_answer(&find, answer))
                    }else { None }
                }
                Err(_) => {
                    None
                }
            }

        }

        Scan::Download(ref url) => {
            Some(vec![ProcessingUrl {
                name: "".to_string(),
                ep: "".to_string(),
                url: url.to_string(),
                genre: "".to_string(),
            }])
        }
    };

    Ok(processing_url)
}

fn find_real_link_with_answer(find: &Vec<ProcessingUrl>, answer: Answer)
    -> Vec<ProcessingUrl> {
    answer
        .try_into_list_items()
        .unwrap()
        .iter()
        .filter_map(|number| find.get(number.index).cloned())
        .collect()
}

fn build_question(find: &Vec<ProcessingUrl>)
    -> requestty::Result<Answer> {
    let multi_select = Question::multi_select("Season")
        .message("What seasons do you want?")
        .choices(
            find.iter()
                .map(|s| {
                    let tmp_genre = s.clone().genre;
                    format!(
                        "{} ({})\n[{}]",
                        s.name,
                        s.ep,
                        if tmp_genre.is_empty() {
                            String::from("no tag found")
                        } else {
                            tmp_genre
                        }
                    )
                })
                .collect::<Vec<String>>(),
        )
        .on_esc(OnEsc::Terminate)
        .page_size(20)
        .should_loop(false)
        .build();

    prompt_one(multi_select)
}

fn build_print_nb_ep_film(find: &Vec<ProcessingUrl>) {
    let mut ep = 0;
    let mut film = 0;

    let _: Vec<_> = find
        .iter()
        .map(|s| {
            if s.ep.starts_with("Film") {
                film += 1;
            } else {
                ep +=
                    s.ep.split_whitespace()
                        .nth(0)
                        .unwrap()
                        .parse::<i32>()
                        .unwrap_or(1);
            };
        })
        .collect();

    header!(
        "Seasons found: {} Episode found: {} ({}~ Go Total) Films found {} ({}~ Go Total)",
        find.len(),
        ep,
        ep * 230 / 1024,
        film,
        film * 1300 / 1024
    );
}

fn ask_keyword(new_args: &mut Args)
    -> Result<(), Box<dyn Error>> {
    if new_args.url_or_search_word.is_empty() {
        if let Ok(reply) = utils_data::ask_keyword("Enter url to direct download or keyword to search: ")
        {
            new_args.url_or_search_word = Scan::from_str(reply.as_string().unwrap().trim())?;
        }
    }
    Ok(())
}
