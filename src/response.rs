use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub version: Version,
    pub players: Players,
    pub description: Description,
    pub favicon: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Version {
    pub name: String,
    pub protocol: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Players {
    pub max: i64,
    pub online: i64,
    pub sample: Vec<Sample>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sample {
    pub name: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Description {
    String(String),
    Map(HashMap<String, String>),
}