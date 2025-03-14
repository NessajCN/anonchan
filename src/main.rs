mod config;
mod db;
mod api;
mod socketio;

use db::DbState;
use api::auth::{authorize, register};

use axum::routing::{get, post};

use config::Config;
use mongodb::Client;
use socketioxide::SocketIo;
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::info;

use axum::http::{HeaderValue, request::Parts as RequestParts};
use std::error::Error;
use tracing_subscriber::fmt::time::ChronoLocal;

use socketio::{OnlineDevs, OnlineUsers, on_connect};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .compact()
        .with_timer(ChronoLocal::new(String::from("[%F %T]")))
        // .without_time()
        .with_target(false)
        .init();

    let config = Config::new().await?;

    // Init mongodb connection
    let uri = config.mongo_uri().ok_or("mongodb uri not set")?;
    let mongo_client = Client::with_uri_str(uri).await?;
    let db_state = DbState::new(config, mongo_client);

    let (layer, io) = SocketIo::builder()
        .with_state(OnlineDevs::default())
        .with_state(OnlineUsers::default())
        .build_layer();

    io.ns("/", on_connect);

    let app = axum::Router::new()
        .with_state(io)
        .route("/", get(|| async { "Hello, World!" }))
        .route("/auth", post(authorize))
        .route("/reg", post(register))
        .layer(
            ServiceBuilder::new()
                // Enable CORS policy
                .layer(CorsLayer::new().allow_origin(AllowOrigin::predicate(
                    |origin: &HeaderValue, _request_parts: &RequestParts| {
                        origin.as_bytes().ends_with(b".tongjiai.cn")
                            || origin.as_bytes().ends_with(b".gdkit.local")
                    },
                )))
                .layer(layer),
        )
        .with_state(db_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3020").await.unwrap();
    info!("Starting server on 0.0.0.0:3020");

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
