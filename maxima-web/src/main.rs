//! HTTP backend for the maxima-rs webUI.
//!
//! Serves a single-page frontend and a JSON compute API backed by the
//! `maxima-rs` symbolic math engine.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
};
use std::sync::Arc;
use tower_http::services::ServeDir;

mod api;
mod service;

use api::{ComputeRequest, HealthResponse, Operation};
use service::compute;

#[derive(Clone)]
struct AppState {
    static_dir: String,
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        static_dir: "static".to_string(),
    });

    let api = Router::new()
        .route("/health", get(health))
        .route("/operations", get(operations))
        .route("/rules", get(rules))
        .route("/compute", post(compute_handler))
        .with_state(state.clone());

    let app = Router::new()
        .nest("/api", api)
        .route("/", get(index_html))
        .nest_service("/static", ServeDir::new(&state.static_dir))
        .fallback(get(index_html))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind to 0.0.0.0:3000");
    println!("maxima-web UI listening on http://localhost:3000");
    axum::serve(listener, app).await.expect("server error");
}

async fn index_html() -> Html<String> {
    Html(
        std::fs::read_to_string("static/index.html")
            .expect("static/index.html not found; run from the maxima-web crate dir"),
    )
}

async fn health(State(_): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".into(),
        crate_name: "maxima-rs".into(),
    })
}

async fn operations(State(_): State<Arc<AppState>>) -> Json<Vec<Operation>> {
    Json(vec![
        Operation {
            id: "simplify",
            name: "Simplify / Optimize",
            description: "E-graph equality saturation + cost-optimal extraction",
            needs_variables: false,
            needs_env: false,
        },
        Operation {
            id: "parse_maxima",
            name: "Parse Maxima",
            description: "Parse a Maxima-syntax expression into the symbolic AST",
            needs_variables: false,
            needs_env: false,
        },
        Operation {
            id: "parse_latex",
            name: "Parse LaTeX",
            description: "Parse a LaTeX math formula into the symbolic AST",
            needs_variables: false,
            needs_env: false,
        },
        Operation {
            id: "differentiate",
            name: "Differentiate",
            description: "Symbolic derivative d/dx (supports multiple variables)",
            needs_variables: true,
            needs_env: false,
        },
        Operation {
            id: "gradient",
            name: "Gradient",
            description: "Vector of partial derivatives ∇f",
            needs_variables: true,
            needs_env: false,
        },
        Operation {
            id: "jacobian",
            name: "Jacobian",
            description: "Jacobian matrix of partial derivatives",
            needs_variables: true,
            needs_env: false,
        },
        Operation {
            id: "hessian",
            name: "Hessian",
            description: "Hessian matrix of second-order partial derivatives",
            needs_variables: true,
            needs_env: false,
        },
        Operation {
            id: "cse",
            name: "Common Subexpression Elimination",
            description: "Factor repeated subcomputations into let-bindings",
            needs_variables: false,
            needs_env: false,
        },
        Operation {
            id: "rust_code",
            name: "Generate Rust Code",
            description: "Zero-overhead optimized Rust function (CSE + codegen)",
            needs_variables: true,
            needs_env: false,
        },
        Operation {
            id: "evaluate",
            name: "Evaluate Numeric",
            description: "Substitute variable values and compute a number",
            needs_variables: false,
            needs_env: true,
        },
        Operation {
            id: "provenance",
            name: "Maxima Rule Coverage",
            description: "1:1 Maxima LISP citations and per-file rule counts",
            needs_variables: false,
            needs_env: false,
        },
    ])
}

async fn rules(State(_): State<Arc<AppState>>) -> Json<api::RulesResponse> {
    Json(service::rules_catalog())
}

async fn compute_handler(
    State(_): State<Arc<AppState>>,
    Json(req): Json<ComputeRequest>,
) -> Result<Json<api::ComputeResponse>, StatusCode> {
    let resp = compute(&req);
    Ok(Json(resp))
}

#[allow(dead_code)]
fn _assert_send_sync() {
    fn _requires_send_sync<T: Send + Sync>() {}
    _requires_send_sync::<AppState>();
}
