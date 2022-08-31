use axum::Router;

pub mod users;

pub fn api_router() -> Router {
    Router::new().merge(users::router())
}
