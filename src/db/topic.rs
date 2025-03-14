use mongodb::{
    Collection,
    bson::{DateTime, Document, doc, oid::ObjectId},
};
use serde::{Deserialize, Serialize};
use std::error::Error;

use super::DbState;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TopicDoc {
    #[serde(rename = "_id")]
    oid: ObjectId,
    pub title: String,
    pub author: ObjectId,
    pub channel: ObjectId,
    pub content: String,
    // #[serde(
    //     serialize_with = "serialize_i64_as_bson_datetime",
    //     rename = "createdAt"
    // )]
    #[serde(rename = "createdAt")]
    pub created_at: DateTime,
}

impl DbState {
    pub async fn new_topic(
        &self,
        title: &str,
        author: &ObjectId,
        content: &str,
    ) -> Result<ObjectId, Box<dyn Error + Send + Sync>> {
        let db = self.db()?;
        let coll: Collection<Document> = db.collection("topics");
        let doc = doc! {"title": title, "author": author, "content": content, "createdAt": DateTime::now()};
        let res = coll.insert_one(doc).await?;

        let topicoid = match res.inserted_id.as_object_id() {
            Some(oid) => oid,
            None => return Err("Error parsing topic objectid".into()),
        };
        Ok(topicoid)
    }

    pub async fn delete_topic(&self, tid: ObjectId) -> Result<(), Box<dyn Error + Send + Sync>> {
        let db = self.db()?;
        let coll: Collection<TopicDoc> = db.collection("topics");
        coll.delete_one(doc! {"_id": tid}).await?;
        Ok(())
    }

    pub async fn get_topic(&self, tid: ObjectId) -> Result<TopicDoc, Box<dyn Error + Send + Sync>> {
        let db = self.db()?;
        let coll: Collection<TopicDoc> = db.collection("topics");
        let t_doc = coll.find_one(doc! {"_id": tid}).await?;
        match t_doc {
            Some(d) => Ok(d),
            None => Err("No topic found".into()),
        }
    }
}
