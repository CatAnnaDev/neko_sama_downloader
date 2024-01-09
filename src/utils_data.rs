use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use regex::Regex;

pub fn kill_process() -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "windows")]
    let _ = Command::new("taskkill")
        .args(["/t", "/f", "/im", "chromedriver.exe"])
        .stdout(Stdio::null())
        .spawn()?;

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let _ = Command::new("killall")
        .arg("\"chromedriver\"")
        .stdout(Stdio::null())
        .spawn()?;
    Ok(())
}
pub fn time_to_human_time(time: Instant) -> String {
    let seconds = time.elapsed().as_secs() % 60;
    let minutes = (time.elapsed().as_secs() / 60) % 60;
    let hours = (time.elapsed().as_secs() / 60) / 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

pub fn custom_sort(vec: &mut Vec<(PathBuf, PathBuf)>) {
    vec.sort_by(|a, b| {
        let num_a = extract_episode_number(&a.1.to_str().unwrap());
        let num_b = extract_episode_number(&b.1.to_str().unwrap());
        num_a.cmp(&num_b)
    });
}

pub fn custom_sort_vlc(vec: &mut Vec<(PathBuf, &String)>) {
    vec.sort_by(|a, b| {
        let num_a = extract_episode_number(&a.0.to_str().unwrap());
        let num_b = extract_episode_number(&b.0.to_str().unwrap());
        num_a.cmp(&num_b)
    });
}

pub fn extract_episode_number(s: &str) -> i32 {
    s.trim_end_matches(".mp4")
        .split_whitespace()
        .filter_map(|word| word.parse::<i32>().ok())
        .last()
        .unwrap_or(0)
}

pub fn edit_for_windows_compatibility(name: &str) -> String {
    let regex = Regex::new(r#"[\\/?%*:|"<>]+"#).unwrap();
    regex.replace_all(name, "").to_string()
}

pub fn remove_dir_contents<P: AsRef<Path>>(path: P) {
    if let Ok(_) = fs::remove_dir_all(path) {}
}
