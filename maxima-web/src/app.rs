//! HTTP router, handlers, compute isolation, and embedded asset serving.

use axum::{
    Router,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderValue, StatusCode, header},
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
        .layer(DefaultBodyLimit::max(max_body))
        .with_state(state)
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

/// Isolated parse endpoint with length check.
async fn parse_isolated(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ParseRequest>,
) -> Result<Json<ParseResponse>, (StatusCode, Json<serde_json::Value>)> {
    if req.expression.len() > state.config.max_expr_len {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "error": format!("Expression length {} exceeds limit of {}", req.expression.len(), state.config.max_expr_len),
                "status": "payload_too_large"
            })),
        ));
    }

    let resp = service::parse(&req);
    Ok(Json(resp))
}

/// Hardened compute endpoint with timeout, concurrency throttle, and panic isolation.
async fn compute_isolated(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ComputeRequest>,
) -> Result<Json<api::ComputeResponse>, (StatusCode, Json<serde_json::Value>)> {
    // 1. Check expression length
    if req.expression.len() > state.config.max_expr_len {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "error": format!("Expression length {} exceeds limit of {}", req.expression.len(), state.config.max_expr_len),
                "status": "payload_too_large"
            })),
        ));
    }

    // 2. Concurrency throttling: acquire owned permit
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

    // 3. Isolated execution via spawn_blocking + catch_unwind with timeout
    if state.config.timeout_ms == 0 {
        return Err((
            StatusCode::REQUEST_TIMEOUT,
            Json(serde_json::json!({
                "error": "Computation timed out after 0ms",
                "status": "timeout"
            })),
        ));
    }

    let req_clone = req.clone();
    let compute_future = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compute(&req_clone)))
    });

    let timeout_duration = Duration::from_millis(state.config.timeout_ms);
    let result = match tokio::time::timeout(timeout_duration, compute_future).await {
        Ok(task_res) => match task_res {
            Ok(unwind_res) => match unwind_res {
                Ok(resp) => resp,
                Err(_panic_err) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({
                            "error": "Engine panic caught during symbolic computation",
                            "status": "engine_panic"
                        })),
                    ));
                }
            },
            Err(join_err) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": format!("Worker task join error: {join_err}"),
                        "status": "internal_error"
                    })),
                ));
            }
        },
        Err(_elapsed) => {
            return Err((
                StatusCode::REQUEST_TIMEOUT,
                Json(serde_json::json!({
                    "error": format!("Computation timed out after {}ms", state.config.timeout_ms),
                    "status": "timeout"
                })),
            ));
        }
    };

    // 4. If status is Error due to invalid parameter / parse failure, return HTTP 400
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
