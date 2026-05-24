//! Request ID middleware — ULID injection.
//!
//! Если client передаёт `X-Request-Id`, используем его (валидируем как ULID).
//! Иначе — генерируем новый. Кладём в request extensions для tracing spans.

use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use ax_common::ids::RequestId;
use std::str::FromStr;

const HEADER: &str = "x-request-id";

pub async fn middleware(mut req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| RequestId::from_str(s).ok())
        .unwrap_or_default();

    req.extensions_mut().insert(request_id);

    let mut response = next.run(req).await;

    if let Ok(value) = HeaderValue::from_str(&request_id.to_string()) {
        response.headers_mut().insert(HEADER, value);
    }
    response
        .headers_mut()
        .insert("x-stack", HeaderValue::from_static("ax"));

    response
}
