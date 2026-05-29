use axum::Router;
#[cfg(feature = "web-ui")]
use axum::http::StatusCode;
#[cfg(feature = "web-ui")]
use axum::http::header;
#[cfg(feature = "web-ui")]
use axum::response::{IntoResponse, Response};
#[cfg(feature = "web-ui")]
use axum::routing::get;
#[cfg(feature = "web-ui")]
use rust_embed::RustEmbed;

#[cfg(feature = "web-ui")]
#[derive(RustEmbed)]
#[folder = "web/dist/"]
struct WebAsset;

#[cfg(feature = "web-ui")]
pub fn router() -> Router<crate::AppState> {
    use axum::extract::Path;
    Router::new()
        .route("/", get(|| async { serve_asset("index.html") }))
        .route(
            "/{*path}",
            get(|Path(p): Path<String>| async move {
                if WebAsset::get(&p).is_some() {
                    serve_asset(&p)
                } else {
                    serve_asset("index.html")
                }
            }),
        )
}

#[cfg(not(feature = "web-ui"))]
pub fn router() -> Router<crate::AppState> {
    Router::new()
}

#[cfg(feature = "web-ui")]
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
