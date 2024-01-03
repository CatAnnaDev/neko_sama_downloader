use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use thirtyfour::{ChromeCapabilities, WebDriver};
use crate::{error, html_parser, info, utils_data, vlc_playlist_builder, warn, web};
use crate::thread_pool::ThreadPool;

pub async fn start(
	url_test: &String,
	exe_path: &Path,
	tmp_dl: &PathBuf,
	chrome: &PathBuf,
	ublock: &PathBuf,
	ffmpeg: &PathBuf,
	mut thread: usize,
) -> Result<(), Box<dyn Error>> {
	let client = Client::builder().build()?;

	let _ = Command::new(chrome)
		.args([
			"--ignore-certificate-errors",
			"--disable-popup-blocking",
			"--disable-logging",
			"--disable-logging-redirect",
			"--port=6969",
		]).stdout(Stdio::null()).spawn()?;

	let before = Instant::now();

	let mut save_path = String::new();

	let base_url = "https://neko-sama.fr";

	let mut prefs = ChromeCapabilities::new();
	prefs
		.add_extension(ublock)
		.expect("can't install ublock origin");


	let driver = WebDriver::new("http://localhost:6969", prefs).await?;
	driver.minimize_window().await?;

	driver
		.set_page_load_timeout(Duration::from_secs(20))
		.await?;

	driver.goto(url_test).await?;

	info!("Scan Main Page");

	let mut episode_url = scan_main_page(&mut save_path, &driver, url_test, base_url, tmp_dl).await?;

	info!("total found: {}", &episode_url.len());

	if &episode_url.len() == &0usize {
		driver.close_window().await?;
		return Ok(());
	}

	info!("Get all .m3u8");
	let (good, error) = get_real_video_link(&mut episode_url, &driver, &client, &tmp_dl).await?;

	if thread > good as usize {
		warn!("update thread count from {thread} to {good}");
		thread = good as usize;
	}

	info!("Start Processing with {} threads", thread);

	let progress_bar = ProgressBar::new(good as u64);
	progress_bar.enable_steady_tick(Duration::from_secs(1));

	progress_bar.set_style(
		ProgressStyle::default_bar()
			.template("[{elapsed_precise}] {bar:60.cyan/blue} {pos}/{len} ({eta})")?
			.progress_chars("$>-"),
	);

	let (tx, rx) = mpsc::channel();

	let mut pool = ThreadPool::new(thread, episode_url.len());

	let mut save_path_vlc = vec![];

	let mut m3u8_path_folder: Vec<_> = fs::read_dir(tmp_dl)?
		.filter_map(|entry| {
			let save = &mut save_path_vlc;

			let entry = entry.ok();
			let file_path = entry?.path();

			if file_path.is_file() {
				let output_path = Path::new(tmp_dl).join(file_path.file_name()?);

				let name = exe_path
					.join(&save_path)
					.join(utils_data::edit_for_windows_compatibility(
						&file_path
							.file_name()
							.unwrap()
							.to_str()
							.unwrap()
							.replace(".m3u8", ".mp4"),
					));

				let _ = &mut save.push((name.clone(), &save_path));

				Some((output_path, name))
			} else {
				None
			}
		})
		.collect();

	utils_data::custom_sort(&mut m3u8_path_folder);

	for (output_path, name) in m3u8_path_folder {
		let tx = tx.clone();
		let ffmpeg = ffmpeg.clone();
		pool.execute(move || {
			tx.send(web::download_build_video(
				&output_path.to_str().unwrap(),
				name.to_str().unwrap(),
				&ffmpeg,
			))
				.unwrap_or(())
		})
	}

	drop(tx);

	for _ in rx.iter().take(episode_url.len()) {
		progress_bar.inc(1);
	}

	progress_bar.finish();
	driver.close_window().await?;
	info!("Clean tmp dir!");
	utils_data::remove_dir_contents(tmp_dl);

	if good >= 2 {
		info!("Build vlc playlist");
		utils_data::custom_sort_vlc(&mut save_path_vlc);
		vlc_playlist_builder::new(save_path_vlc)?;
	}

	let seconds = before.elapsed().as_secs() % 60;
	let minutes = (before.elapsed().as_secs() / 60) % 60;
	let hours = (before.elapsed().as_secs() / 60) / 60;

	let time = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

	info!(
        "Done in: {} for {} episodes and {} error",
        time, good, error
    );

	driver.quit().await?;
	let _ = utils_data::kill_process();
	Ok(())
}



pub async fn scan_main_page(
	save_path: &mut String,
	driver: &WebDriver,
	url_test: &str,
	base_url: &str,
	tmp_dl: &PathBuf,
) -> Result<Vec<(String, String)>, Box<dyn Error>> {
	fs::create_dir_all(tmp_dl)?;

	save_path.push_str(&utils_data::edit_for_windows_compatibility(
		&driver.title().await?.replace(" - Neko Sama", ""),
	));

	fs::create_dir_all(tmp_dl.parent().unwrap().join(save_path))?;
	Ok(html_parser::recursive_find_url(&driver, url_test, base_url).await?)
}

pub async fn get_real_video_link(
	episode_url: &mut Vec<(String, String)>,
	driver: &WebDriver,
	client: &Client,
	tmp_dl: &PathBuf,
) -> Result<(u16, u16), Box<dyn Error>> {
	let mut nb_found = 0u16;
	let mut nb_error = 0u16;
	for (name, url) in episode_url {
		if url.starts_with("http") {
			driver.goto(&url).await?;

			if let Ok(script) = driver
				.execute(
					r#"jwplayer().play(); let ret = jwplayer().getPlaylistItem(); return ret;"#,
					vec![],
				)
				.await
			{
				info!("Get m3u8 for: {}", name);
				if let Some(url) = script.json()["file"].as_str() {
					html_parser::fetch_url(url, &name.trim().replace(":", ""), &tmp_dl, &client)
						.await?;
					nb_found += 1;
				}
			} else {
				error!("Can't get .m3u8 {name} (probably 404)");
				nb_error += 1;
			}
		} else {
			error!("Error with: {name} url: {url}");
			nb_error += 1;
		}
	}
	println!();
	Ok((nb_found, nb_error))
}