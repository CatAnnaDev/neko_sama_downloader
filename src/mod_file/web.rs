use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::Instant,
};

use reqwest::{Client, Response};

use crate::{debug, warn};

pub fn download_build_video(path: &str, name: &str, _ffmpeg: &PathBuf, debug: &bool) -> i16 {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let _ffmpeg = "ffmpeg";

    let time = Instant::now();
    let mut process = Command::new(_ffmpeg);
    let args = [
        "-protocol_whitelist",
        "file,http,https,tcp,tls,crypto",
        "-i",
        path,
        "-bsf:a",
        "aac_adtstoasc",
        "-c:v",
        "copy",
        "-c:a",
        "copy",
        &name,
    ];
    if *debug {
        debug!("save path: {} output name: {}", path, name);
        process
            .args(args)
            .stdout(Stdio::piped())
            .spawn()
            .expect("Can't start ffmpeg")
            .wait()
            .expect("Error wait ffmpeg");
    } else {
        process.args(args).output().expect("Can't start ffmpeg");
    }

    let end = time.elapsed().as_secs();

    if end < 1 {
        warn!("Episode {} are skipped or something went wrong, Please check download folder or use -v argument", name.split("/").last().unwrap())
    }

    // thread return 1 via channel to update progress bar
    1
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
