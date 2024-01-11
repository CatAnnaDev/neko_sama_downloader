use std::error::Error;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use reqwest::{Client, StatusCode};
use thirtyfour::{By, WebDriver};

use crate::{debug, error, info, utils_data, web};

pub async fn recursive_find_url(
    driver: &WebDriver,
    _url_test: &str,
    base_url: &str,
    debug: &bool,
    client: &Client,
    tmp_dl: &PathBuf
) -> Result<(u16, u16), Box<dyn Error>> {
    let mut all_l = vec![];

    if _url_test.contains("/episode/") {
        driver.goto(_url_test).await?;
        let video_url = get_video_url(&driver, debug, all_l, base_url, client, tmp_dl).await?;
        return Ok(video_url);
    }

    let n = driver.find_all( By::ClassName("animeps-next-page")).await?;

    if n.len() == 0 {
        all_l.extend(get_all_link_base(&driver, debug).await?);
    }

    while n.len() != 0 {
        all_l.extend(get_all_link_base(&driver, debug).await?);
        let n = driver.find_all( By::ClassName("animeps-next-page")).await?;
        if !n
            .first()
            .expect("first")
            .attr("class").await?
            .expect("euh")
            .contains("disabled")
        {
            info!("Next page");
            driver.execute(r#"document.querySelector('.animeps-next-page').click();"#, vec![]).await?;
        } else {
            break;
        }
    }

    let video_url = get_video_url(&driver, debug, all_l, base_url, client, tmp_dl).await?;
    Ok(video_url)
}

pub async fn get_all_link_base(
    driver: &WebDriver,
    debug: &bool,
) -> Result<Vec<String>, Box<dyn Error>> {
    let mut url_found = vec![];
    let mut play_class = driver.find_all( By::ClassName("play")).await?;

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

pub async fn get_video_url(driver: &WebDriver, debug: &bool, all_l: Vec<String>, base_url: &str, client: &Client, tmp_dl: &PathBuf,) -> Result<(u16, u16), Box<dyn Error>> {
    let mut nb_found = 0u16;
    let mut nb_error = 0u16;

    for fuse_iframe in all_l {
        driver.handle.goto(&format!("{base_url}{fuse_iframe}")).await?;

        let name = &utils_data::edit_for_windows_compatibility(&driver.title().await?.replace(" - Neko Sama", ""), );

        let url = driver.handle.find(By::Id("un_episode")).await?;
        match url.handle.clone().enter_frame(0).await{
            Ok(_) => {
                loop {
                    match driver.handle.find( By::Id("main-player")).await{
                        Ok(e) => {
                            if let Ok(a) = e.attr("class").await
                            {
                                if let Some(a) = a{
                                    if a.contains("jwplayer") {
                                        break;
                                    }
                                    continue;
                                }
                                continue;
                            }
                            continue;
                        }
                        Err(_) => { continue; }
                    }
                }

                match driver.handle.execute(r#"return jwplayer().getPlaylistItem();"#, vec![], ).await{
                    Ok(script) =>{
                        info!("Get m3u8 for: {}", name);
                        match script.json()["file"].as_str() {
                            None => {
                                error!("can't exec js for {name}: {:?}", script)
                            }
                            Some(url) => {
                                fetch_url(
                                    url,
                                    &name.trim().replace(":", "").replace(" ", "_"),
                                    &tmp_dl,
                                    &client,
                                    debug,
                                ).await?;

                                nb_found += 1;
                            }
                        }
                    }
                    Err(e) =>{
                        error!("Can't get .m3u8 {name} (probably 404)\n{:?}", e);
                        nb_error += 1;
                    }
                }
            }
            Err(_) => {}
        }
        driver.handle.enter_parent_frame().await?;
    }
    utils_data::kill_process()?;
    Ok((nb_found, nb_error))
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
                    File::create(format!("{}/{file_name}.m3u8", tmp_dl.to_str().unwrap())).expect("failed to create file");
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
