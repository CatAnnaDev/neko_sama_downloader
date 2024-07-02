use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub language: String,
    pub thread: usize,
    pub save_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self{
            language: "vf".to_string(),
            thread: 1,
            save_path: "".to_string(),
        }
    }
}