use crate::db::encode_oid;

use super::DbState;
use bcrypt::{DEFAULT_COST, hash, verify};
use mongodb::{
    Collection, IndexModel,
    bson::{Document, doc, oid::ObjectId},
    options::IndexOptions,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::info;

impl DbState {
    pub async fn add_user(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<ObjectId, Box<dyn Error + Send + Sync>> {
        let db = self.db()?;
        let coll: Collection<Document> = db.collection("users");
        let user_doc = coll.find_one(doc! {"email": &email}).await?;

        if let Some(_) = user_doc {
            return Err("Already existed".into());
        }

        let hashed_pwd = hash(password, DEFAULT_COST)?;
        let opts = IndexOptions::builder().unique(true).build();
        let index1 = IndexModel::builder()
            .keys(doc! {"email": 1})
            .options(opts.clone())
            .build();
        let index2 = IndexModel::builder()
            .keys(doc! {"name": 1})
            .options(opts)
            .build();
        let _idx = coll.create_index(index1).await?;
        let _idx = coll.create_index(index2).await?;

        let res = coll
            .insert_one(doc! {"name": &name, "email": &email, "password": hashed_pwd})
            .await?;

        let useroid = match res.inserted_id.as_object_id() {
            Some(oid) => oid,
            None => return Err("Error parsing user objectid".into()),
        };
        info!(
            "user added: {:?}, encoded: {}",
            useroid,
            encode_oid(useroid)
        );
        Ok(useroid)
    }

    pub async fn auth_user(
        &self,
        name: &str,
        password: &str,
    ) -> Result<ObjectId, Box<dyn Error + Send + Sync>> {
        let db = self.db()?;
        let coll: Collection<UserDoc> = db.collection("users");
        let user_doc = match coll.find_one(doc! {"name": name}).await {
            Ok(d) => match d {
                Some(d) => d,
                None => return Err("No user found".into()),
            },
            Err(_) => return Err("User doc query error".into()),
        };

        info!("user doc: {:?}", user_doc);

        match verify(password, &user_doc.password) {
            Ok(valid) => {
                if valid {
                    Ok(user_doc.oid)
                } else {
                    Err("Password unmatch".into())
                }
            }
            Err(_) => Err("Verification failed".into()),
        }
    }

    pub async fn get_user(&self, uid: ObjectId) -> Result<UserDoc, Box<dyn Error + Send + Sync>> {
        let db = self.db()?;
        let coll: Collection<UserDoc> = db.collection("users");
        match coll.find_one(doc! {"_id": uid}).await {
            Ok(d) => match d {
                Some(d) => Ok(d),
                None => Err("No user found".into()),
            },
            Err(_) => Err("User doc query error".into()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDoc {
    pub name: String,
    pub email: String,
    password: String,
    #[serde(rename = "_id")]
    oid: ObjectId,
}
