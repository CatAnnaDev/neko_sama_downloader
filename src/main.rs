use std::{{fs, io}, collections::HashMap, env, error::Error, fs::File, path::Path, process::Command, time::Instant};
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::process::exit;
use async_recursion::async_recursion;
use reqwest::{Client, Response};
use thirtyfour::{By, WebDriver};
use thirtyfour::common::capabilities::chrome::ChromeCapabilities;

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

    if url_test.len() != 2{
        println!("usage: ./anime_dl \"https://neko-sama.fr/anime/info/5821-sword-art-online_vf\"");
        exit(0);
    }

    fs::create_dir_all(&extract_path)?;

    loop {
        for entry in fs::read_dir(&extract_path)? {
            if let Ok(x) = entry {
                if x.file_name().to_str().unwrap().ends_with(".exe") {
                    if x.file_name().to_str().unwrap().contains("chromedriver"){
                        chrome_check = true;
                    }
                    if x.file_name().to_str().unwrap().contains("ffmpeg"){
                        ffmpeg_check = true;
                    }
                }
            }
        }

        println!("chromedriver is present ? {chrome_check}");
        println!("ffmpeg is present ? {ffmpeg_check}");

        if ffmpeg_check && chrome_check{
            start(&url_test).await?;
            break;
        }else if !ffmpeg_check && chrome_check {
            download_and_extract_archive(ffmpeg_url, &ffmpeg_destination, &extract_path).await.expect("Erreur lors du téléchargement de FFmpeg.");
        }else if !chrome_check && ffmpeg_check {
            download_and_extract_archive(chrome_url, &chrome_destination, &extract_path).await.expect("Erreur lors du téléchargement de Chrome.");
        }else {
            download_and_extract_archive(chrome_url, &chrome_destination, &extract_path).await.expect("Erreur lors du téléchargement de Chrome.");
            download_and_extract_archive(ffmpeg_url, &ffmpeg_destination, &extract_path).await.expect("Erreur lors du téléchargement de FFmpeg.");
        }
    }
    Ok(())
}

async fn download_and_extract_archive(url: &str, destination: &PathBuf, extract_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    println!("Download: {url}");
    let response = Client::new().get(url).send().await?;
    let archive_bytes = response.bytes().await?.to_vec();

    let mut archive_file = File::create(destination)?;
    archive_file.write_all(&archive_bytes)?;


    if url.ends_with(".zip") {
        extract_zip(archive_bytes, extract_path).await?;
    }

    if url.ends_with(".7z") {
        extract_7z(destination, extract_path).await?;
    }
    Ok(())
}

async fn extract_zip(zip_path: Vec<u8>, extract_path: &Path) -> Result<(), Box<dyn Error>> {
    zip_extract::extract(Cursor::new(zip_path), extract_path , true)?;
    Ok(())
}

async fn extract_7z(archive_path: &Path, extract_path: &Path) -> Result<(), Box<dyn Error>> {
    sevenz_rust::decompress_file(archive_path, extract_path).expect("complete");
    for x in fs::read_dir(extract_path)? {
        if let Ok(path) = x{
            if path.path().is_dir(){
                let x = format!("./{}/bin/ffmpeg.exe", path.path().to_str().unwrap());
                fs::rename(x, "./utils/ffmpeg.exe")?;
            }
        }
    }
    Ok(())
}

