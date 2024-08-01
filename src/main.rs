use std::{env, error::Error, fs, str::FromStr, time::Instant};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use chromiumoxide::{Browser, BrowserConfig, Page};
use chromiumoxide::browser::HeadlessMode;
use clap::Parser;
use futures::StreamExt;
use indicatif::MultiProgress;
use requestty::{Answer, OnEsc, prompt_one, Question};
use reqwest::Client;
use tokio::sync::Semaphore;

use crate::cmd_arg::cmd_line_parser;
use crate::cmd_arg::cmd_line_parser::{Args, Scan};
use crate::config::Config;
use crate::neko_process::html_parser::enter_iframe_wait_jwplayer;
use crate::neko_process::process;
use crate::neko_process::process::build_path_to_save_final_video;
use crate::search_engine::search;
use crate::search_engine::search::ProcessingUrl;
use crate::thread::thread_pool;
use crate::utils::{static_data, utils_data};
use crate::utils::utils_data::ask_config;
use crate::utils_data::time_to_human_time;
use crate::web_client::web;

mod neko_process;
mod vlc;
mod utils;
mod thread;
mod cmd_arg;
mod search_engine;
mod web_client;
mod config;

struct AllPath {
    config_path: PathBuf,
    tmp_path: PathBuf,
    m3u8_tmp: PathBuf,
}

pub struct MainArg {
    new_args: Args,
    path: AllPath,
    processing_url: Vec<ProcessingUrl>,
    client: Client,
}

fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut config_dir: PathBuf = Default::default();

    match env::consts::OS {
        "windows" => {
            config_dir = PathBuf::from(env::var("APPDATA").unwrap_or_else(|_| String::from("C:\\Users\\Default\\AppData\\Roaming\\neko_dl")));
        }
        "macos" => {
            config_dir = PathBuf::from(env::var("HOME").unwrap_or_else(|_| String::from("/Users/Default"))).join(".config/neko_dl");
        }
        "linux" => {
            config_dir = PathBuf::from(env::var("HOME").unwrap_or_else(|_| String::from("/home/default"))).join(".config/neko_dl");
        }
        _ => {
            eprintln!("Système d'exploitation non supporté.");
        }
    }

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir)
}

#[tokio::main]
async fn main()
    -> Result<(), Box<dyn Error>> {
    let mut new_args = cmd_line_parser::Args::parse();
    let config_path = get_config_path().unwrap().join("config.json");
    let mut tmp_path = env::temp_dir();

    if let Ok(mut file) = File::create_new(&config_path) {
        let language = ask_config("Language ?", vec!["vf", "vostfr"])?;
        let thread = utils_data::ask_keyword("Nb Worker?")?;
        let save_path = utils_data::ask_keyword("Save Path")?;

        let config = Config {
            language: match language.as_list_item() {
                Some(e) => { e.clone().text }
                None => { String::from("vf") }
            },
            thread: match thread.as_string() {
                Some(e) => {
                    match e.parse::<usize>() {
                        Ok(e) => { e }
                        Err(_) => { 1 }
                    }
                }
                None => { 1 }
            },
            save_path: match save_path.as_string() {
                Some(e) => if e.is_empty() { tmp_path.display().to_string() } else { e.to_string() },
                None => tmp_path.display().to_string()
            },
        };

        let json = serde_json::to_string(&config)?;
        file.write_all(json.as_bytes())?;
    } else if !new_args.ignore_config_file {
        let mut file = File::open(&config_path)?;
        let mut tmp = String::new();
        file.read_to_string(&mut tmp)?;
        let x = serde_json::from_str::<Config>(&tmp)?;

        new_args.thread = x.thread;
        new_args.language = x.language;
        new_args.save_path = x.save_path;
    }

    header!("{}", static_data::HEADER);
    warn!("Please if you got an Error remember to update or download Google chrome");
    let mut processing_url = None;
    while processing_url.is_none() {
        let _ = ask_keyword(&mut new_args);
        processing_url = setup_search_or_download(&mut new_args).await?;
        if processing_url.is_none() {
            new_args.url_or_search_word = Scan::Search("".to_owned())
        }
    }

    thread_pool::max_thread_check(&mut new_args);


    if !new_args.ignore_config_file {
        tmp_path = PathBuf::from(&new_args.save_path);
    }else {
        new_args.save_path = tmp_path.display().to_string()
    }

    let client = Client::builder().build()?;

    info!("{}", new_args);

    let mut arg = MainArg {
        new_args,
        path: AllPath {
            config_path,
            m3u8_tmp: env::temp_dir().join("neko_dl_m3u8/"),
            tmp_path,
        },
        processing_url: processing_url.unwrap(),
        client,
    };

    let _ = iter_over_url_found(&mut arg).await?;

    Ok(())
}

