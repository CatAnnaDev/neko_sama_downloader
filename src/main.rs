#![feature(pattern)]
use std::{
    fs,
    env,
    error::Error,
    path::{Path, PathBuf},
    process::{Command, exit},
    sync::mpsc,
    time::Duration,
    time::Instant,
};
use std::io::{stdin, stdout, Write};
use std::process::Stdio;

use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use reqwest::Client;
use thirtyfour::{common::capabilities::chrome::ChromeCapabilities, WebDriver};

use crate::thread_pool::ThreadPool;

mod html_parser;
mod log_color;
mod static_data;
mod thread_pool;
mod utils_check;
mod vlc_playlist_builder;
mod web;
mod search;

// 120.0.6099.110

// https://googlechromelabs.github.io/chrome-for-testing/known-good-versions-with-downloads.json

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let binding = env::current_exe()?;
    let exe_path = binding.parent().unwrap();

    let ublock_destination = exe_path.join(PathBuf::from("utils/uBlock-Origin.crx"));

    let extract_path = exe_path.join(PathBuf::from("utils/"));
    let tmp_dl = exe_path.join(PathBuf::from("tmp/"));

    remove_dir_contents(&tmp_dl);

    // chrome driver
    #[cfg(target_os = "macos")]
    #[cfg(target_os = "linux")]
        let chrome_path = extract_path.join(PathBuf::from("chromedriver"));
    #[cfg(target_os = "windows")]
        let chrome_path = extract_path.join(PathBuf::from("chromedriver.exe"));

    // ffmpeg
    #[cfg(target_os = "macos")]
    #[cfg(target_os = "linux")]
        let ffmpeg_path = extract_path.join(PathBuf::from("ffmpeg"));
    #[cfg(target_os = "windows")]
        let ffmpeg_path = extract_path.join(PathBuf::from("ffmpeg.exe"));

    // ublock
    let u_block_path = extract_path.join(PathBuf::from("uBlock-Origin.crx"));


    let mut chrome_check = false;
    let mut ffmpeg_check = false;
    let mut ublock_check = false;

    let args: Vec<_> = env::args().collect::<_>();

    let arg_type = args.iter().nth(1).expect("truc");

    let mut processing_url = vec![];
    let mut thread = 0;

    match arg_type.as_str() {
        "search" => {
            let find = search::search_over_json(args.iter().nth(2), args.iter().nth(3)).await?;
            thread = args.iter().nth(4).unwrap_or(&String::from("1")).parse::<usize>().unwrap();
            processing_url.extend(find.clone());

            if find.len() <= 50 {
                for (id, (name, url)) in find.iter().enumerate() {
                    println!("({}): {name}:\n{url}\n", id + 1);
                }
            }else { warn!("more than 50 seasons found") }

            let mut s=String::new();
            if args.iter().nth(2).unwrap() != " " {
                print!("All is good for you to download ({}) seasons ? [Y/n]: ", processing_url.len());
            }else {
                print!("All is good for you to download NekoSama ? ({}) seasons ? [Y/n]: ", processing_url.len());
            }
            let _=stdout().flush();
            stdin().read_line(&mut s).expect("Did not enter a correct string");
            if let Some('\n')=s.chars().next_back() {
                s.pop();
            }
            if let Some('\r')=s.chars().next_back() {
                s.pop();
            }
            if s == "n" {
                exit(0);
            }
        }
        "download" => {
            let url_test = args.iter().nth(2).expect("usage: ./anime_dl \"https://neko-sama.fr/anime/info/5821-sword-art-online_vf\"");
            processing_url.extend(vec![("".to_string(),url_test.to_string())]);
            thread = args.iter().nth(3).unwrap_or(&String::from("1")).parse::<usize>().unwrap();
        }
        "help" => {
            println!(r#"
./anime_dl search "my super anime name" <vf or vostfr> <thread number>
./anime_dl download "https://neko-sama.fr/anime/info/5821-sword-art-online_vf" <thread number>
            "#);
            exit(0);
        }
        _ => {}
    }

    if processing_url.is_empty() {
        println!(r#"
./anime_dl search "my super anime name" <vf or vostfr> <thread number>
./anime_dl download "https://neko-sama.fr/anime/info/5821-sword-art-online_vf" <thread number>
            "#);
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

            #[cfg(target_os = "macos")]
            #[cfg(target_os = "linux")]
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
        for (_, url) in processing_url {
            info!("Process: {url}");
            start(
                &url,
                exe_path,
                &tmp_dl,
                &chrome_path,
                &u_block_path,
                &ffmpeg_path,
                thread,
            ).await?;
        }

    } else if !ffmpeg_check && chrome_check {
        error!(
            "Please download then extract {} ffmpeg here:\n{}",
            ffmpeg_path.display(),
            static_data::FFMPEG_PATH
        );
    } else if !chrome_check && ffmpeg_check {
        error!(
            "Please download chrome wed driver then extract {} in utils folder here:\n{}",
            chrome_path.display(),
            static_data::DRIVER_PATH
        );
    } else {
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

async fn start(
    url_test: &String,
    exe_path: &Path,
    tmp_dl: &PathBuf,
    chrome: &PathBuf,
    ublock: &PathBuf,
    ffmpeg: &PathBuf,
    mut thread: usize,
) -> Result<(), Box<dyn Error>> {
    let client = Client::builder().build()?;

    let _ = Command::new(chrome)
        .args([
            "--ignore-certificate-errors",
            "--disable-popup-blocking",
            "--disable-logging",
            "--disable-logging-redirect",
            "--port=6969",
        ]).stdout(Stdio::null()).spawn()?;

    let before = Instant::now();

    let mut save_path = String::new();

    let base_url = "https://neko-sama.fr";

    let mut prefs = ChromeCapabilities::new();
    prefs
        .add_extension(ublock)
        .expect("can't install ublock origin");

    let driver = WebDriver::new("http://localhost:6969", prefs).await?;
    driver.minimize_window().await?;

    driver
        .set_page_load_timeout(Duration::from_secs(20))
        .await?;

    driver.goto(url_test).await?;

    info!("Scan Main Page");

    let mut episode_url = scan_main_page(&mut save_path, &driver, url_test, base_url, tmp_dl).await?;

    info!("total found: {}", &episode_url.len());

    if &episode_url.len() == &0usize {
        driver.close_window().await?;
        return Ok(());
    }

    info!("Get all .m3u8");
    let (good, error) = get_real_video_link(&mut episode_url, &driver, &client, &tmp_dl).await?;

    if thread > good as usize {
        warn!("update thread count from {thread} to {good}");
        thread = good as usize;
    }

    info!("Start Processing with {} threads", thread);

    let progress_bar = ProgressBar::new(good as u64);
    progress_bar.enable_steady_tick(Duration::from_secs(1));

    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:60.cyan/blue} {pos}/{len} ({eta})")?
            .progress_chars("$>-"),
    );

    let (tx, rx) = mpsc::channel();

    let mut pool = ThreadPool::new(thread, episode_url.len());

    let mut save_path_vlc = vec![];

    let mut m3u8_path_folder: Vec<_> = fs::read_dir(tmp_dl)?
        .filter_map(|entry| {
            let save = &mut save_path_vlc;

            let entry = entry.ok();
            let file_path = entry?.path();

            if file_path.is_file() {
                let output_path = Path::new(tmp_dl).join(file_path.file_name()?);

                let name = exe_path
                    .join(&save_path)
                    .join(edit_for_windows_compatibility(
                        &file_path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .replace(".m3u8", ".mp4"),
                    ));

                let _ = &mut save.push((name.clone(), &save_path));

                Some((output_path, name))
            } else {
                None
            }
        })
        .collect();

    custom_sort(&mut m3u8_path_folder);

    for (output_path, name) in m3u8_path_folder {
        let tx = tx.clone();
        let ffmpeg = ffmpeg.clone();
        pool.execute(move || {
            tx.send(web::download_build_video(
                &output_path.to_str().unwrap(),
                name.to_str().unwrap(),
                &ffmpeg,
            ))
                .unwrap_or(())
        })
    }

    drop(tx);

    for _ in rx.iter().take(episode_url.len()) {
        progress_bar.inc(1);
    }

    progress_bar.finish();
    driver.close_window().await?;
    info!("Clean tmp dir!");
    remove_dir_contents(tmp_dl);

    if good >= 2 {
        info!("Build vlc playlist");
        custom_sort_vlc(&mut save_path_vlc);
        vlc_playlist_builder::new(save_path_vlc)?;
    }

    let seconds = before.elapsed().as_secs() % 60;
    let minutes = (before.elapsed().as_secs() / 60) % 60;
    let hours = (before.elapsed().as_secs() / 60) / 60;

    let time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

    info!(
        "Done in: {} for {} episodes and {} error",
        time, good, error
    );

    Ok(())
}

async fn scan_main_page(
    save_path: &mut String,
    driver: &WebDriver,
    url_test: &str,
    base_url: &str,
    tmp_dl: &PathBuf,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    fs::create_dir_all(tmp_dl)?;

    save_path.push_str(&edit_for_windows_compatibility(
        &driver.title().await?.replace(" - Neko Sama", ""),
    ));

    fs::create_dir_all(tmp_dl.parent().unwrap().join(save_path))?;
    Ok(html_parser::recursive_find_url(&driver, url_test, base_url).await?)
}

async fn get_real_video_link(
    episode_url: &mut Vec<(String, String)>,
    driver: &WebDriver,
    client: &Client,
    tmp_dl: &PathBuf,
) -> Result<(u16, u16), Box<dyn Error>> {
    let mut nb_found = 0u16;
    let mut nb_error = 0u16;
    for (name, url) in episode_url {
        if url.starts_with("http") {
            driver.goto(&url).await?;

            if let Ok(script) = driver
                .execute(
                    r#"jwplayer().play(); let ret = jwplayer().getPlaylistItem(); return ret;"#,
                    vec![],
                )
                .await
            {
                info!("Get m3u8 for: {}", name);
                if let Some(url) = script.json()["file"].as_str() {
                    html_parser::fetch_url(url, &name.trim().replace(":", ""), &tmp_dl, &client)
                        .await?;
                    nb_found += 1;
                }
            } else {
                error!("Can't get .m3u8 {name} (probably 404)");
                nb_error += 1;
            }
        } else {
            error!("Error with: {name} url: {url}");
            nb_error += 1;
        }
    }
    println!();
    Ok((nb_found, nb_error))
}


fn custom_sort(vec: &mut Vec<(PathBuf, PathBuf)>) {
    vec.sort_by(|a, b| {
        let num_a = extract_episode_number(&a.1.to_str().unwrap());
        let num_b = extract_episode_number(&b.1.to_str().unwrap());
        num_a.cmp(&num_b)
    });
}

fn custom_sort_vlc(vec: &mut Vec<(PathBuf, &String)>) {
    vec.sort_by(|a, b| {
        let num_a = extract_episode_number(&a.0.to_str().unwrap());
        let num_b = extract_episode_number(&b.0.to_str().unwrap());
        num_a.cmp(&num_b)
    });
}

fn extract_episode_number(s: &str) -> i32 {
    s.trim_end_matches(".mp4").split_whitespace()
        .filter_map(|word| word.parse::<i32>().ok())
        .last()
        .unwrap_or(0)
}

fn edit_for_windows_compatibility(name: &str) -> String {
    let regex = Regex::new(r#"[\\/?%*:|"<>]+"#).unwrap();
    regex.replace_all(name, "").to_string()
}

fn remove_dir_contents<P: AsRef<Path>>(path: P) {
    if let Ok(_) = fs::remove_dir_all(path){

    }
}