async fn start(url_test: &Vec<String>) -> Result<(), Box<dyn Error>>{
    let _ = Command::new("./utils/chromedriver.exe").arg("--port=4444").spawn()?;
    let before = Instant::now();
    let mut save_path = String::new();
    let base_url = "https://neko-sama.fr";
    let mut prefs = ChromeCapabilities::new();
    prefs.add_extension(Path::new(r#"./utils/uBlock-Origin.crx"#)).expect("can't install ublock origin");
    let driver = WebDriver::new("http://localhost:4444", prefs).await?;
    driver.goto(url_test.last().unwrap()).await?;

    println!("Main url");

    save_path.push_str(driver.title().await?.replace(" - Neko Sama", "").replace(":", "").as_str());
    fs::create_dir_all(save_path.clone())?;
    fs::create_dir_all(TMP_DL)?;
    let episod_url = recursive_find_url(&driver, url_test.last().unwrap(), base_url).await?;
    println!("dump url\n{:#?}", episod_url);
    println!("total found: {}", episod_url.len());
    for (_name, url) in episod_url.clone() {
        if url.starts_with("http") {
            driver.goto(url).await?;
            let x = driver.execute(r#"jwplayer().play(); let ret = jwplayer().getPlaylistItem(); return ret;"#, vec![]).await?;
            fetch_url(x.json()["file"].as_str().unwrap(), &_name.trim().replace(":", "")).await?;
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
                let name = format!("./{}/{}.mp4", save_path.clone(), &file_path.file_name().unwrap().to_str().unwrap().replace(".m3u8", ""));
                Some(std::thread::spawn(move || download_build_video(&output_path.to_str().unwrap().to_string().as_str(), name)))
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
    println!("Done in: {}s for {} episodes", elapsed.as_secs(), episod_url.len());
    Ok(())
}

fn download_build_video(path: &str, name: String) {
    let _ = Command::new("./utils/ffmpeg")
        .args([
            "-protocol_whitelist",
            "file,http,https,tcp,tls,crypto",
            "-i",
            {path},
            "-acodec",
            "copy",
            "-bsf:a",
            "aac_adtstoasc",
            "-vcodec",
            "copy",
            &name
        ])
        .output()
        .unwrap();

    println!("Done: {}", name)
}

#[async_recursion]
async fn recursive_find_url(driver: &WebDriver, _url_test: &str, base_url: &str) -> Result<HashMap<String, String>, Box<dyn Error>>  {

    let mut episod_url = HashMap::new();

    let all_links = get_all_link_base(&driver).await?;
    for s in all_links {
        driver.goto(format!("{base_url}{s}")).await?;
        let video_url = get_video_url(&driver).await?;
        episod_url.insert(driver.title().await?.replace(" - Neko Sama", ""), video_url);
    }


    // driver.goto(url_test).await?;
    // let n = driver.find_all(By::ClassName("animeps-next-page disabled")).await?;
    // println!("{:?}", n.len());
    // if n.len() == 0 {
    //     if let Some(next) = n.first() {
    //         driver.execute(r#"document.querySelector('.animeps-next-page').click();"#, vec![]).await?;
//
    //         episod_url.extend(recursive_find_url(driver, url_test, base_url).await?);
    //     }
//
    // }

    Ok(episod_url)
}

async fn get_all_link_base(driver: &WebDriver) -> Result<Vec<String>, Box<dyn Error>>{
    let mut url_found = vec![];
    let play_class = driver.find_all(By::ClassName("play")).await?;
    for x in play_class {
        if let Some(url) = x.attr("href").await?{
            url_found.push(url)
        }
    }
    Ok(url_found)
}

async fn get_video_url(driver: &WebDriver) -> Result<String, Box<dyn  Error>>{
    let url = driver.find_all(By::Id("un_episode")).await?;
    for x in url {
        let x = x.attr("src").await?;
        if let Some(uri) = x{
            return Ok(uri);
        }
    }
    Ok(String::from(""))
}


async fn fetch_url(url: &str, file_name: &str) -> Result<(), Box<dyn  Error>> {
    let client = Client::builder().build()?;
    let body = web_request(&client, &url).await?;
    if body.status().is_success() {
        let split = body.text().await.expect("body invalid").lines().map(|s| s.to_string()).collect::<Vec<_>>();
        let mut out = File::create(format!("{TMP_DL}/{file_name}.m3u8")).expect("failed to create file");
        let link = &split[2];
        io::copy(&mut web_request(&client, link).await?.text().await?.as_bytes(), &mut out).expect("Error copy");

    }else { println!("Error 404") }
    Ok(())
}

async fn web_request(client: &Client, url: &str) -> Result<Response, reqwest::Error> {
    client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Upgrade-Insecure-Requests", 1)
        .send()
        .await
}

fn remove_dir_contents<P: AsRef<Path>>(path: P) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        if let Ok(x) = entry{
            if x.file_name().to_str().unwrap().ends_with(".m3u8") {
                fs::remove_file(x.path())?;
            }
        }
    }
    Ok(())
}