async fn start(url_test: &str, page: &Page, main_arg: &MainArg)
               -> Result<(), Box<dyn Error>> {
    let before = Instant::now();

    let all_url_found = process::scan_main(page, url_test, main_arg).await?;

    let mut save_path = String::new();
    // make final path to save
    build_path_to_save_final_video(&mut save_path, &page, url_test, main_arg).await?;

    // iter overs all urls found
    let (good, error) = enter_iframe_wait_jwplayer(page, all_url_found, main_arg).await?;

    info!("total found: {}", good);

    process::prevent_case_nothing_found_or_error(good, error, main_arg);

    let mut new_thread = main_arg.new_args.thread;
    if new_thread > good {
        warn!("update thread count from {new_thread} to {good}");
        new_thread = good;
    }

    let (mut vec_m3u8_path_folder, vec_save_path_vlc) = process::build_vec_m3u8_folder_path(&main_arg.path, save_path)?;

    utils_data::custom_sort(&mut vec_m3u8_path_folder);

    info!("Start Processing with {} threads", new_thread);


    let semaphore = Arc::new(Semaphore::new(new_thread));

    let mp = Arc::new(MultiProgress::new());

    let handles = vec_m3u8_path_folder.into_iter().map(|(output_path, name)| {
        let mp = Arc::clone(&mp);
        let sema = Arc::clone(&semaphore);
        tokio::spawn(async move {
            let permit = sema.acquire_owned().await.unwrap();

            web::download_build_video(
                &output_path.to_str().unwrap(),
                name.to_str().unwrap(),
                &mp,
            ).await;

            drop(permit);
        })
    }).collect::<Vec<_>>();

    futures::future::join_all(handles).await;

    if good >= 2 && main_arg.new_args.vlc_playlist {
        process::build_vlc_playlist(vec_save_path_vlc)?;
    }
    process::end_print(before, &main_arg.path, good, error);

    Ok(())
}

async fn iter_over_url_found(main_arg: &mut MainArg)
                             -> Result<(), Box<dyn Error>> {
    time_it!("Global time:", {

        if main_arg.new_args.debug {
            debug!("spawn chrome process");
        }

        let config = BrowserConfig::builder().headless_mode(HeadlessMode::New).build().unwrap();
        let (mut browser, mut handler) = Browser::launch(config).await.unwrap();

        let handle = tokio::spawn(async move {
            while let Some(Ok(event)) = handler.next().await {
                println!("Received browser event: {:?}", event);
            }
        });
        let page = browser.new_page("https://neko-sama.fr/").await?;
        for (index, x) in main_arg.processing_url.iter().enumerate() {
            header!("Step {} / {}", index + 1, main_arg.processing_url.len());
            info!("Process: {}", x.url);
            start(&x.url, &page ,&main_arg).await?;
        }

        browser.close().await?;
        handle.await.unwrap();
    });

    Ok(())
}

async fn setup_search_or_download(new_args: &mut Args)
                                  -> Result<Option<Vec<ProcessingUrl>>, Box<dyn Error>> {
    let processing_url = match new_args.url_or_search_word {
        Scan::Search(ref keyword) => {
            match search::search_over_json(&keyword, &new_args.language, &new_args.debug).await {
                Ok(find) => {
                    if find.len() != 0 {
                        build_print_nb_ep_film(&find);
                        let answer = build_question(&find)?;
                        Some(find_real_link_with_answer(&find, answer))
                    } else { None }
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
                _description: "".to_string(),
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
