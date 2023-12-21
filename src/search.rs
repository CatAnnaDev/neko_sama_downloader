use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::error::Error;
use std::str::pattern::Pattern;
use reqwest::Client;
use crate::web;

pub(crate) async fn search_over_json(name: Option<&String>, lang: Option<&String>) -> Result<Vec<(String, String)>, Box<dyn Error>>{
	let client = Client::builder().build()?;
	let base_url = "https://neko-sama.fr";
	let mut find = vec![];
	if let Some(name) = name{
		let resp = web::web_request(&client, &format!("https://neko-sama.fr/animes-search-{}.json", lang.unwrap_or(&String::from("vf")))).await.unwrap();

		let v: Root = serde_json::from_str(&*resp.text().await.unwrap()).unwrap();
		for x in v {
			if name.to_lowercase().is_contained_in(&x.title.to_lowercase()){
				find.push((x.title, format!("{}{}", base_url, x.url)));
			}
		}
	}
	Ok(find)
}



pub type Root = Vec<Season>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Season {
	pub id: i64,
	pub title: String,
	#[serde(rename = "title_english")]
	pub title_english: Option<String>,
	#[serde(rename = "title_romanji")]
	pub title_romanji: Option<String>,
	#[serde(rename = "title_french")]
	pub title_french: Option<String>,
	pub others: String,
	#[serde(rename = "type")]
	pub type_field: String,
	pub status: String,
	pub popularity: f64,
	pub url: String,
	pub genres: Vec<String>,
	#[serde(rename = "url_image")]
	pub url_image: String,
	pub score: String,
	#[serde(rename = "start_date_year")]
	pub start_date_year: String,
	#[serde(rename = "nb_eps")]
	pub nb_eps: String,
}
