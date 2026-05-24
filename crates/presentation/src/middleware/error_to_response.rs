//! Error response middleware — for now a no-op placeholder.
//!
//! `AppError` уже имеет `IntoResponse` impl в `ax-common`. Этот модуль
//! зарезервирован для cross-cutting error handling (Sentry capture, request_id
//! injection в error body) — Phase B.

// Stub. Phase B implementation:
// pub async fn middleware(req: Request, next: Next) -> Response { ... }
