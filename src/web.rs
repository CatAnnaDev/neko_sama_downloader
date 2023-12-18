use std::path::PathBuf;
use reqwest::{Client, Response};
use std::process::Command;

pub fn download_build_video(path: &str, name: String, ffmpeg: &PathBuf) -> i16 {
    let _ = Command::new(ffmpeg)
        .args([
            "-protocol_whitelist",
            "file,http,https,tcp,tls,crypto",
            "-i",
            path,
            "-bsf:a",
            "aac_adtstoasc",
	        "-c",
	        "copy",
            "-vcodec",
            "copy",
            &name,
        ])
	    .output().unwrap();
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
