pub mod auth;
pub mod topic;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum HandleError {
    WrongCredentials,
    MissingCredentials,
    BadRequest(String),
    ServerError(String),
}

impl IntoResponse for HandleError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            HandleError::WrongCredentials => {
                (StatusCode::UNAUTHORIZED, "Wrong credentials".to_string())
            }
            HandleError::MissingCredentials => {
                (StatusCode::BAD_REQUEST, "Missing credentials".to_string())
            }
            HandleError::BadRequest(s) => (StatusCode::BAD_REQUEST, s),
            HandleError::ServerError(s) => (StatusCode::INTERNAL_SERVER_ERROR, s),
        };
        let body = Json(json!({
            "success": false,
            "message": error_message,
        }));
        (status, body).into_response()
    }
}
