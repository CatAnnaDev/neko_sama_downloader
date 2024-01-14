// chrome driver
#[cfg(target_os = "macos")]
#[cfg(target_arch = "x86_64")]
pub(crate) static DRIVER_PATH: &str =
    "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/mac-x64/chromedriver-mac-x64.zip";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "arm")]
pub(crate) static DRIVER_PATH: &str =
    "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/mac-arm64/chromedriver-mac-arm64.zip";

#[cfg(target_os = "linux")]
pub(crate) static DRIVER_PATH: &str =
    "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/linux64/chromedriver-linux64.zip";

#[cfg(target_os = "windows")]
pub(crate) static DRIVER_PATH: &str =
    "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/win64/chromedriver-win64.zip";

// ublock from github for chrome driver
pub(crate) static UBLOCK_PATH: &str =
    "https://github.com/PsykoDev/neko_sama_downloader/raw/main/utils/uBlock-Origin.crx";

// ffmpeg
#[cfg(target_os = "windows")]
pub(crate) static FFMPEG_PATH: &str =
    "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "x86_64")]
pub(crate) static FFMPEG_PATH: &str = "https://evermeet.cx/ffmpeg/ffmpeg-6.1.zip";

#[cfg(target_os = "macos")]
#[cfg(target_arch = "arm")]
pub(crate) static FFMPEG_PATH: &str = "https://evermeet.cx/ffmpeg/ffmpeg-6.1.zip";

#[cfg(target_os = "linux")]
pub(crate) static FFMPEG_PATH: &str =
    "static build: https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-amd64-static.tar.xz\n
    arch linux, other: sudo pacman -S ffmpeg\n
    ubuntu, debian, other: sudo apt install ffmpeg";


pub(crate) static HEADER: &str =r#"
  _   _      _                   _ _
 | \ | | ___| | _____         __| | |
 |  \| |/ _ \ |/ / _ \ _____ / _` | |
 | |\  |  __/   < (_) |_____| (_| | |
 |_| \_|\___|_|\_\___/       \__,_|_|
                          by PsykoDev
"#;