//! Hardening and service isolation integration tests for maxima-web.
//!
//! Validates:
//! - Concurrency throttling and saturation (HTTP 503)
//! - Timeout enforcement (HTTP 408)
//! - Expression length limits and oversized payloads (HTTP 413)
//! - Negative validation and missing parameters (HTTP 400)
//! - Unknown routes (HTTP 404)
//! - Normal execution (HTTP 200)
//! - Embedded static assets (HTML, CSS, JS, fonts) with zero disk dependencies

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use maxima_web::{AppState, ServerConfig, build_app};
use std::sync::Arc;
use tower::ServiceExt;

fn test_state() -> Arc<AppState> {
    let config = ServerConfig {
        timeout_ms: 5000,
        max_expr_len: 10000,
        ..Default::default()
    };
    Arc::new(AppState::new(config))
}

#[tokio::test]
async fn test_static_embedded_assets_served() {
    let app = build_app(test_state());

    // 1. Root index.html
    let res = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let content_type = res
        .headers()
        .get(header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("text/html"));
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8_lossy(&bytes);
    assert!(html.contains("maxima-rs"));
    assert!(html.contains("Offline-First"));

    // 2. CSS bundle
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/vendor/app.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&bytes).contains("--bg-app"));

    // 3. KaTeX CSS & JS
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/vendor/katex/katex.min.css")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/vendor/katex/katex.min.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 4. WOFF2 Font
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/vendor/katex/fonts/KaTeX_Main-Regular.woff2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let font_bytes = res.into_body().collect().await.unwrap().to_bytes();
    assert!(!font_bytes.is_empty());

    // 5. JavaScript Module
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/js/main.js")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    // 6. Non-existent asset -> 404
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/static/missing-file.xyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_status_200_ok() {
    let app = build_app(test_state());

    let payload = serde_json::json!({
        "expression": "sin(x)^2 + cos(x)^2",
        "operation": "simplify",
        "input_format": "maxima"
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["result"], "1");
}

#[tokio::test]
async fn test_api_status_400_bad_request_on_missing_param() {
    let app = build_app(test_state());

    // 1. Missing variable when none detected in constant expression
    let payload_missing_var = serde_json::json!({
        "expression": "1 + 1",
        "operation": "differentiate",
        "input_format": "maxima",
        "params": {}
    });

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload_missing_var.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);

    // 2. Invalid parameter type (string instead of integer for order)
    let payload_invalid_type = serde_json::json!({
        "expression": "sin(x)",
        "operation": "taylor",
        "input_format": "maxima",
        "params": {
            "order": "not_an_integer"
        }
    });

    let res_type = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload_invalid_type.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_type.status(), StatusCode::BAD_REQUEST);

    // 3. Unknown operation
    let payload_unknown = serde_json::json!({
        "expression": "x",
        "operation": "nonexistent_cas_op",
        "input_format": "maxima"
    });

    let res_unknown = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload_unknown.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_unknown.status(), StatusCode::BAD_REQUEST);

    // 4. 3000 parentheses returning HTTP 400 bad request with error and error_span
    let deep_expr = format!("{}x{}", "(".repeat(3000), ")".repeat(3000));
    let payload_deep = serde_json::json!({
        "expression": deep_expr,
        "operation": "simplify",
        "input_format": "maxima"
    });

    let res_deep = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload_deep.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_deep.status(), StatusCode::BAD_REQUEST);
    let bytes = res_deep.into_body().collect().await.unwrap().to_bytes();
    let body_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body_json["status"], "bad_request");
    assert!(
        body_json["error"]
            .as_str()
            .unwrap()
            .contains("Recursion limit exceeded")
    );
    assert!(body_json["error_span"].is_object());
}

