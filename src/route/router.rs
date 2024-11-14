use crate::route::account;
use crate::GlyphState;
use axum::extract::Request;
use axum::http::header::DATE;
use axum::middleware::Next;
use axum::response::Response;
use axum::{middleware, routing::{post, Router}};
use chrono::Utc;
use std::sync::Arc;

async fn add_headers(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(DATE, Utc::now().to_rfc2822().parse().unwrap());
    response
}

pub fn create_router(shared_state: Arc<GlyphState>) -> Router {
    Router::new()
        .route("/account/api/oauth/token", post(account::auth::oauth))
        .layer(middleware::from_fn(add_headers))
        .with_state(shared_state)
}
