//! Complete HTTPS origin for browser credential entry, including scripts and login.
use axum::{
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
pub(super) fn routes() -> Router {
    Router::new()
        .route(
            "/square",
            get(|| async {
                asset(
                    "text/html; charset=utf-8",
                    include_str!("../../assets/article_square_page.html"),
                )
            }),
        )
        .route(
            "/assets/article_square_page.js",
            get(|| async {
                asset(
                    "application/javascript; charset=utf-8",
                    include_str!("../../assets/article_square_page.js"),
                )
            }),
        )
        .route(
            "/assets/article_square.js",
            get(|| async {
                asset(
                    "application/javascript; charset=utf-8",
                    include_str!("../../assets/article_square.js"),
                )
            }),
        )
        .route(
            "/assets/article_square_media.js",
            get(|| async {
                asset(
                    "application/javascript; charset=utf-8",
                    include_str!("../../assets/article_square_media.js"),
                )
            }),
        )
        .route(
            "/assets/articles.css",
            get(|| async {
                asset(
                    "text/css; charset=utf-8",
                    include_str!("../../assets/articles.css"),
                )
            }),
        )
}
fn asset(kind: &'static str, body: &'static str) -> Response {
    ([("content-type",kind),("cache-control","no-store"),("referrer-policy","no-referrer"),("x-content-type-options","nosniff"),("content-security-policy","default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src data: blob:; media-src blob:; connect-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'")],body).into_response()
}