#[tokio::test]
async fn test_api_status_404_on_unknown_endpoint() {
    let app = build_app(test_state());

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/nonexistent_endpoint")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_status_413_on_oversized_expression() {
    let config = ServerConfig {
        max_expr_len: 50,
        ..Default::default()
    };
    let app = build_app(Arc::new(AppState::new(config)));

    let long_expr = "x + ".repeat(20) + "1"; // Length > 60 chars
    let payload = serde_json::json!({
        "expression": long_expr,
        "operation": "simplify",
        "input_format": "maxima"
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn test_api_status_408_timeout() {
    let config = ServerConfig {
        timeout_ms: 0,
        ..Default::default()
    };
    let app = build_app(Arc::new(AppState::new(config)));

    let payload = serde_json::json!({
        "expression": "sin(x)^2 + cos(x)^2",
        "operation": "simplify",
        "input_format": "maxima"
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::REQUEST_TIMEOUT);
}

#[tokio::test]
async fn test_api_status_503_concurrency_saturation() {
    let config = ServerConfig {
        concurrency: 1,
        ..Default::default()
    };
    let state = Arc::new(AppState::new(config));

    // Exhaust the single permit manually
    let _permit = state.semaphore.try_acquire().expect("acquire permit");

    let app = build_app(state.clone());

    let payload = serde_json::json!({
        "expression": "x + 1",
        "operation": "simplify",
        "input_format": "maxima"
    });

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_security_headers_present() {
    let app = build_app(test_state());

    for uri in ["/", "/api/health"] {
        let res = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "uri={uri}");
        let headers = res.headers();
        assert!(
            headers.contains_key(header::CONTENT_SECURITY_POLICY),
            "missing CSP on {uri}"
        );
        assert_eq!(
            headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
        assert_eq!(headers.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
    }
}

#[tokio::test]
async fn test_parse_isolated_bounds_and_timeout() {
    // 1. 413 on oversized expression
    let cfg_len = ServerConfig {
        max_expr_len: 10,
        ..Default::default()
    };
    let app_len = build_app(Arc::new(AppState::new(cfg_len)));
    let res_413 = app_len
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/parse")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "x + y + z + w + a + b",
                        "input_format": "maxima"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_413.status(), StatusCode::PAYLOAD_TOO_LARGE);

    // 2. 408 on timeout_ms = 0
    let cfg_to = ServerConfig {
        timeout_ms: 0,
        ..Default::default()
    };
    let app_to = build_app(Arc::new(AppState::new(cfg_to)));
    let res_408 = app_to
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/parse")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "x + 1",
                        "input_format": "maxima"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_408.status(), StatusCode::REQUEST_TIMEOUT);

    // 3. Sorted variables on valid parse
    let app = build_app(test_state());
    let res_ok = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/parse")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "z + a + m",
                        "input_format": "maxima"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_ok.status(), StatusCode::OK);
    let bytes = res_ok.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["variables"], serde_json::json!(["a", "m", "z"]));
}

#[tokio::test]
async fn test_phase46_compute_contract_validation() {
    let app = build_app(test_state());

    // 1. Invalid float_type -> 400
    let res_ft = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "x^2",
                        "operation": "rust_code",
                        "float_type": "f128"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_ft.status(), StatusCode::BAD_REQUEST);

    // 2. Rust keyword injection -> 400
    let res_kw = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "x + 1",
                        "operation": "rust_code",
                        "variables": ["fn"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_kw.status(), StatusCode::BAD_REQUEST);

    // 3. Taylor order > 64 -> 400
    let res_ord = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "sin(x)",
                        "operation": "taylor",
                        "params": { "variable": "x", "order": 65 }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_ord.status(), StatusCode::BAD_REQUEST);

    // 4. Vector [f1, f2] in jacobian -> 200 with 2x2 matrix
    let res_jac = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "[x*y, x^2 + y]",
                        "operation": "jacobian",
                        "variables": ["x", "y"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_jac.status(), StatusCode::OK);
    let jac_bytes = res_jac.into_body().collect().await.unwrap().to_bytes();
    let jac_json: serde_json::Value = serde_json::from_slice(&jac_bytes).unwrap();
    assert_eq!(jac_json["extra"]["matrix"].as_array().unwrap().len(), 2);

    // 5. Auto-dispatch updates resp.operation
    let res_auto = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "integrate(x^2, x)",
                        "operation": "simplify"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_auto.status(), StatusCode::OK);
    let auto_bytes = res_auto.into_body().collect().await.unwrap().to_bytes();
    let auto_json: serde_json::Value = serde_json::from_slice(&auto_bytes).unwrap();
    assert_eq!(auto_json["operation"], "integrate");
}

