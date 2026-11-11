//! The phone/tablet client: a static web app compiled into the binary.
//! No build step, no framework, no CDN — the computer serves it directly.

use super::Shared;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use std::sync::Arc;

const INDEX: &str = include_str!("web/index.html");
const APP_JS: &str = include_str!("web/app.js");
const STYLE: &str = include_str!("web/style.css");
const MANIFEST: &str = include_str!("web/manifest.webmanifest");
const ICON: &str = include_str!("web/icon.svg");

fn asset(content_type: &'static str, body: &'static str) -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, content_type),
            // Always fetch fresh so a rebuilt desktop app never serves a stale client.
            (header::CACHE_CONTROL, "no-store"),
        ],
        body,
    )
}

pub(super) fn routes() -> Router<Arc<Shared>> {
    Router::new()
        .route("/", get(|| async { asset("text/html; charset=utf-8", INDEX) }))
        .route("/app.js", get(|| async { asset("text/javascript; charset=utf-8", APP_JS) }))
        .route("/style.css", get(|| async { asset("text/css; charset=utf-8", STYLE) }))
        .route(
            "/manifest.webmanifest",
            get(|| async { asset("application/manifest+json", MANIFEST) }),
        )
        .route("/icon.svg", get(|| async { asset("image/svg+xml", ICON) }))
}
