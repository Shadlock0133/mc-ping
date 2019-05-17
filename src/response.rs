use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub version: Version,
    pub players: Players,
    pub description: Description,
    pub favicon: Option<String>,
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
pub struct Description {
    pub text: String,
}
