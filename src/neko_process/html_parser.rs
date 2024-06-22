use std::{error::Error, fs::File, io};

use m3u8_rs::Playlist;
use reqwest::StatusCode;
use thirtyfour::{By, WebDriver, WebElement};

use crate::{debug, error, info, MainArg, warn};
use crate::cmd_arg::cmd_line_parser::Args;
use crate::utils::utils_data;
use crate::web_client::web;

pub async fn recursive_find_url(driver: &WebDriver, _url_test: &str, main_arg: &MainArg)
                                -> Result<Vec<String>, Box<dyn Error>> {
    let mut all_l = vec![];

    // direct url
    if _url_test.contains("/episode/") {
        driver.goto(_url_test).await?;
        all_l.push(_url_test.to_string());
        return Ok(all_l);
    }

    // check next page
    let n = driver.find_all(By::ClassName("animeps-next-page")).await?;

    // only one page
    if n.len() == 0 {
        all_l.extend(get_all_link_base_href(&driver, &main_arg.new_args).await?);
        return Ok(all_l);
    }

    // iter over all page possible
    let page_return = next_page(&driver, &main_arg.new_args, &n).await?;
    all_l.extend(page_return);
    Ok(all_l)
}

async fn next_page(driver: &WebDriver, args: &Args, n: &Vec<WebElement>)
                   -> Result<Vec<String>, Box<dyn Error>> {
    let mut all_links = vec![];
    while n.len() != 0 {
        all_links.extend(get_all_link_base_href(&driver, args).await?);
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

    Ok(all_links)
}


async fn get_all_link_base_href(driver: &WebDriver, args: &Args)
                                -> Result<Vec<String>, Box<dyn Error>> {
    let mut url_found = vec![];
    let mut play_class = driver.find_all(By::ClassName("play")).await?;

    if play_class.len() == 0 {
        play_class = driver.find_all(By::ClassName("text-left")).await?;
    }

    for x in play_class {
        if let Some(url) = x.attr("href").await? {
            if args.debug {
                debug!("get_all_link_base_href: {url}")
            }
            url_found.push(url)
        }
    }
    Ok(url_found)
}

pub async fn enter_iframe_wait_jwplayer(driver: &WebDriver, all_l: Vec<String>, main_arg: &MainArg)
                                        -> Result<(usize, usize), Box<dyn Error>> {
    let mut nb_found = 0;
    let mut nb_error = 0;

    for fuse_iframe in all_l {
        driver.handle.goto(&fuse_iframe).await?;

        let url = driver.handle.find(By::Id("un_episode")).await?;
        if url.attr("src").await?.unwrap() != "undefined".to_string() {
            // force wait after iframe update jwplayer in html
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
                                    }
                                }
                            }
                            Err(_) => {
                                continue;
                            }
                        }
                    }
                    let (found, error) = find_and_get_m3u8(nb_found, nb_error, &driver, main_arg).await?;
                    nb_found = found;
                    nb_error = error;
                }
                Err(_) => {}
            }
        } else {
            nb_error += 1;
            warn!("ignored 404: {}", driver.title().await?)
        }
        driver.handle.enter_parent_frame().await?;
    }
    // utils_data::kill_process()?;
    Ok((nb_found, nb_error))
}

async fn find_and_get_m3u8(mut nb_found: usize, mut nb_error: usize, driver: &WebDriver, main_arg: &MainArg)
                           -> Result<(usize, usize), Box<dyn Error>> {
    let name = utils_data::edit_for_windows_compatibility(
        &driver.title().await?.replace(" - Neko Sama", ""),
    );
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
                    download_and_save_m3u8(
                        url,
                        &name.trim().replace(":", "").replace(" ", "_"),
                        main_arg,
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

    Ok((nb_found, nb_error))
}

async fn download_and_save_m3u8(url: &str, file_name: &str, main_arg: &MainArg)
                                -> Result<(), Box<dyn Error>> {
    match web::web_request(&main_arg.client, &url).await {
        Ok(body) => match body.status() {
            StatusCode::OK => {
                let await_response = body.text().await?;
                let split = await_response.as_bytes();
                let parsed = m3u8_rs::parse_playlist_res(split).unwrap();

                let good_url = test_resolution(parsed, main_arg).await;

                let mut out =
                    File::create(format!("{}/{file_name}.m3u8", main_arg.path.tmp_dl.to_str().unwrap()))
                        .expect("failed to create file");

                if main_arg.new_args.debug {
                    debug!("create .m3u8 for {}", file_name);
                }

                io::copy(
                    &mut web::web_request(&main_arg.client, &good_url)
                        .await?
                        .text()
                        .await?
                        .as_bytes(),
                    &mut out,
                )
                    .expect("Error copy");

                if main_arg.new_args.debug {
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

async fn test_resolution(parsed: Playlist, main_arg: &MainArg)
                         -> String {
    let mut _good_url = String::new();
    match parsed {
        Playlist::MasterPlaylist(pl) => {
            if main_arg.new_args.debug {
                debug!("MasterPlaylist {:#?}", pl);
            }
            for ele in pl.variants {
                let resolution = ele.resolution.expect("No resolution found").height;
                let test = web::web_request(&main_arg.client, &ele.uri).await;
                match test {
                    Ok(code) => match code.status() {
                        StatusCode::OK => {
                            info!("Download as {}p", resolution);
                            _good_url = ele.uri;
                            if main_arg.new_args.debug {
                                debug!("url .m3u8 {}", _good_url);
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
        Playlist::MediaPlaylist(_) => {}
    }
    _good_url
}
