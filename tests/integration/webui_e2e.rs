//! Final end-to-end integration suite for the webUI service layer (Phase 33).

use maxima_web::api::ComputeRequest;
use maxima_web::service::{compute, functions_catalog, rules_catalog};
use std::collections::HashMap;

fn req(expr: &str, op: &str, vars: &[&str]) -> ComputeRequest {
    ComputeRequest {
        expression: expr.to_string(),
        input_format: "maxima".to_string(),
        operation: op.to_string(),
        variables: vars.iter().map(|s| s.to_string()).collect(),
        variable: vars.first().map(|s| s.to_string()).unwrap_or_default(),
        float_type: "f64".to_string(),
        env: HashMap::new(),
    }
}

#[test]
fn functions_catalog_contract() {
    let cat = functions_catalog();
    assert!(!cat.functions.is_empty());
    assert!(cat.functions.iter().any(|f| f.name == "sin"));
    assert!(cat.functions.iter().any(|f| f.name == "+"));
    assert!(cat.functions.iter().any(|f| f.name == "expand"));
    assert!(cat.functions.iter().any(|f| f.name == "ode2"));
}

#[test]
fn rules_catalog_reports_maxima_rules() {
    let rules = rules_catalog();
    assert!(
        rules.total >= 92,
        "expected >= 92 rules, got {}",
        rules.total
    );
}

#[test]
fn compute_simplify_recursive_expression() {
    let resp = compute(&req("sin(cos(x))", "simplify", &["x"]));
    assert!(resp.success, "simplify failed: {:?}", resp.error);
}

#[test]
fn compute_matrix_roundtrip() {
    // Matrix parsing/optimization must succeed through the service layer.
    let resp = compute(&req("matrix([1, 2], [3, 4])", "simplify", &[]));
    assert!(resp.success, "matrix compute failed: {:?}", resp.error);
}

#[test]
fn compute_provenance_operation() {
    let resp = compute(&req("x", "provenance", &[]));
    assert!(resp.success);
    assert!(resp.extra.contains_key("total"));
}

#[test]
fn compute_differentiate_operation() {
    let resp = compute(&req("x^2", "differentiate", &["x"]));
    assert!(resp.success, "differentiate failed: {:?}", resp.error);
}
