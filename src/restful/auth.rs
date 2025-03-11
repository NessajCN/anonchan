use axum::{
    Extension, Json, RequestPartsExt, debug_handler,
    extract::{FromRequestParts, State},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use mongodb::{
    Collection, IndexModel,
    bson::{Document, doc, oid::ObjectId},
    options::IndexOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::Display;
use tracing::{error, info};

use crate::AppState;

pub async fn register(
    State(app_state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthBody>, AuthError> {
    // Check if the user sent the credentials
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    // Here you can check the user credentials from a database
    let client = app_state.mongo_client.clone();

    // lock config mutex in scope to drop it before await
    let db_name = {
        let config = app_state
            .config
            .lock()
            .map_err(|_| AuthError::SecretNotFound)?;
        config.mongo_db().ok_or(AuthError::DatabaseConnection)?
    };
    let db = client.database(&db_name);
    let coll: Collection<Document> = db.collection("users");
    let user_doc = coll
        .find_one(doc! {"name": payload.username.clone()})
        .await
        .map_err(|_| AuthError::DatabaseConnection)?;

    if let Some(_) = user_doc {
        return Err(AuthError::WrongCredentials);
    }

    let hashed_pwd =
        hash(payload.password, DEFAULT_COST).map_err(|_| AuthError::WrongCredentials)?;
    let opts = IndexOptions::builder().unique(true).build();
    let index = IndexModel::builder()
        .keys(doc! {"name": 1})
        .options(opts)
        .build();
    let _idx = coll
        .create_index(index)
        .await
        .map_err(|_| AuthError::DatabaseConnection)?;

    let res = coll
        .insert_one(doc! {"name": &payload.username, "password": hashed_pwd})
        .await
        .map_err(|_| AuthError::DatabaseConnection)?;

    info!("user added: {:?}", res.inserted_id);

    let claims = Claims {
        user: payload.username,
        // Mandatory expiry time as UTC timestamp
        exp: Utc::now().timestamp() + 7 * 24 * 3600,
    };
    // Create the authorization token
    let config = app_state
        .config
        .lock()
        .map_err(|_| AuthError::SecretNotFound)?;
    let secret = config.get_secret().ok_or(AuthError::SecretNotFound)?;
    let keys = Keys::new(secret.as_bytes());
    let token = encode(&Header::default(), &claims, &keys.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}

#[debug_handler]
pub async fn authorize(
    State(app_state): State<AppState>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<AuthBody>, AuthError> {
    // Check if the user sent the credentials
    if payload.username.is_empty() || payload.password.is_empty() {
        return Err(AuthError::MissingCredentials);
    }
    // Here you can check the user credentials from a database
    let client = app_state.mongo_client.clone();

    // lock config mutex in scope to drop it before await
    let db_name = {
        let config = app_state
            .config
            .lock()
            .map_err(|_| AuthError::SecretNotFound)?;
        config.mongo_db().ok_or(AuthError::DatabaseConnection)?
    };
    let db = client.database(&db_name);
    let coll: Collection<UserDoc> = db.collection("users");
    let user_doc = coll
        .find_one(doc! {"name": payload.username.clone()})
        .await
        .map_err(|_| AuthError::DatabaseConnection)?
        .ok_or(AuthError::WrongCredentials)?;

    info!("user doc: {:?}", user_doc);

    let valid =
        verify(payload.password, &user_doc.password).map_err(|_| AuthError::WrongCredentials)?;

    if !valid {
        return Err(AuthError::WrongCredentials);
    }

    let claims = Claims {
        user: payload.username,
        // Mandatory expiry time as UTC timestamp
        exp: Utc::now().timestamp() + 7 * 24 * 3600,
    };
    // Create the authorization token
    let config = app_state
        .config
        .lock()
        .map_err(|_| AuthError::SecretNotFound)?;
    let secret = config.get_secret().ok_or(AuthError::SecretNotFound)?;
    let keys = Keys::new(secret.as_bytes());
    let token = encode(&Header::default(), &claims, &keys.encoding)
        .map_err(|_| AuthError::TokenCreation)?;

    // Send the authorized token
    Ok(Json(AuthBody::new(token)))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct UserDoc {
    name: String,
    // email: String,
    password: String,
    #[serde(rename = "_id")]
    oid: ObjectId,
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
    exp: i64,
}

#[derive(Debug, Serialize)]
pub struct AuthBody {
    access_token: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    username: String,
    password: String,
}

#[derive(Debug)]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    SecretNotFound,
    DatabaseConnection,
}

impl Display for Claims {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "User: {}", self.user)
    }
}
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| AuthError::InvalidToken)?;

        let Extension(app_state) = parts
            .extract::<Extension<AppState>>()
            .await
            .map_err(|_| AuthError::SecretNotFound)?;

        let config = app_state
            .config
            .lock()
            .map_err(|_| AuthError::SecretNotFound)?;
        let secret = config.get_secret().ok_or(AuthError::SecretNotFound)?;
        let keys = Keys::new(secret.as_bytes());

        // Decode the user data
        let token_data = decode::<Claims>(bearer.token(), &keys.decoding, &Validation::default())
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }
}

impl Claims {
    pub fn getuser(&self) -> String {
        self.user.clone()
    }
}

impl AuthBody {
    pub fn new(access_token: String) -> Self {
        Self {
            access_token,
            token_type: "Bearer".to_string(),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AuthError::WrongCredentials => (StatusCode::UNAUTHORIZED, "Wrong credentials"),
            AuthError::MissingCredentials => (StatusCode::BAD_REQUEST, "Missing credentials"),
            AuthError::TokenCreation => (StatusCode::INTERNAL_SERVER_ERROR, "Token creation error"),
            AuthError::InvalidToken => (StatusCode::BAD_REQUEST, "Invalid token"),
            AuthError::SecretNotFound => (StatusCode::INTERNAL_SERVER_ERROR, "Secret not found"),
            AuthError::DatabaseConnection => (StatusCode::INTERNAL_SERVER_ERROR, "Database error"),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}
