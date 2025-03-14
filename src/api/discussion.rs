use axum::{Json, extract::State};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::db::{DbState, OidDec, encode_oid};

use super::HandleError;

pub async fn topic(
    State(db_state): State<DbState>,
    OidDec(tid): OidDec,
) -> Result<Json<TopicPayload>, HandleError> {
    let d = db_state
        .get_topic(tid)
        .await
        .map_err(|err| HandleError::NotFound(format!("Topic not found: {err}")))?;

    let u = db_state
        .get_user(d.author)
        .await
        .map_err(|err| HandleError::NotFound(format!("Author not found: {err}")))?;

    let resp = TopicPayload {
        success: true,
        message: "Topic queried".to_string(),
        author: Author {
            uid: encode_oid(d.author),
            email: u.email,
            name: u.name,
        },
        channel: Channel::new(d.channel),
        title: d.title,
        content: d.content,
        created_at: d.created_at.timestamp_millis(),
    };
    Ok(Json(resp))
}

#[derive(Debug, Serialize, Deserialize)]
struct Author {
    uid: String,
    name: String,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Channel {
    cid: String,
    title: String,
    tags: Vec<String>,
}

impl Channel {
    fn new(cid: ObjectId) -> Self {
        Self {
            cid: encode_oid(cid),
            title: "unset".to_string(),
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicPayload {
    success: bool,
    message: String,
    author: Author,
    channel: Channel,
    title: String,
    content: String,
    created_at: i64,
}
