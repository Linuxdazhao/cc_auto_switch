pub mod routes;
pub mod stream;

use crate::AppState;
use axum::Router;

pub fn build_api_app(state: AppState) -> Router {
    Router::new().merge(routes::router()).with_state(state)
}
