use std::{error::Error, process::exit, time::Duration};

use reqwest::Client;
use serde_derive::{Deserialize, Serialize};
use tokio::time;

use crate::{debug, mod_file::web, warn};

#[derive(Clone, Debug, Default)]
pub struct ProcessingUrl {
    pub name: String,
    pub ep: String,
    pub url: String,
    pub genre: String,
}

pub async fn search_over_json(
    name: &str,
    lang: &str,
    debug: &bool,
) -> Result<Vec<ProcessingUrl>, Box<dyn Error>> {
    let mut edit_lang = lang.to_lowercase();
    if edit_lang != "vf".to_string() && edit_lang != "vostfr".to_string() {
        warn!("\"{edit_lang}\" doesn't exist, replaced by \"vf\" automatically, use only \"vf\" or \"vostfr\"");
        edit_lang = "vf".to_string();
    }

    let client = Client::builder().build()?;
    let base_url = "https://neko-sama.fr";
    let mut find = vec![];
    let resp = web::web_request(
        &client,
        &format!("{}/animes-search-{}.json", base_url, edit_lang),
    )
        .await
        .unwrap();

    let rep = resp.text().await?;
    let cleaned_name = clean_string(name);

    let v = serde_json::from_str::<Root>(&rep)?;

    for x in v {
        let cleaned_title = clean_string(&x.title);

        let levenshtein_distance = strsim::levenshtein(&cleaned_name, &cleaned_title) as f64;
        let max_length = cleaned_name.len().max(cleaned_title.len()) as f64;
        let levenshtein_similarity = 1.0 - levenshtein_distance / max_length;

        if jaccard_similarity(&cleaned_name, &cleaned_title) > 0.8
            || levenshtein_similarity > 0.8
            || cleaned_title.contains(&cleaned_name)
        {
            let x = ProcessingUrl {
                name: x.title,
                ep: x.nb_eps,
                url: format!("{}{}", base_url, x.url),
                genre: x.genres.join(", ").replace("c0m1dy", "comedy"),
            };
            if *debug {
                debug!("Search engine {:#?}", x);
            }
            find.push(x);
        }
    }
    if find.len() == 0 {
        warn!("Noting found retry with another keyword");
        warn!("Or try with -l vostfr or -l vf (vf is used by default)");
        time::sleep(Duration::from_secs(20)).await;
        exit(130);
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
