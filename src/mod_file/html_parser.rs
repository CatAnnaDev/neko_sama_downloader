use std::{error::Error, fs::File, io, path::PathBuf};
use m3u8_rs::Playlist;
use reqwest::{Client, StatusCode};
use thirtyfour::{By, WebDriver};

use crate::{debug, error, info, warn};
use crate::mod_file::{cmd_line_parser::Args, utils_check::AllPath, utils_data, web};

pub async fn recursive_find_url(driver: &WebDriver, _url_test: &str, base_url: &str, args: &Args, client: &Client, path: &AllPath) -> Result<(u16, u16), Box<dyn Error>> {
    let mut all_l = vec![];

    if _url_test.contains("/episode/") {
        driver.goto(_url_test).await?;
        all_l.push(_url_test.replace(base_url, ""));
        let video_url = get_video_url(&driver, args, all_l, base_url, client, path).await?;
        return Ok(video_url);
    }

    let n = driver.find_all(By::ClassName("animeps-next-page")).await?;

    if n.len() == 0 {
        all_l.extend(get_all_link_base(&driver, args).await?);
    }

    while n.len() != 0 {
        all_l.extend(get_all_link_base(&driver, args).await?);
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

    let video_url = get_video_url(&driver, args, all_l, base_url, client, path).await?;
    Ok(video_url)
}

pub async fn get_base_name_direct_url(driver: &WebDriver) -> String {
    let class = driver
        .find(By::XPath(
            r#"//*[@id="watch"]/div/div[4]/div[1]/div/div/h2/a"#,
        ))
        .await
        .expect("Can't get real name direct url");
    let path = class
        .inner_html()
        .await
        .expect("Can't get real name direct innerhtml");
    path
}

pub async fn get_all_link_base(driver: &WebDriver, args: &Args) -> Result<Vec<String>, Box<dyn Error>> {
    let mut url_found = vec![];
    let mut play_class = driver.find_all(By::ClassName("play")).await?;

    if play_class.len() == 0 {
        play_class = driver.find_all(By::ClassName("text-left")).await?;
    }

    for x in play_class {
        if let Some(url) = x.attr("href").await? {
            if args.debug {
                debug!("get_all_link_base: {url}")
            }
            url_found.push(url)
        }
    }
    Ok(url_found)
}

pub async fn get_video_url(driver: &WebDriver, args: &Args, all_l: Vec<String>, base_url: &str, client: &Client, path: &AllPath) -> Result<(u16, u16), Box<dyn Error>> {
    let mut nb_found = 0u16;
    let mut nb_error = 0u16;
    for fuse_iframe in all_l {
        let url = format!("{base_url}{fuse_iframe}");
        driver.handle.goto(&url).await?;

        let name = utils_data::edit_for_windows_compatibility(
            &driver.title().await?.replace(" - Neko Sama", ""),
        );

        let url = driver.handle.find(By::Id("un_episode")).await?;
        match url.handle.clone().enter_frame(0).await {
            Ok(_) => {
                loop {
                    match driver.handle.find(By::Id("main-player")).await {
                        Ok(e) => {
                            if let Ok(a) = e.attr("class").await {
                                if let Some(a) = a {
                                    if a.contains("jwplayer") {
                                        break;
                                    }
                                    continue;
                                }
                                continue;
                            }
                            continue;
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }

                match driver
                    .handle
                    .execute(r#"return jwplayer().getPlaylistItem();"#, vec![])
                    .await
                {
                    Ok(script) => {
                        info!("Get m3u8 for: {}", name);
                        match script.json()["file"].as_str() {
                            None => {
                                error!("can't exec js for {name}: {:?}", script)
                            }
                            Some(url) => {
                                fetch_url(
                                    url,
                                    &name.trim().replace(":", "").replace(" ", "_"),
                                    &path.tmp_dl,
                                    &client,
                                    args,
                                )
                                    .await?;

                                nb_found += 1;
                            }
                        }
                    }
                    Err(e) => {
                        error!("Can't get .m3u8 {name} (probably 404)\n{:?}", e);
                        nb_error += 1;
                    }
                }
            }
            Err(_) => {}
        }
        driver.handle.enter_parent_frame().await?;
    }
    // utils_data::kill_process()?;
    Ok((nb_found, nb_error))
}

pub async fn fetch_url(url: &str, file_name: &str, tmp_dl: &PathBuf, client: &Client, args: &Args) -> Result<(), Box<dyn Error>> {
    let body = web::web_request(&client, &url).await;
    let mut good_url = String::new();
    match body {
        Ok(body) => match body.status() {
            StatusCode::OK => {
                let await_response = body.text().await?;
                let split = await_response.as_bytes();
                let parsed = m3u8_rs::parse_playlist_res(split);
                match parsed {
                    Ok(Playlist::MasterPlaylist(pl)) => {
                        if args.debug {
                            debug!("MasterPlaylist {:#?}", pl);
                        }
                        for ele in pl.variants {
                            let resolution = ele.resolution.expect("No resolution found").height;
                            let test = web::web_request(&client, &ele.uri).await;
                            match test {
                                Ok(code) => match code.status() {
                                    StatusCode::OK => {
                                        info!("Download as {}p", resolution);
                                        good_url = ele.uri;
                                        if args.debug {
                                            debug!("url .m3u8 {}", good_url);
                                        }
                                        break;
                                    }
                                    _ => {
                                        warn!("{}p not found, try next", resolution);
                                    }
                                },
                                Err(e) => error!("m3u8 check resolution error {}", e),
                            }
                        }
                    }
                    Ok(Playlist::MediaPlaylist(_)) => {}
                    Err(e) => println!("Error parse m3u8 : {:?}", e)
                }

                let mut out =
                    File::create(format!("{}/{file_name}.m3u8", tmp_dl.to_str().unwrap()))
                        .expect("failed to create file");

                if args.debug {
                    debug!("create .m3u8 for {}", file_name);
                }

                io::copy(
                    &mut web::web_request(&client, &good_url)
                        .await?
                        .text()
                        .await?
                        .as_bytes(),
                    &mut out,
                )
                    .expect("Error copy");
                if args.debug {
                    debug!("write .m3u8 for {}", file_name);
                }
            }
            _ => error!("Error base url check: {:?}", body.status()),
        },
        Err(e) => {
            error!("fetch_url: {:?}", e)
        }
    }
    Ok(())
}