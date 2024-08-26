use std::io::BufRead;
use std::fs::File;
use std::time::Duration;
use indicatif::{MultiProgress, MultiProgressAlignment, ProgressBar, ProgressStyle};
use m3u8_rs::MediaPlaylist;
use std::io::{BufReader, Read};
use std::process::Command;
use std::sync::Arc;

use reqwest::{Client, Response};

pub async fn download_build_video(path: &str, name: &str, mp: &Arc<MultiProgress>) {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let ffmpeg = "ffmpeg";

    #[cfg(target_os = "windows")]
    let ffmpeg = std::env::current_exe().unwrap().parent().unwrap().join("utils/ffmpeg.exe"); // TODO change this

    let mut process = Command::new(ffmpeg).args(&[
        "-protocol_whitelist",
        "file,http,https,tcp,tls,crypto",
        "-i",
        &path,
        "-bsf:a",
        "aac_adtstoasc",
        "-c:v",
        "copy",
        "-c:a",
        "copy",
        &name,
    ])
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();



    let mut file = File::open(path).unwrap();
    let mut bytes: Vec<u8> = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    let parsed = m3u8_rs::parse_media_playlist_res(&bytes).unwrap();
    let size = match parsed {
        MediaPlaylist { segments, .. } => segments.len(),
    };

    mp.set_alignment(MultiProgressAlignment::Bottom);
    let progress_bar = mp.add(ProgressBar::new(size as u64));
    progress_bar.enable_steady_tick(Duration::from_secs(1));
    progress_bar.set_message(name.split("/").last().unwrap().to_string());
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] |{wide_bar:.cyan/blue}| {pos}/{len} ({eta}) ({msg})")
            .unwrap()
            .progress_chars("=> "),
    );

    let s = BufReader::new(process.stderr.take().unwrap());
    let mut lines = s.lines();
    while let Some(Ok(l)) = lines.next() {
        if l.contains(".ts") && l.contains("Opening") && l.contains("https @") {
            progress_bar.inc(1);
        }
    }

    let _ = process.wait().unwrap().success();

    progress_bar.finish();
}

pub async fn web_request(client: &Client, url: &str) -> Result<Response, reqwest::Error> {
    client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Upgrade-Insecure-Requests", 1)
        .send()
        .await
}