#[test]
fn test_cli_toml_config_precedence() {
    use maxima_web::config::Cli;
    let unique = format!(
        "maxima_web_test_{}_{}.toml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let path = std::env::temp_dir().join(unique);
    std::fs::write(
        &path,
        "bind = \"0.0.0.0\"\nport = 9090\ntimeout_ms = 1234\nconcurrency = 2\nmax_body_size = 4096\nmax_expr_len = 500\n",
    )
    .unwrap();

    let cli = Cli {
        bind: None,
        port: Some(7777),
        timeout_ms: None,
        concurrency: None,
        max_body_size: None,
        max_expr_len: None,
        config: Some(path.clone()),
    };
    let cfg = ServerConfig::from_cli(&cli).unwrap();
    let _ = std::fs::remove_file(&path);

    assert_eq!(cfg.bind, "0.0.0.0");
    assert_eq!(cfg.port, 7777);
    assert_eq!(cfg.timeout_ms, 1234);
    assert_eq!(cfg.concurrency, 2);
    assert_eq!(cfg.max_body_size, 4096);
    assert_eq!(cfg.max_expr_len, 500);
}

#[tokio::test]
async fn test_phase52_cooperative_timeout_releases_permit_10_of_10() {
    let config = ServerConfig {
        timeout_ms: 150,
        concurrency: 1,
        max_expr_len: 10_000,
        ..Default::default()
    };
    let state = Arc::new(AppState::new(config));
    let app = build_app(state.clone());

    // Heavy CAS computation that exceeds 150ms and polls CancelToken cooperatively
    let heavy = "sum(sin(k*x)^2 + cos(k*x)^2, k, 1, 200)".to_string();

    for run in 0..10 {
        let res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/compute")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "expression": heavy,
                            "operation": "simplify",
                            "fast_math": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        // Either timed out (408) or completed fast (200); if 408, permit must release within 250ms
        assert!(
            res.status() == StatusCode::REQUEST_TIMEOUT || res.status() == StatusCode::OK,
            "run {run}: unexpected status {}",
            res.status()
        );

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(250);
        let mut acquired = false;
        while tokio::time::Instant::now() < deadline {
            if state.semaphore.available_permits() == 1 {
                acquired = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert!(
            acquired,
            "run {run}: semaphore permit not released within 250ms after timeout"
        );

        // Follow-up trivial request must succeed with 200 OK (no 503 leak)
        let ok_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/parse")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "expression": "1 + 1",
                            "input_format": "maxima"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok_res.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn test_phase52_adversarial_inputs_never_panic_or_oom() {
    let app = build_app(test_state());

    // 1. 2^2^... depth 1000
    let pow_tower = vec!["2"; 1000].join("^");
    let res_pow = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": pow_tower,
                        "operation": "simplify"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_pow.status(), StatusCode::BAD_REQUEST);

    // 2. 1e1000000000 scientific notation overflow rejection
    let res_sci = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "1e1000000000",
                        "operation": "simplify"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_sci.status(), StatusCode::BAD_REQUEST);

    // 3. taylor order 10000 rejection
    let res_taylor = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "sin(x)",
                        "operation": "taylor",
                        "params": { "variable": "x", "center": "0", "order": 10000 }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_taylor.status(), StatusCode::BAD_REQUEST);

    // 4. orthopoly degree 1000 rejection
    let res_ortho = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expression": "x",
                        "operation": "orthopoly",
                        "params": { "kind": "legendre_p", "degree": 1000, "variable": "x" }
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res_ortho.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_phase53_json_schema_contracts() {
    let app = build_app(test_state());

    // 1. GET /api/health
    let r_health = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r_health.status(), StatusCode::OK);
    let h: serde_json::Value =
        serde_json::from_slice(&r_health.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(h["status"], "ok");
    assert!(h["crate_name"].is_string());

    // 2. GET /api/operations
    let r_ops = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/operations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r_ops.status(), StatusCode::OK);
    let ops: Vec<serde_json::Value> =
        serde_json::from_slice(&r_ops.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert!(!ops.is_empty());
    for op in &ops {
        assert!(op["id"].is_string());
        assert!(op["name"].is_string());
        assert!(op["category"].is_string());
        assert!(op["description"].is_string());
        assert!(op["needs_variables"].is_boolean());
        assert!(op["needs_env"].is_boolean());
        assert!(op["params"].is_array());
    }

    // 3. GET /api/rules
    let r_rules = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/rules")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r_rules.status(), StatusCode::OK);
    let rules: serde_json::Value =
        serde_json::from_slice(&r_rules.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert!(rules["total"].as_u64().unwrap_or(0) >= 92);
    assert!(rules["by_file"].is_array());
    assert!(rules["rules"].is_array());

    // 4. GET /api/functions
    let r_fns = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/functions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r_fns.status(), StatusCode::OK);
    let fns: serde_json::Value =
        serde_json::from_slice(&r_fns.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert!(fns["functions"].is_array());

    // 5. POST /api/parse
    let r_parse = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/parse")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "expression": "sin(x)", "input_format": "maxima" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r_parse.status(), StatusCode::OK);
    let parse_json: serde_json::Value =
        serde_json::from_slice(&r_parse.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(parse_json["ok"], true);
    assert!(parse_json["latex"].is_string());
    assert!(parse_json["variables"].is_array());

    // 6. POST /api/compute (200 OK)
    let r_comp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "expression": "x^2 - 1", "operation": "factor" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r_comp.status(), StatusCode::OK);
    let comp_json: serde_json::Value =
        serde_json::from_slice(&r_comp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(comp_json["success"], true);
    assert_eq!(comp_json["status"], "ok");
    assert!(comp_json["result"].is_string());
    assert!(comp_json["result_latex"].is_string());
    assert!(comp_json["outputs"].is_array());
    assert!(comp_json["elapsed_ms"].is_u64());

    // 7. POST /api/compute (400 Bad Request contract)
    let r_err = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/compute")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({ "expression": "2**", "operation": "simplify" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r_err.status(), StatusCode::BAD_REQUEST);
    let err_json: serde_json::Value =
        serde_json::from_slice(&r_err.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(err_json["status"], "bad_request");
    assert!(err_json["error"].is_string());
    assert!(err_json["error_span"]["start"].is_u64());
    assert!(err_json["error_span"]["end"].is_u64());
}
