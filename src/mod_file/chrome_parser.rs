use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromeParse {
    pub timestamp: String,
    pub channels: Channels,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channels {
    #[serde(rename = "Stable")]
    pub stable: Stable,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stable {
    pub channel: String,
    pub version: String,
    pub revision: String,
    pub downloads: Downloads,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Downloads {
    pub chromedriver: Vec<Chromedriver>,
}


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chromedriver {
    pub platform: String,
    pub url: String,
}
