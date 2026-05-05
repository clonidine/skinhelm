use axum::body::Body;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, LOCATION};
use axum::http::{HeaderValue, Response, StatusCode};

const VIEWER_INDEX: &str = include_str!("../../skinhelm-wasm/index.html");
#[cfg(debug_assertions)]
const VIEWER_BOOTSTRAP: &str = include_str!("../../skinhelm-wasm/bootstrap.debug.js");
#[cfg(not(debug_assertions))]
const VIEWER_BOOTSTRAP: &str = include_str!("../../skinhelm-wasm/bootstrap.js");
const VIEWER_WASM_JS: &str = include_str!("../../skinhelm-wasm/pkg/skinhelm_wasm.js");
const VIEWER_WASM: &[u8] = include_bytes!("../../skinhelm-wasm/pkg/skinhelm_wasm_bg.wasm");

pub async fn viewer_redirect() -> Response<Body> {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::PERMANENT_REDIRECT;
    response
        .headers_mut()
        .insert(LOCATION, HeaderValue::from_static("/viewer/"));
    response
}

pub async fn viewer_index() -> Response<Body> {
    static_response("text/html; charset=utf-8", VIEWER_INDEX.as_bytes())
}

pub async fn viewer_bootstrap() -> Response<Body> {
    static_response(
        "text/javascript; charset=utf-8",
        VIEWER_BOOTSTRAP.as_bytes(),
    )
}

pub async fn viewer_wasm_js() -> Response<Body> {
    static_response("text/javascript; charset=utf-8", VIEWER_WASM_JS.as_bytes())
}

pub async fn viewer_wasm() -> Response<Body> {
    static_response("application/wasm", VIEWER_WASM)
}

fn static_response(content_type: &'static str, body: &'static [u8]) -> Response<Body> {
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}
