use axum::{
    Extension, Json, RequestPartsExt, debug_handler,
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::Utc;
use email_address::EmailAddress;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
// use tracing::{error, info};

use crate::db::DbState;

use super::HandleError;

pub async fn register(
    State(db_state): State<DbState>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthBody>, HandleError> {
    // Check if the user sent the credentials
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(HandleError::MissingCredentials);
    }
    let email = match payload.email {
        Some(m) => {
            if EmailAddress::is_valid(&m) {
                m
            } else {
                return Err(HandleError::MissingCredentials);
            }
        }
        None => return Err(HandleError::MissingCredentials),
    };

    let useroid = db_state
        .add_user(&payload.username, &email, &payload.password)
        .await
        .map_err(|e| HandleError::ServerError(e.to_string()))?;

    let claims = Claims {
        user: payload.username,
        oid: useroid.to_string(),
        // Mandatory expiry time as UTC timestamp
        exp: Utc::now().timestamp() + 7 * 24 * 3600,
    };
    // Create the authorization token
    let secret = db_state.secret().ok_or(HandleError::ServerError(
        "Secret not found in config".to_string(),
    ))?;
    let keys = Keys::new(secret.as_bytes());
    let token = encode(&Header::default(), &claims, &keys.encoding)
        .map_err(|_| HandleError::ServerError("Token creation failed".to_string()))?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}

#[debug_handler]
pub async fn authorize(
    State(db_state): State<DbState>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthBody>, HandleError> {
    // Check if the user sent the credentials
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(HandleError::MissingCredentials);
    }
    let useroid = db_state
        .auth_user(&payload.username, &payload.password)
        .await
        .map_err(|_| HandleError::WrongCredentials)?;
    // if !valid {
    //     return Err(AuthError::WrongCredentials);
    // }
    let claims = Claims {
        user: payload.username,
        oid: useroid.to_string(),
        // Mandatory expiry time as UTC timestamp
        exp: Utc::now().timestamp() + 7 * 24 * 3600,
    };
    // Create the authorization token
    let secret = db_state.secret().ok_or(HandleError::ServerError(
        "Secret not found in config".to_string(),
    ))?;
    let keys = Keys::new(secret.as_bytes());
    let token = encode(&Header::default(), &claims, &keys.encoding)
        .map_err(|_| HandleError::ServerError("Token creation failed".to_string()))?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}

struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    user: String,
    oid: String,
    exp: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthBody {
    success: bool,
    message: String,
    access_token: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    username: String,
    email: Option<String>,
    password: String,
}

// #[derive(Debug)]
// pub enum AuthError {
//     WrongCredentials,
//     MissingCredentials,
//     TokenCreation,
//     InvalidToken,
//     ServerError(String),
// }

impl Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "User: {}", self.user)
    }
}
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = HandleError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| HandleError::BadRequest("Invalid token".to_string()))?;

        let Extension(db_state) = parts
            .extract::<Extension<DbState>>()
            .await
            .map_err(|_| HandleError::ServerError("Extension rejected".to_string()))?;

        let secret = db_state.secret().ok_or(HandleError::ServerError(
            "Secret not found in config".to_string(),
        ))?;
        let keys = Keys::new(secret.as_bytes());

        // Decode the user data
        let token_data = decode::<Claims>(bearer.token(), &keys.decoding, &Validation::default())
            .map_err(|_| HandleError::BadRequest("Invalid token".to_string()))?;

        Ok(token_data.claims)
    }
}

impl Claims {
    pub fn getuser(&self) -> String {
        self.user.clone()
    }

    pub fn userid(&self) -> Option<ObjectId> {
        ObjectId::from_str(&self.oid).ok()
    }
}

impl AuthBody {
    pub fn new(access_token: String) -> Self {
        Self {
            success: true,
            message: "Token generated".to_string(),
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

// impl IntoResponse for AuthError {
//     fn into_response(self) -> Response {
//         let (status, error_message) = match self {
//             AuthError::WrongCredentials => {
//                 (StatusCode::UNAUTHORIZED, "Wrong credentials".to_string())
//             }
//             AuthError::MissingCredentials => {
//                 (StatusCode::BAD_REQUEST, "Missing credentials".to_string())
//             }
//             AuthError::TokenCreation => (
//                 StatusCode::INTERNAL_SERVER_ERROR,
//                 "Token creation error".to_string(),
//             ),
//             AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token".to_string()),
//             AuthError::ServerError(s) => (StatusCode::INTERNAL_SERVER_ERROR, s),
//         };
//         let body = Json(json!({
//             "success": false,
//             "message": error_message,
//         }));
//         (status, body).into_response()
//     }
// }
