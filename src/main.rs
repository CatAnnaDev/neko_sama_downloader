use std::{
    env,
    error::Error,
    path::{Path, PathBuf},
    process::{exit, Command},
    time::Instant,
    {fs, io},
};
use thirtyfour::{common::capabilities::chrome::ChromeCapabilities, WebDriver};

mod html_parser;
mod utils_check;
mod web;

const TMP_DL: &str = "./tmp";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let chrome_url = "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/120.0.6099.71/win64/chromedriver-win64.zip";
    let ffmpeg_url = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-git-essentials.7z";

    let chrome_destination = PathBuf::from("./utils/chrome-win64.zip");
    let ffmpeg_destination = PathBuf::from("./utils/ffmpeg-git-essentials.7z");
    let extract_path = PathBuf::from("./utils");

    let mut chrome_check = false;
    let mut ffmpeg_check = false;

    let url_test = env::args().collect::<Vec<_>>();

    if url_test.len() != 2 {
        println!("usage: ./anime_dl \"https://neko-sama.fr/anime/info/5821-sword-art-online_vf\"");
        //url_test.push("https://neko-sama.fr/anime/info/3458-hagane-no-renkinjutsushi-fullmetal-alchemist_vostfr".to_string());
        exit(0);
    }

    fs::create_dir_all(&extract_path)?;

    loop {
        for entry in fs::read_dir(&extract_path)? {
            if let Ok(x) = entry {
                if x.file_name().to_str().unwrap().ends_with(".exe") {
                    if x.file_name().to_str().unwrap().contains("chromedriver") {
                        chrome_check = true;
                    }
                    if x.file_name().to_str().unwrap().contains("ffmpeg") {
                        ffmpeg_check = true;
                    }
                }
            }
        }

        println!("chromedriver is present ? {chrome_check}");
        println!("ffmpeg is present ? {ffmpeg_check}");

        if ffmpeg_check && chrome_check {
            start(&url_test).await?;
            break;
        } else if !ffmpeg_check && chrome_check {
            utils_check::download_and_extract_archive(
                ffmpeg_url,
                &ffmpeg_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de FFmpeg.");
        } else if !chrome_check && ffmpeg_check {
            utils_check::download_and_extract_archive(
                chrome_url,
                &chrome_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de Chrome.");
        } else {
            utils_check::download_and_extract_archive(
                chrome_url,
                &chrome_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de Chrome.");
            utils_check::download_and_extract_archive(
                ffmpeg_url,
                &ffmpeg_destination,
                &extract_path,
            )
            .await
            .expect("Erreur lors du téléchargement de FFmpeg.");
        }
    }
    Ok(())
}

async fn start(url_test: &Vec<String>) -> Result<(), Box<dyn Error>> {
    let _ = Command::new("./utils/chromedriver.exe")
        .arg("--port=4444")
        .spawn()?;
    let before = Instant::now();
    let mut save_path = String::new();
    let base_url = "https://neko-sama.fr";
    let mut prefs = ChromeCapabilities::new();
    prefs
        .add_extension(Path::new(r#"./utils/uBlock-Origin.crx"#))
        .expect("can't install ublock origin");
    let driver = WebDriver::new("http://localhost:4444", prefs).await?;
    driver.goto(url_test.last().unwrap()).await?;

    println!("Main url");

    save_path.push_str(
        driver
            .title()
            .await?
            .replace(" - Neko Sama", "")
            .replace(":", "")
            .as_str(),
    );
    fs::create_dir_all(save_path.clone())?;
    fs::create_dir_all(TMP_DL)?;
    let episod_url =
        html_parser::recursive_find_url(&driver, url_test.last().unwrap(), base_url).await?;
    println!("dump url\n{:#?}", episod_url);
    println!("total found: {}", episod_url.len());
    for (_name, url) in episod_url.clone() {
        if url.starts_with("http") {
            driver.goto(url).await?;
            let x = driver
                .execute(
                    r#"jwplayer().play(); let ret = jwplayer().getPlaylistItem(); return ret;"#,
                    vec![],
                )
                .await?;
            html_parser::fetch_url(
                x.json()["file"].as_str().unwrap(),
                &_name.trim().replace(":", ""),
            )
            .await?;
        }
    }

    let paths = fs::read_dir(TMP_DL)?;
    let handles: Vec<_> = paths
        .filter_map(|entry| {
            let entry = entry.ok();
            let file_path = entry?.path();
            if file_path.is_file() {
                let output_path = Path::new(TMP_DL).join(file_path.file_name()?);
                println!("Added to process: {:?}", &file_path.file_name().unwrap());
                let name = format!(
                    "./{}/{}.mp4",
                    save_path.clone(),
                    &file_path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace(".m3u8", "")
                );
                Some(std::thread::spawn(move || {
                    web::download_build_video(
                        &output_path.to_str().unwrap().to_string().as_str(),
                        name,
                    )
                }))
            } else {
                None
            }
        })
        .collect();

    for handle in handles {
        handle.join().ok();
    }

    driver.close_window().await?;
    println!("Clean !");
    remove_dir_contents(TMP_DL)?;
    let elapsed = before.elapsed();
    println!(
        "Done in: {}s for {} episodes",
        elapsed.as_secs(),
        episod_url.len()
    );
    Ok(())
}

fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        if let Ok(x) = entry {
            if x.file_name().to_str().unwrap().ends_with(".m3u8") {
                fs::remove_file(x.path())?;
            }
        }
    }
    Ok(())
}
