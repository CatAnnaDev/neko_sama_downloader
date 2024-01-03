use std::error::Error;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use reqwest::{Client, StatusCode};
use thirtyfour::{By, WebDriver};

use crate::{debug, error, info, web};

pub async fn recursive_find_url(
    driver: &WebDriver,
    _url_test: &str,
    base_url: &str,
    debug: &bool,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut episode_url = Vec::new();
    let mut all_l = vec![];

    if _url_test.contains("/episode/") {
        driver.goto(_url_test).await?;
        let video_url = get_video_url(&driver, debug).await?;
        episode_url.push((driver.title().await?.replace(" - Neko Sama", ""), video_url));
        return Ok(episode_url);
    }

    let n = driver.find_all(By::ClassName("animeps-next-page")).await?;

    if n.len() == 0 {
        all_l.extend(get_all_link_base(&driver, debug).await?);
    }

    while n.len() != 0 {
        all_l.extend(get_all_link_base(&driver, debug).await?);

        let n = driver.find_all(By::ClassName("animeps-next-page")).await?;
        if !n
            .first()
            .expect("first")
            .attr("class")
            .await?
            .expect("euh")
            .contains("disabled")
        {
            info!("Next page");
            driver
                .execute(
                    r#"document.querySelector('.animeps-next-page').click();"#,
                    vec![],
                )
                .await?;
        } else {
            break;
        }
    }

    for s in all_l {
        driver.goto(format!("{base_url}{s}")).await?;
        let video_url = get_video_url(&driver, debug).await?;
        episode_url.push((driver.title().await?.replace(" - Neko Sama", ""), video_url));
    }
    Ok(episode_url)
}

pub async fn get_all_link_base(
    driver: &WebDriver,
    debug: &bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut url_found = vec![];
    let mut play_class = driver.find_all(By::ClassName("play")).await?;

    if play_class.len() == 0 {
        play_class = driver.find_all(By::ClassName("text-left")).await?;
    }

    for x in play_class {
        if let Some(url) = x.attr("href").await? {
            if *debug {
                debug!("get_all_link_base: {url}")
            }
            url_found.push(url)
        }
    }
    Ok(url_found)
}

pub async fn get_video_url(driver: &WebDriver, debug: &bool) -> Result<String, Box<dyn Error>> {
    let url = driver.find_all(By::Id("un_episode")).await?;
    for x in url {
        let x = x.attr("src").await?;
        if let Some(uri) = x {
            if *debug {
                debug!("get_video_url: {uri}")
            }
            return Ok(uri);
        }
    }
    Ok(String::from(""))
}

pub async fn fetch_url(
    url: &str,
    file_name: &str,
    tmp_dl: &PathBuf,
    client: &Client,
    debug: &bool,
) -> Result<(), Box<dyn Error>> {
    let body = web::web_request(&client, &url).await;
    match body {
        Ok(body) => match body.status() {
            StatusCode::OK => {
                let split = body
                    .text()
                    .await
                    .expect("body invalid")
                    .lines()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                let mut out =
                    File::create(format!("{}/{file_name}.m3u8", tmp_dl.to_str().unwrap()))
                        .expect("failed to create file");
                if *debug {
                    debug!("create .m3u8 for {}", file_name);
                }
                let link = &split[2];
                if *debug {
                    debug!("url .m3u8 {}", link);
                }
                io::copy(
                    &mut web::web_request(&client, link)
                        .await?
                        .text()
                        .await?
                        .as_bytes(),
                    &mut out,
                )
                .expect("Error copy");
                if *debug {
                    debug!("write .m3u8 for {}", file_name);
                }
            }
            _ => error!("Error not OK: {:?}", body.status()),
        },
        Err(e) => {
            error!("fetch_url: {:?}", e)
        }
    }

    Ok(())
}
