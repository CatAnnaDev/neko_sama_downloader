use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::error::Error;
use reqwest::Client;
use crate::web;

pub(crate) async fn search_over_json(name: Option<&String>, lang: Option<&String>) -> Result<Vec<(String, String, String)>, Box<dyn Error>>{
	let client = Client::builder().build()?;
	let base_url = "https://neko-sama.fr";
	let mut find = vec![];
	if let Some(name) = name{
		let resp = web::web_request(&client, &format!("https://neko-sama.fr/animes-search-{}.json", lang.unwrap_or(&String::from("vf")))).await.unwrap();

		let rep = &*resp.text().await?;
		let cleaned_name = clean_string(&name);

		let v: Root = serde_json::from_str(rep)?;
		for x in v {
			let cleaned_title = clean_string(&x.title);

			let levenshtein_distance = strsim::levenshtein(&cleaned_name, &cleaned_title) as f64;
			let max_length = cleaned_name.len().max(cleaned_title.len()) as f64;
			let levenshtein_similarity = 1.0 - levenshtein_distance / max_length;

			if jaccard_similarity(&cleaned_name, &cleaned_title) > 0.8
				|| levenshtein_similarity > 0.8
				|| cleaned_title.contains(&cleaned_name)
			{
				find.push((x.title, x.nb_eps, format!("{}{}", base_url, x.url)));
			}
		}
	}
	Ok(find)
}

fn clean_string(s: &str) -> String {
	s.chars()
		.filter(|&c| c.is_alphanumeric() || c.is_whitespace())
		.collect::<String>()
		.to_lowercase()
}

fn jaccard_similarity(s1: &str, s2: &str) -> f64 {
	let set1: std::collections::HashSet<_> = s1.chars().collect();
	let set2: std::collections::HashSet<_> = s2.chars().collect();
	let intersection_size = set1.intersection(&set2).count() as f64;
	let union_size = set1.union(&set2).count() as f64;
	intersection_size / union_size
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