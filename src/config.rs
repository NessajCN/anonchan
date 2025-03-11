use serde::{Deserialize, Serialize};
use std::error::Error;
use tokio::{fs::File, io::AsyncReadExt};
// use tracing::info;

#[derive(Clone, Serialize, Debug, Deserialize)]
struct Auth {
    secret: Option<String>,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
struct Mongo {
    uri: Option<String>,
    db: Option<String>,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
pub struct Config {
    auth: Option<Auth>,
    mongodb: Option<Mongo>,
}

impl Config {
    pub async fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut f = File::open("config.toml").await?;
        let mut buf = String::new();
        f.read_to_string(&mut buf).await?;
        let c = toml::from_str::<Self>(&buf)?;
        Ok(c)
    }

    pub fn get_secret(&self) -> Option<String> {
        match &self.auth {
            Some(a) => match &a.secret {
                Some(secret) => Some(secret.clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn mongo_uri(&self) -> Option<String> {
        match &self.mongodb {
            Some(mongo) => match &mongo.uri {
                Some(u) => Some(u.clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn mongo_db(&self) -> Option<String> {
        match &self.mongodb {
            Some(mongo) => match &mongo.db {
                Some(db) => Some(db.clone()),
                None => None,
            },
            None => None,
        }
    }
}
