#![feature(pattern)]
use std::{
    fs,
    env,
    error::Error,
    path::PathBuf,
    process::exit,
    time::Instant,
};
use std::io::{stdin, stdout, Write};
use clap::Parser;
use crate::search::ProcessingUrl;


mod html_parser;
mod log_color;
mod static_data;
mod thread_pool;
mod utils_check;
mod vlc_playlist_builder;
mod web;
mod search;
mod cmd_line_parser;
mod utils_data;
mod process_part1;

// 120.0.6099.110

// https://googlechromelabs.github.io/chrome-for-testing/known-good-versions-with-downloads.json



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let binding = env::current_exe()?;
    let exe_path = binding.parent().unwrap();

    let ublock_destination = exe_path.join(PathBuf::from("utils/uBlock-Origin.crx"));

    let extract_path = exe_path.join(PathBuf::from("utils/"));
    let tmp_dl = exe_path.join(PathBuf::from("tmp/"));

    utils_data::remove_dir_contents(&tmp_dl);

    // chrome driver
    #[cfg(target_family = "unix")]
        let chrome_path = extract_path.join(PathBuf::from("chromedriver"));
    #[cfg(target_os = "windows")]
        let chrome_path = extract_path.join(PathBuf::from("chromedriver.exe"));

    // ffmpeg
    #[cfg(target_family = "unix")]
        let ffmpeg_path = extract_path.join(PathBuf::from("ffmpeg"));
    #[cfg(target_os = "windows")]
        let ffmpeg_path = extract_path.join(PathBuf::from("ffmpeg.exe"));

    // ublock
    let u_block_path = extract_path.join(PathBuf::from("uBlock-Origin.crx"));


    let mut chrome_check = false;
    let mut ffmpeg_check = false;
    let mut ublock_check = false;
    let new_args = cmd_line_parser::Args::parse();

    let mut processing_url = vec![];
    let mut thread = new_args.thread as usize;
    let max_thread = std::thread::available_parallelism()?.get() * 4;

    if thread > max_thread {
        warn!("Max thread for your cpu is between 1 and {}", max_thread);
        thread = max_thread;
    }

    match new_args.scan.as_str() {
        "search" => {

            let find = search::search_over_json(&new_args.url_or_search_word, &new_args.language).await?;
            processing_url.extend(find.clone());

            let mut nb_episodes = 0;
            if find.len() <= 50 {
                for (id, processing_url) in find.iter().enumerate() {
                    dl_ready!("({}): {} ({}):", id + 1, processing_url.name, processing_url.ep);
                    println!("{}\n", processing_url.url);
                    nb_episodes += processing_url.ep.split_whitespace().nth(0).unwrap().parse::<i32>().unwrap_or(1);
                }
            }else {
                for x in find {
                    nb_episodes += x.ep.split_whitespace().nth(0).unwrap().parse::<i32>().unwrap_or(1);
                }
                warn!("more than 50 seasons found")
            }
            let proc_len = processing_url.len();
            let mut s=String::new();
            if proc_len == 0{
                warn!("Noting found retry with another keyword");
                exit(0);
            }
            if new_args.url_or_search_word != " "  {
                print!("All is good for you to download ({}) seasons ? so {} Eps [Y/n] or [1-{}]: ", proc_len, nb_episodes, proc_len);
            }else {
                print!("All is good for you to download NekoSama ? ({}) seasons ? so {} Eps  [Y/n]: ", proc_len, nb_episodes);
            }
            let _=stdout().flush();
            stdin().read_line(&mut s).expect("Did not enter a correct string");
            if let Some('\n')=s.chars().next_back() {
                s.pop();
            }
            if let Some('\r')=s.chars().next_back() {
                s.pop();
            }
            if let Ok(mut pick) = s.parse::<usize>(){
                if pick <= 0{ pick = 0; }
                if pick >= proc_len{ pick = proc_len; }

                let url = processing_url[pick - 1].clone();
                processing_url.clear();
                processing_url.append(&mut vec![url]);
            }
            if s == "n" {
                exit(0);
            }
        }
        "download" => {
            let x = ProcessingUrl{
                name: "".to_string(),
                ep: "".to_string(),
                url: new_args.url_or_search_word,
            };
            processing_url.extend(vec![x]);
        }
        _ => {}
    }


    if processing_url.is_empty() {
        warn!("you can't download 0");
        exit(0);
    }

    fs::create_dir_all(&extract_path)?;

    for entry in fs::read_dir(&extract_path)? {
        if let Ok(x) = entry {
            #[cfg(target_os = "windows")]
            if x.file_name().to_str().unwrap().ends_with(".exe") {
                if x.file_name().to_str().unwrap().contains("chromedriver") {
                    chrome_check = true;
                }
                if x.file_name().to_str().unwrap().contains("ffmpeg") {
                    ffmpeg_check = true;
                }
            }

            #[cfg(target_family = "unix")]
            if x.file_name().to_str().unwrap().ends_with("") {
                if x.file_name().to_str().unwrap().contains("chromedriver") {
                    chrome_check = true;
                }
                if x.file_name().to_str().unwrap().contains("ffmpeg") {
                    ffmpeg_check = true;
                }
            }

            if x.file_name().to_str().unwrap().ends_with(".crx") {
                if x.file_name().to_str().unwrap().contains("uBlock-Origin") {
                    ublock_check = true;
                }
            }
        }
    }
    info!("chromedriver is present\t ? {chrome_check}");
    info!("ffmpeg is present\t ? {ffmpeg_check}");
    info!("uBlock Origin is present ? {ublock_check}");
    if !ublock_check {
        utils_check::download(static_data::UBLOCK_PATH, &ublock_destination)
            .await
            .expect("Erreur lors du téléchargement de uBlock Origin.");
    }
    if ffmpeg_check && chrome_check && ublock_check {

        let global_time = Instant::now();

        for x in processing_url {
            info!("Process: {}", x.url);
            process_part1::start(
                &x.url,
                exe_path,
                &tmp_dl,
                &chrome_path,
                &u_block_path,
                &ffmpeg_path,
                thread,
            ).await?;
        }
        info!("Global time: {}",utils_data::time_to_human_time(global_time));
    }
    else if !ffmpeg_check && chrome_check {
        error!(
            "Please download then extract {} ffmpeg here:\n{}",
            ffmpeg_path.display(),
            static_data::FFMPEG_PATH
        );
    }
    else if !chrome_check && ffmpeg_check {
        error!(
            "Please download chrome wed driver then extract {} in utils folder here:\n{}",
            chrome_path.display(),
            static_data::DRIVER_PATH
        );
    }
    else {
        error!(
            "Please download chrome wed driver then extract {} in utils folder here:\n{}",
            chrome_path.display(),
            static_data::DRIVER_PATH
        );
        println!();
        error!(
            "Please download then extract {} ffmpeg here:\n{}",
            ffmpeg_path.display(),
            static_data::FFMPEG_PATH
        );
    }

    Ok(())
}

