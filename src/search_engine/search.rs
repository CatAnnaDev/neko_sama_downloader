use std::error::Error;

use reqwest::Client;
use serde_derive::{Deserialize, Serialize};

use crate::{debug, warn};
use crate::web_client::web;

#[derive(Clone, Debug, Default)]
pub struct ProcessingUrl {
    pub name: String,
    pub ep: String,
    pub _description: String,
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
        let cleaned_description = clean_string(&x.others);

        let levenshtein_distance = levenshtein_distance(&cleaned_name, &cleaned_title) as f64;
        let max_length = cleaned_name.len().max(cleaned_title.len()) as f64;
        let levenshtein_similarity = 1.0 - levenshtein_distance / max_length;

        if jaccard_similarity(&cleaned_name, &cleaned_title) > 0.7
            || levenshtein_similarity > 0.7
            || cleaned_title.contains(&cleaned_name)
            || cleaned_description.contains(&cleaned_name)
        {
            let x = ProcessingUrl {
                name: x.title,
                ep: x.nb_eps,
                _description: x.others,
                url: x.url,
                genre: x.genres.join(", "),
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
    }
    Ok(find)
}

fn clean_string(s: &str) -> String {
    s.chars()
        .filter(|&c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn levenshtein_distance(word1: &str, word2: &str) -> usize {
    let w1 = word1.chars().collect::<Vec<_>>();
    let w2 = word2.chars().collect::<Vec<_>>();

    let word1_length = w1.len() + 1;
    let word2_length = w2.len() + 1;

    let mut matrix = vec![vec![0; word1_length]; word2_length];

    for i in 1..word1_length { matrix[0][i] = i; }
    for j in 1..word2_length { matrix[j][0] = j; }

    for j in 1..word2_length {
        for i in 1..word1_length {
            let x: usize = if w1[i-1] == w2[j-1] {
                matrix[j-1][i-1]
            } else {
                1 + std::cmp::min(
                    std::cmp::min(matrix[j][i-1], matrix[j-1][i])
                    , matrix[j-1][i-1])
            };
            matrix[j][i] = x;
        }
    }
    matrix[word2_length-1][word1_length-1]
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
