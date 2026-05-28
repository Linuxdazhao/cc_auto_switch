pub mod routes;
pub mod stream;
pub mod ui;

use crate::AppState;
use axum::Router;

pub fn build_api_app(state: AppState) -> Router {
    Router::new()
        .merge(routes::router())
        .merge(ui::router())
        .with_state(state)
}
