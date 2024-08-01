use std::{error::Error, fs, fs::File, io};
use chromiumoxide::Page;
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::error::CdpError;
use m3u8_rs::Playlist;
use reqwest::StatusCode;
use scraper::{Html, Selector};
use crate::{debug, error, info, MainArg, warn};
use crate::cmd_arg::cmd_line_parser::Args;
use crate::utils::utils_data;
use crate::web_client::web;

pub async fn recursive_find_url(page: &Page, _url_test: &str, main_arg: &MainArg)
                                -> Result<Vec<String>, Box<dyn Error>> {
    let mut all_l = vec![];
    page.goto(NavigateParams::builder().url(_url_test).build().unwrap()).await?.wait_for_navigation().await?;
    // direct url
    if _url_test.contains("/episode/") {
        all_l.push(_url_test.to_string());
        return Ok(all_l);
    }

    all_l.extend(get_all_link_base_href(&page, &main_arg.new_args).await?);

    Ok(all_l)
}


async fn get_all_link_base_href(page: &&Page, args: &Args)
                                -> Result<Vec<String>, Box<dyn Error>> {
    let mut url_found = vec![];
    let content = page.content().await?;
    let document = Html::parse_document(&content);
    let selector = Selector::parse(".ui.button.fluid.small.black.text-left").unwrap();

    for element in document.select(&selector) {
        if let Some(href) = element.value().attr("href") {
            if args.debug {
                debug!("get_all_link_base_href: {href}")
            }
            url_found.push(href.to_owned())
        }
    }

    url_found.sort();

    Ok(url_found)
}

pub async fn enter_iframe_wait_jwplayer(page: &Page, all_l: Vec<String>, main_arg: &MainArg)
                                        -> Result<(usize, usize), Box<dyn Error>> {
    let mut nb_found = 0;
    let mut nb_error = 0;

    for fuse_iframe in all_l {
        page.goto(NavigateParams::builder().url(&fuse_iframe).build().unwrap())
            .await?
            .wait_for_navigation()
            .await?;

        let name = utils_data::edit_for_windows_compatibility(
            &page.get_title().await?.unwrap().replace(" - Neko Sama", ""),
        );

        let iframe_selector = r#"//*[@id="un_episode"]"#;
        let iframes = page.find_xpath(iframe_selector).await?;


        if iframes.attribute("src").await?.unwrap() != "undefined".to_string() {
            match page.goto(NavigateParams::builder().url(&iframes.attribute("src").await?.unwrap()).build().unwrap()).await?.wait_for_navigation().await {
                Ok(e) => {
                    let (found, error) = find_and_get_m3u8(nb_found, nb_error, &e, main_arg, name).await?;
                    nb_found = found;
                    nb_error = error;
                }
                Err(_) => {}
            }
        } else {
            nb_error += 1;
            warn!("ignored 404: {}", page.get_title().await?.unwrap())
        }
    }
    // utils_data::kill_process()?;
    Ok((nb_found, nb_error))
}

async fn extract_m3u8_url(page: &Page) -> chromiumoxide::Result<String> {
    let js_code = r#"
new Promise(async(resolve) => {
    const checkJWPlayer = async() => {
        if (window.jwplayer != null) {
            resolve(jwplayer().getPlaylistItem().file)
        }  else {
            setTimeout(checkJWPlayer, 500);
        }
    };
   return await checkJWPlayer();
});
    "#;

    let evaluate_params = EvaluateParams::builder()
        .expression(js_code)
        .await_promise(true)
        .build()
        .unwrap();
    let result = page.evaluate(evaluate_params).await?;

    if let Some(value) = result.value() {
        let m3u8_url = value.as_str().unwrap();
        Ok(m3u8_url.to_string())
    } else {
        Err(CdpError::NoResponse)
    }
}

async fn find_and_get_m3u8(mut nb_found: usize, mut nb_error: usize, page: &Page, main_arg: &MainArg, name: String)
                           -> Result<(usize, usize), Box<dyn Error>> {
    match extract_m3u8_url(&page).await
    {
        Ok(url) => {
            info!("Get m3u8 for: {}", name);
            download_and_save_m3u8(
                &url,
                &name.trim().replace(":", "").replace(" ", "_"),
                main_arg,
            )
                .await?;

            nb_found += 1;
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
                let _ = fs::create_dir_all( &main_arg.path.m3u8_tmp);
                let mut out =
                    File::create(format!("{}/{file_name}.m3u8", main_arg.path.m3u8_tmp.display()))
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
