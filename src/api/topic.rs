use axum::{Json, extract::State};

use crate::db::{DbState, OidDec, TopicDoc};

use super::HandleError;

// pub async fn get_topic(
//     State(db_state): State<DbState>,
//     OidDec(oid): OidDec,
// ) -> Result<Json<TopicDoc>, HandleError> {

// }
