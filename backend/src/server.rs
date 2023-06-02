use std::{net::SocketAddr, path::PathBuf, time::Duration};

use axum::{
    body::{boxed, Full},
    error_handling::HandleErrorLayer,
    extract::{DefaultBodyLimit, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::get,
    BoxError, Json, Router,
};
use rust_embed::RustEmbed;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::{
    cli::{ClientConfig, Config},
    websocket,
};

pub async fn start(config: Config) {
    let app = Router::new()
        .route("/config", get(client_config_handler))
        .route("/ws", get(websocket::handler))
        .fallback(get(static_handler))
        .layer(
            tower::builder::ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_timeout_error))
                .timeout(Duration::from_secs(30))
                .layer(DefaultBodyLimit::max(1024))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new())
                        .on_response(DefaultOnResponse::new().level(Level::TRACE)),
                ),
        )
        .with_state(config.clone());

    let addr = SocketAddr::new(config.address, config.port);
    tracing::info!("Listening on: http://{}", addr);
    tracing::info!(
        "Will run command \"{} {}\" for connected clients",
        config.command,
        config.args.join(" "),
    );
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn client_config_handler(State(config): State<Config>) -> Json<ClientConfig> {
    Json(config.client_config)
}

async fn handle_timeout_error(err: BoxError) -> StatusCode {
    match err.is::<tower::timeout::error::Elapsed>() {
        true => StatusCode::REQUEST_TIMEOUT,
        false => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let mut path = PathBuf::from(uri.path().trim_start_matches("/"));

    if path.file_name() == None {
        path = path.join("index.html");
    }

    match Asset::get(path.to_str().unwrap()) {
        Some(content) => {
            let body = boxed(Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap()
        }
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(boxed(Full::from("Not Found")))
            .unwrap(),
    }
}

#[derive(RustEmbed)]
#[folder = "static/"]
struct Asset;
