#![feature(fs_try_exists)]

use std::{error::Error, time::{Duration, Instant}, str::FromStr, sync::mpsc};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use requestty::{Answer, OnEsc, prompt_one, Question};
use reqwest::Client;
use thirtyfour::WebDriver;

use mod_file::{
    {search, search::ProcessingUrl},
    {utils_data, utils_data::time_to_human_time}, chrome_spawn::ChromeChild,
    cmd_line_parser,
    cmd_line_parser::Scan, process_part1, process_part1::{add_ublock, connect_to_chrome_driver},
    static_data,
    thread_pool,
    utils_check,
};
use crate::mod_file::cmd_line_parser::Args;
use crate::mod_file::thread_pool::ThreadPool;
use crate::mod_file::utils_check::AllPath;
use crate::mod_file::web;

mod mod_file;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut new_args = cmd_line_parser::Args::parse();

    header!("{}", static_data::HEADER);

    let _ = ask_keyword(&mut new_args);

    info!("{}", new_args);

    let thread = thread_pool::max_thread_check(&new_args)?;

    let client = Client::builder().build()?;

    let path = utils_check::confirm_chrome_ffmpeg_ublock_presence().await?;

    let processing_url = setup_search_or_download(&mut new_args).await?;

    let _ = iter_over_url_found(&new_args, &path, processing_url, thread, &client).await?;

    Ok(())
}

async fn start(url_test: &str, path: &AllPath, mut thread: usize, args: &Args, driver: WebDriver, client: &Client) -> Result<(), Box<dyn Error>> {
    let before = Instant::now();

    let (save_path, good, error) = process_part1::scan_main(&driver, url_test, path, &client, args).await?;

    process_part1::prevent_case_nothing_found_or_error(good, error, args);

    process_part1::shutdown_chrome(args, &driver).await;

    if thread > good as usize {
        warn!("update thread count from {thread} to {good}");
        thread = good as usize;
    }

    let (mut vec_m3u8_path_folder, vec_save_path_vlc) =
        process_part1::build_vec_m3u8_folder_path(path, save_path)?;

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

    process_part1::build_vlc_playlist(good, args, vec_save_path_vlc)?;

    process_part1::end_print(before, path, good, error);

    Ok(())
}

async fn iter_over_url_found(new_args: &Args, path: &AllPath, processing_url: Vec<ProcessingUrl>, thread: usize, client: &Client) -> Result<(), Box<dyn Error>>{
    time_it!("Global time:", {
        if new_args.debug {
            debug!("spawn chrome process");
        }

        let mut child = ChromeChild::spawn(&path.chrome_path);
        if new_args.debug {
            debug!("wait 1sec chrome process spawn correctly");
        }
        tokio::time::sleep(Duration::from_secs(1)).await;

        for (index, x) in processing_url.iter().enumerate() {
            header!("Step {} / {}", index + 1, processing_url.len());
            info!("Process: {}", x.url);
            let driver = connect_to_chrome_driver(&new_args, add_ublock(&new_args, &path)?, &x.url).await?;

            start(&x.url, &path, thread, &new_args, driver, client).await?;
        }

        child.chrome.kill()?;
    });

    Ok(())
}

async fn setup_search_or_download(new_args: &mut Args) -> Result<Vec<ProcessingUrl>, Box<dyn Error>>{

    let processing_url = match new_args.url_or_search_word {
        Scan::Search(ref keyword) => {
            let find = search::search_over_json(&keyword, &new_args.language, &new_args.debug).await?;
            build_print_nb_ep_film(&find);
            let answer = build_question(&find)?;
            find_real_link_with_answer(&find, answer)
        }

        Scan::Download(ref url) => {
            vec![ProcessingUrl {
                name: "".to_string(),
                ep: "".to_string(),
                url: url.to_string(),
                genre: "".to_string(),
            }]
        }
    };

    Ok(processing_url)
}

fn find_real_link_with_answer(find: &Vec<ProcessingUrl>, answer: Answer, ) -> Vec<ProcessingUrl> {
    answer
        .try_into_list_items()
        .unwrap()
        .iter()
        .filter_map(|number| find.get(number.index).cloned())
        .collect()
}

fn build_question(find: &Vec<ProcessingUrl>) -> requestty::Result<Answer> {
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

fn build_print_nb_ep_film(find: &Vec<ProcessingUrl>){
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
                ep * 250 / 1024,
                film,
                film * 1300 / 1024
            );
}

fn ask_keyword(new_args: &mut Args) -> Result<(), Box<dyn Error>> {
    if new_args.url_or_search_word.is_empty() {
        warn!("prefers use ./{} -h", utils_data::exe_name());
        if let Ok(reply) = utils_data::ask_keyword("Enter url to direct download or keyword to search: ")
        {
            new_args.url_or_search_word = Scan::from_str(reply.as_string().unwrap().trim())?;
        }
    }
    Ok(())
}
