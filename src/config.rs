use std::{env, fs};
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::error;
use crate::cmd_arg::cmd_line_parser::Args;
use crate::utils::utils_data;
use crate::utils::utils_data::ask_config;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub language: String,
    pub thread: usize,
    pub save_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self{
            language: "vf".to_string(),
            thread: 1,
            save_path: "".to_string(),
        }
    }
}

impl Config{
    pub(crate) fn load(new_args: &mut Args, tmp_path: &PathBuf, config_path: &PathBuf) -> Result<(), Box<dyn Error>> {
        let config = match File::open(&config_path) {
            Ok(mut file) => {
                let mut tmp = String::new();
                file.read_to_string(&mut tmp)?;
                match serde_json::from_str::<Config>(&tmp) {
                    Ok(e) => e,
                    Err(_) => Self::make_config_file(&tmp_path, &config_path)?,
                }
            }
            Err(_) => Self::make_config_file(&tmp_path, &config_path)?,
        };

        if new_args.language == "vf" && new_args.language != config.language {
            new_args.language = config.language;
        }

        if new_args.thread == 1 && new_args.thread != config.thread {
            new_args.thread = config.thread;
        }

        if new_args.save_path == "~/tmp" && new_args.save_path != config.save_path {
            new_args.save_path = config.save_path;
        }

        Ok(())
    }

    pub(crate) fn make_config_file(tmp_path: &PathBuf, config_path: &PathBuf) -> Result<Config, Box<dyn Error>>{
        let language = ask_config("Language ?", vec!["vf", "vostfr"])?;
        let thread = utils_data::ask_keyword("Nb Worker?")?;
        let save_path = utils_data::ask_keyword("Save Path")?;

        let config = Config {
            language: language.as_list_item()
                .map(|e| e.clone().text)
                .unwrap_or_else(|| String::from("vf")),
            thread: thread.as_string()
                .and_then(|e| e.parse::<usize>().ok())
                .unwrap_or(1),
            save_path: match save_path.as_string() {
                Some(e) => if e.is_empty() { tmp_path.display().to_string() } else { e.to_string() },
                None => tmp_path.display().to_string()
            },
        };

        let json = serde_json::to_string(&config)?;
        if let Ok(mut file) = File::options().truncate(true).read(true).write(true).create(true).open(&config_path) {
            file.write_all(json.as_bytes())?;
        } else {
            error!("Can't create config file at {}", config_path.display());
        }
        Ok(config)
    }

    pub(crate) fn get_config_path() -> Result<PathBuf, Box<dyn Error>> {
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
}