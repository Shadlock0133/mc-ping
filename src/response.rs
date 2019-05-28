use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{self, Display},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub version: Version,
    pub players: Players,
    pub description: Description,
    pub favicon: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, String>,
}

impl Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Version: {}", self.version.name)?;
        writeln!(f, "Players: {}/{}", self.players.online, self.players.max)?;
        writeln!(f, "Desc: {:?}", self.description)?;
        writeln!(f, "Has Favicon: {}", self.favicon.is_some())?;
        writeln!(f, "Extra: {:?}", self.extra)?;
        Ok(())
    }
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
