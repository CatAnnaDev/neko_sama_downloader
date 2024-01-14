#![feature(pattern)]
#![feature(fs_try_exists)]
#![feature(stmt_expr_attributes)]

use std::{error::Error, thread, time::Instant};
use std::time::Duration;

use clap::Parser;
use requestty::{OnEsc, prompt_one, Question};

use chrome_spawn::{kill_chrome, spawn_chrome};
use crate::search::ProcessingUrl;

mod chrome_spawn;
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

enum Scan {
    Download,
    Search,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut new_args = cmd_line_parser::Args::parse();
    header!("{}", static_data::HEADER);

    if new_args.url_or_search_word.is_empty() {
        warn!("prefers use ./{} -h", utils_data::exe_name());
        let questions = Question::input("keyword")
            .message("Enter url to direct download or keyword to search: ")
            .build();
        let reply = prompt_one(questions)?;
        new_args.url_or_search_word = reply.as_string().unwrap().trim().to_string();
    }

    info!("{}", new_args);

    let mut thread = new_args.thread as usize;
    let max_thread = thread::available_parallelism()?.get() * 4;
    if thread > max_thread {
        warn!("Max thread for your cpu is between 1 and {}", max_thread);
        thread = max_thread;
        warn!("Update thread for {} continue", thread);
    }

    let mut processing_url = vec![];
    let mut _scan = Scan::Search;

    if new_args.url_or_search_word.starts_with("https://neko-sama.fr/") {
        _scan = Scan::Download;
    } else {
        _scan = Scan::Search;
    }

    match _scan {
        Scan::Search => {
            let find = search::search_over_json(&new_args.url_or_search_word, &new_args.language, &new_args.debug, ).await?;

            let mut ep = 0;
            let mut film = 0;

            let _: Vec<_> = find.iter().map(|s| {
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
                }).collect();

            header!("Seasons found: {} Episode found: {} ({}~ Go Total) Films found {} ({}~ Go Total)",find.len(),ep,ep * 250 / 1024,film,film * 1300 / 1024);

            let multi_select = Question::multi_select("Season")
                .message("What seasons do you want?")
                .choices(find.iter()
                        .map(|s| {
                            let tmp_genre = s.clone().genre;
                            format!("{} ({})\n[{}]", s.name, s.ep, if tmp_genre.is_empty(){ String::from("no tag found") }else { tmp_genre })
                        }).collect::<Vec<String>>(),
                ).on_esc(OnEsc::Terminate).page_size(20).should_loop(false).build();

            let answer = prompt_one(multi_select)?;
            let matching_processing_urls: Vec<_> = answer.try_into_list_items().unwrap()
                .iter().filter_map(|number| find.get(number.index).cloned()).collect();

            processing_url.extend(matching_processing_urls);
        }

        Scan::Download => {
            processing_url.extend(vec![ProcessingUrl { name: "".to_string(), ep: "".to_string(), url: new_args.url_or_search_word.clone(), genre: "".to_string(), }]);
        }
    }

    let path = utils_check::confirm().await?;

    let global_time = Instant::now();
    if new_args.debug {
        debug!("spawn chrome process");
    }

    let child = spawn_chrome(&path.chrome_path)?;
    if new_args.debug {
        debug!("wait 1sec chrome process spawn correctly");
    }
    tokio::time::sleep(Duration::from_secs(1)).await;

    for (index, x) in processing_url.iter().enumerate() {
        header!("Step {} / {}", index + 1, processing_url.len());
        info!("Process: {}", x.url);
        process_part1::start(&x.url, &path.exe_path.parent().unwrap(), &path.tmp_dl, &path.u_block_path, &path.ffmpeg_path, thread, &new_args, ).await?;
    }

    kill_chrome(child)?;
    info!("Global time: {}",utils_data::time_to_human_time(global_time));
    Ok(())
}
