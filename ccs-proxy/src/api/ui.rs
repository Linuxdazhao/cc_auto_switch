use axum::Router;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/"]
struct WebAsset;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(|| async { serve_asset("index.html") }))
        .route("/index.html", get(|| async { serve_asset("index.html") }))
        .route("/app.js", get(|| async { serve_asset("app.js") }))
        .route("/style.css", get(|| async { serve_asset("style.css") }))
}

fn serve_asset(name: &str) -> Response {
    match WebAsset::get(name) {
        Some(asset) => {
            let mime = match std::path::Path::new(name)
                .extension()
                .and_then(|x| x.to_str())
            {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "application/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                _ => "application/octet-stream",
            };
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime)],
                asset.data.into_owned(),
            )
                .into_response()
        }
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
