use std::error::Error;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use reqwest::Client;
use thirtyfour::{By, WebDriver};

use crate::{error, info, web};

pub async fn recursive_find_url(
    driver: &WebDriver,
    _url_test: &str,
    base_url: &str,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut episode_url = Vec::new();
    let mut all_l = vec![];

    if _url_test.contains("/episode/") {
        driver.goto(_url_test).await?;
        let video_url = get_video_url(&driver).await?;
        episode_url.push((driver.title().await?.replace(" - Neko Sama", ""), video_url));
        return Ok(episode_url);
    }

    let n = driver.find_all(By::ClassName("animeps-next-page")).await?;

    if n.len() == 0 {
        all_l.extend(get_all_link_base(&driver).await?);
    }

    while n.len() != 0 {
        all_l.extend(get_all_link_base(&driver).await?);

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
        let video_url = get_video_url(&driver).await?;
        episode_url.push((driver.title().await?.replace(" - Neko Sama", ""), video_url));
    }
    Ok(episode_url)
}

pub async fn get_all_link_base(driver: &WebDriver) -> Result<Vec<String>, Box<dyn Error>> {
    let mut url_found = vec![];
    let mut play_class = driver.find_all(By::ClassName("play")).await?;

    if play_class.len() == 0 {
        play_class = driver.find_all(By::ClassName("text-left")).await?;
    }

    for x in play_class {
        if let Some(url) = x.attr("href").await? {
            url_found.push(url)
        }
    }
    Ok(url_found)
}

pub async fn get_video_url(driver: &WebDriver) -> Result<String, Box<dyn Error>> {
    let url = driver.find_all(By::Id("un_episode")).await?;
    for x in url {
        let x = x.attr("src").await?;
        if let Some(uri) = x {
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
) -> Result<(), Box<dyn Error>> {
    let body = web::web_request(&client, &url).await?;
    if body.status().is_success() {
        let split = body
            .text()
            .await
            .expect("body invalid")
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let mut out = File::create(format!("{}/{file_name}.m3u8", tmp_dl.to_str().unwrap()))
            .expect("failed to create file");
        let link = &split[2];
        io::copy(
            &mut web::web_request(&client, link)
                .await?
                .text()
                .await?
                .as_bytes(),
            &mut out,
        )
            .expect("Error copy");
    } else {
        error!("Error 404")
    }
    Ok(())
}
