mod topic;
mod user;

use crate::{api::HandleError, config::Config};
use axum::{
    RequestPartsExt,
    extract::{FromRequestParts, Path},
    http::request::Parts,
};
use base64::{Engine, prelude::BASE64_URL_SAFE};
use mongodb::{Client, Database, bson::oid::ObjectId};
use std::error::Error;

#[derive(Debug, Clone, Default)]
pub struct OidDec(pub ObjectId);

impl<S> FromRequestParts<S> for OidDec
where
    S: Send + Sync,
{
    type Rejection = HandleError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let Path(path) = parts
            .extract::<Path<String>>()
            .await
            .map_err(|_| HandleError::NotFound("Invalid path".to_string()))?;

        let oid = decode_oid(&path).ok_or(HandleError::NotFound("Invalid path".to_string()))?;
        Ok(OidDec(oid))
    }
}

pub fn encode_oid(oid: ObjectId) -> String {
    BASE64_URL_SAFE.encode(oid.bytes())
}

fn decode_oid<T: AsRef<[u8]>>(enc: T) -> Option<ObjectId> {
    match BASE64_URL_SAFE.decode(enc) {
        Ok(d) => {
            let arr: [u8; 12] = match d.try_into() {
                Ok(a) => a,
                Err(_) => return None,
            };
            Some(ObjectId::from_bytes(arr))
        }
        Err(_) => None,
    }
}

// Our shared state
#[derive(Clone)]
pub struct DbState {
    config: Config,
    mongo_client: Client,
}

impl DbState {
    pub fn new(config: Config, mongo_client: Client) -> Self {
        Self {
            config,
            mongo_client,
        }
    }

    pub fn secret(&self) -> Option<String> {
        self.config.get_secret()
    }

    pub fn db(&self) -> Result<Database, Box<dyn Error + Send + Sync>> {
        let db_name = match self.config.mongo_db() {
            Some(n) => n,
            None => return Err("DB name not found in config".into()),
        };
        Ok(self.mongo_client.database(&db_name))
    }
}
