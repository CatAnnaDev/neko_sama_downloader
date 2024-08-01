use std::{
    error::Error,
};

use crate::search_engine::search::ProcessingUrl;


fn _pick_season_list(
    input: &str,
    processing_url: Vec<ProcessingUrl>,
) -> Result<Vec<ProcessingUrl>, Box<dyn Error>> {
    let numbers: Vec<usize> = input
        .split(|c: char| !c.is_digit(10))
        .filter_map(|s| s.parse().ok())
        .collect();
    Ok(numbers
        .iter()
        .filter_map(|&number| processing_url.get(number - 1).map(|url| url.clone()))
        .collect())
}
