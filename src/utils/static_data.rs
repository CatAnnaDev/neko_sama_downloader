// ublock from github for chrome driver
pub const UBLOCK_PATH: &str =
    "https://github.com/PsykoDev/neko_sama_downloader/raw/main/utils/uBlock-Origin.crx";

// ffmpeg
#[cfg(target_os = "windows")]
pub const FFMPEG_PATH: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "x86_64")]
pub const FFMPEG_PATH: &str = "https://evermeet.cx/ffmpeg/ffmpeg-6.1.zip";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "arm")]
pub const FFMPEG_PATH: &str = "https://evermeet.cx/ffmpeg/ffmpeg-6.1.zip";

#[cfg(target_os = "linux")]
pub const FFMPEG_PATH: &str =
    "static build: https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz\n
    arch linux, other: sudo pacman -S ffmpeg\n
    ubuntu, debian, other: sudo apt install ffmpeg";

pub const HEADER: &str = r#"
  _   _      _                   _ _
 | \ | | ___| | _____         __| | |
 |  \| |/ _ \ |/ / _ \ _____ / _` | |
 | |\  |  __/   < (_) |_____| (_| | |
 |_| \_|\___|_|\_\___/       \__,_|_|
                   by CatAnnaDev ᓚᘏᗢ
"#;
