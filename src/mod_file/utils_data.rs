use std::{env, fs, error::Error, ffi::OsStr, path::{Path, PathBuf}, time::Instant};
use regex::Regex;
use requestty::Answer;
use crate::{mod_file::cmd_line_parser::Args, Scan};

pub fn search_download(new_args: &Args) -> Scan {
    if new_args.url_or_search_word.starts_with("https://neko-sama.fr/") {
        Scan::Download(&new_args.url_or_search_word)
    } else {
        Scan::Search(&new_args.url_or_search_word)
    }
}

pub fn exe_name() -> String {
    env::args().next()
        .as_ref()
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
        .map(String::from).expect("Can't find executable name")
}

pub fn ask_something(question: &str) -> Result<Answer, Box<dyn Error>> {
    let question = requestty::Question::confirm("anonymous")
        .message(question)
        .build();
    Ok(requestty::prompt_one(question)?)
}

pub fn ask_keyword(question: &str) -> Result<Answer, Box<dyn Error>> {
    let question = requestty::Question::input("anonymous")
        .message(question)
        .build();
    Ok(requestty::prompt_one(question)?)
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

pub fn custom_sort_vlc(vec: &mut Vec<(PathBuf, String)>) {
    vec.sort_by(|a, b| {
        let num_a = extract_episode_number(&a.0.to_str().unwrap());
        let num_b = extract_episode_number(&b.0.to_str().unwrap());
        num_a.cmp(&num_b)
    });
}

pub fn extract_episode_number(s: &str) -> i32 {
    s.trim_end_matches(".mp4")
        .split("_")
        .filter_map(|word| word.parse::<i32>().ok())
        .last()
        .unwrap_or(0)
}

pub fn edit_for_windows_compatibility(name: &str) -> String {
    let regex = Regex::new(r#"[\\/?%*:|"<>]+"#).unwrap();
    regex.replace_all(name, "").to_string()
}

#[cfg(target_os = "windows")]
pub fn _path_length_windows(path: &str) -> Result<(), Box<dyn Error>> {
    use crate::{error, info, warn};
    use std::{
        io::{stdin, stdout, Write},
        process::exit,
    };
    if path.len() > 240 {
        let mut s = String::new();
        warn!("Path too long do you want enable long path in windows? [Y/n]: ");
        let _ = stdout().flush();
        stdin()
            .read_line(&mut s)
            .expect("Did not enter a correct string");
        if s.to_lowercase().trim() == "y" {
            use winreg::enums::*;
            use winreg::RegKey;
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            let path = Path::new("SYSTEM\\CurrentControlSet\\Control\\FileSystem")
                .join("LongPathsEnabled");
            let (key, _) = hklm.create_subkey(&path)?;
            key.set_value("LongPathsEnabled", &1u32)?;
            info!("LongPathsEnabled, continue")
        } else {
            error!("Quitting app, path too long");
            exit(130);
        };
    }
    Ok(())
}

pub fn remove_dir_contents<P: AsRef<Path>>(path: P) {
    if let Ok(_) = fs::remove_dir_all(path) {}
}
