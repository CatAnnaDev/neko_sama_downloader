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

pub async fn search_over_json(name: &str, lang: &str, debug: &bool, )
    -> Result<Vec<ProcessingUrl>, Box<dyn Error>> {

    let mut edit_lang = lang.to_lowercase();

    if edit_lang != "vf".to_string() && edit_lang != "vostfr".to_string() {
        warn!("\"{edit_lang}\" doesn't exist, replaced by \"vf\" automatically, use only \"vf\" or \"vostfr\"");
        edit_lang = "vf".to_string();
    }

    let client = Client::builder().build()?;

    let resp = web::web_request(
        &client,
        &format!("https://neko-sama.fr/animes-search-{}.json", edit_lang),
    )
        .await
        .unwrap();

    let response_text = resp.text().await?;
    let parsed_jon = serde_json::from_str::<Root>(&response_text)?;

    let cleaned_name = clean_string(name);


    let mut find = vec![];
    for x in parsed_jon {
        let cleaned_title = clean_string(&x.title);
        let cleaned_description = clean_string(&x.others);

        if is_match(&cleaned_title, &cleaned_name, 0.6, 0.6) || cleaned_description.contains(&cleaned_name)
        {
            let p_url = ProcessingUrl {
                name: x.title,
                ep: x.nb_eps,
                _description: x.others,
                url: x.url,
                genre: x.genres.join(", "),
            };
            if *debug {
                debug!("Search engine {:#?}", p_url);
            }
            find.push(p_url);
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

fn is_match(input: &str, query: &str, levenshtein_threshold: f64, matched_threshold: f64) -> bool {
    let input = input.to_lowercase();
    let query = query.to_lowercase();

    if input.contains(&query) {
        return true;
    }

    let query_words: Vec<&str> = query.split_whitespace().collect();
    let input_words: Vec<&str> = input.split_whitespace().collect();

    let mut matched = 0f64;

    for query_word in &query_words {
        if input_words.iter().any(|&word| levenshtein(word, query_word) <= ((1.0 - levenshtein_threshold) * word.len() as f64) as usize )
        {
            matched+=1.0;
            false;
        }
    }
    if matched != 0.0 {
        println!("{matched} >= {} || {}", matched_threshold * query_words.len() as f64, levenshtein_threshold * query_words.len() as f64);
    }
    matched >= (matched_threshold * query_words.len() as f64)
}

fn levenshtein(word1: &str, word2: &str) -> usize {
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
