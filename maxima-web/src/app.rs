//! HTTP router, handlers, compute isolation, and embedded asset serving.

use axum::{
    Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
};
use rust_embed::RustEmbed;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

use crate::api::{
    self, ComputeRequest, HealthResponse, Operation, ParseRequest, ParseResponse, Status,
};
use crate::config::ServerConfig;
use crate::service::{self, compute};

#[derive(RustEmbed)]
#[folder = "static/"]
pub struct StaticAssets;

/// Application state shared across all HTTP request handlers.
#[derive(Clone)]
pub struct AppState {
    pub config: ServerConfig,
    pub semaphore: Arc<Semaphore>,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        let concurrency = config.concurrency.max(1);
        Self {
            config,
            semaphore: Arc::new(Semaphore::new(concurrency)),
        }
    }
}

/// Constructs the full Axum [`Router`] with compute isolation, limits, and embedded assets.
pub fn build_app(state: Arc<AppState>) -> Router {
    let max_body = state.config.max_body_size;

    let api_routes = Router::new()
        .route("/health", get(health))
        .route("/operations", get(operations))
        .route("/rules", get(rules))
        .route("/functions", get(functions))
        .route("/compute", post(compute_isolated))
        .route("/parse", post(parse_isolated))
        .fallback(api_not_found);

    Router::new()
        .nest("/api", api_routes)
        .route("/", get(serve_index))
        .route("/index.html", get(serve_index))
        .route("/static/{*path}", get(serve_static))
        .fallback(serve_index)
        .layer(middleware::from_fn(security_headers))
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state)
}

async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    let headers = res.headers_mut();
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; font-src 'self'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'",
        ),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    res
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        crate_name: "maxima-rs".into(),
    })
}

async fn operations() -> Json<Vec<Operation>> {
    Json(service::operations_catalog())
}

async fn rules() -> Json<api::RulesResponse> {
    Json(service::rules_catalog())
}

async fn functions() -> Json<api::FunctionCatalog> {
    Json(service::functions_catalog())
}

async fn api_not_found() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "API endpoint not found",
            "status": "not_found"
        })),
    )
}

async fn run_isolated<T, F>(
    state: &AppState,
    expr_len: usize,
    work: F,
) -> Result<T, (StatusCode, Json<serde_json::Value>)>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    if expr_len > state.config.max_expr_len {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "error": format!("Expression length {expr_len} exceeds limit of {}", state.config.max_expr_len),
                "status": "payload_too_large"
            })),
        ));
    }

    let permit = match state.semaphore.clone().try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "error": "Server is saturated with concurrent compute requests. Please retry shortly.",
                    "status": "service_unavailable"
                })),
            ));
        }
    };

    if state.config.timeout_ms == 0 {
        return Err((
            StatusCode::REQUEST_TIMEOUT,
            Json(serde_json::json!({
                "error": "Computation timed out after 0ms",
                "status": "timeout"
            })),
        ));
    }

    let cancel_token = maxima_rs::cancel::CancelToken::new();
    let worker_cancel = cancel_token.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();

    let spawn_res = std::thread::Builder::new()
        .name("maxima-cas-worker".to_string())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let _permit = permit;
            let res = maxima_rs::cancel::with_cancel_token(&worker_cancel, || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
            });
            let _ = tx.send(res);
        });

    if let Err(spawn_err) = spawn_res {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Failed to spawn compute worker thread: {spawn_err}"),
                "status": "internal_error"
            })),
        ));
    }

    let timeout_duration = Duration::from_millis(state.config.timeout_ms);
    match tokio::time::timeout(timeout_duration, rx).await {
        Ok(Ok(Ok(val))) => Ok(val),
        Ok(Ok(Err(_panic_err))) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Engine panic caught during symbolic computation",
                "status": "engine_panic"
            })),
        )),
        Ok(Err(recv_err)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": format!("Worker channel error: {recv_err}"),
                "status": "internal_error"
            })),
        )),
        Err(_elapsed) => {
            cancel_token.cancel();
            Err((
                StatusCode::REQUEST_TIMEOUT,
                Json(serde_json::json!({
                    "error": format!("Computation timed out after {}ms", state.config.timeout_ms),
                    "status": "timeout"
                })),
            ))
        }
    }
}

/// Isolated parse endpoint with length check, semaphore, panic catch, and timeout.
async fn parse_isolated(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ParseRequest>,
) -> Result<Json<ParseResponse>, (StatusCode, Json<serde_json::Value>)> {
    let expr_len = req.expression.len();
    let resp = run_isolated(&state, expr_len, move || service::parse(&req)).await?;
    Ok(Json(resp))
}

/// Hardened compute endpoint with timeout, concurrency throttle, and panic isolation.
async fn compute_isolated(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ComputeRequest>,
) -> Result<Json<api::ComputeResponse>, (StatusCode, Json<serde_json::Value>)> {
    let expr_len = req.expression.len();
    let result = run_isolated(&state, expr_len, move || compute(&req)).await?;

    // If status is Error due to invalid parameter / parse failure, return HTTP 400
    if result.status == Status::Error {
        let error_msg = result.error.unwrap_or(result.result);
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": error_msg,
                "error_span": result.error_span,
                "status": "bad_request",
                "warnings": result.warnings
            })),
        ));
    }

    Ok(Json(result))
}

/// Serves embedded `index.html`.
async fn serve_index() -> impl IntoResponse {
    serve_embedded_file("index.html")
}

/// Serves embedded static assets from `static/`.
async fn serve_static(Path(path): Path<String>) -> impl IntoResponse {
    serve_embedded_file(&path)
}

fn serve_embedded_file(path: &str) -> Response {
    let clean_path = path.trim_start_matches('/');
    match StaticAssets::get(clean_path) {
        Some(content) => {
            let mime = mime_guess::from_path(clean_path).first_or_octet_stream();
            let mut res = Response::new(content.data.into());
            *res.status_mut() = StatusCode::OK;
            if let Ok(hv) = HeaderValue::from_str(mime.as_ref()) {
                res.headers_mut().insert(header::CONTENT_TYPE, hv);
            }
            if clean_path.ends_with(".woff2")
                || clean_path.ends_with(".woff")
                || clean_path.ends_with(".ttf")
                || clean_path.starts_with("vendor/fonts/")
                || clean_path.starts_with("fonts/")
            {
                res.headers_mut().insert(
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                );
            }
            res
        }
        None => (
            StatusCode::NOT_FOUND,
            Html("<h1>404 Not Found</h1><p>Asset not found in embedded bundle</p>"),
        )
            .into_response(),
    }
}
