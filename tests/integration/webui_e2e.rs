//! Final end-to-end integration suite for the webUI service layer.

use maxima_web::api::{ComputeRequest, ParseRequest, Status};
use maxima_web::service::{compute, functions_catalog, operations_catalog, parse, rules_catalog};
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
        params: HashMap::new(),
    }
}

fn req_with_params(expr: &str, op: &str, params: &[(&str, serde_json::Value)]) -> ComputeRequest {
    let mut param_map = HashMap::new();
    for (k, v) in params {
        param_map.insert(k.to_string(), v.clone());
    }
    ComputeRequest {
        expression: expr.to_string(),
        input_format: "maxima".to_string(),
        operation: op.to_string(),
        variables: Vec::new(),
        variable: String::new(),
        float_type: "f64".to_string(),
        env: HashMap::new(),
        params: param_map,
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
fn operations_catalog_contract() {
    let ops = operations_catalog();
    assert!(ops.len() >= 18);
    for op in &ops {
        assert!(!op.id.is_empty());
        assert!(!op.name.is_empty());
        assert!(!op.category.is_empty());
        assert!(!op.description.is_empty());
    }
    assert!(
        ops.iter()
            .any(|o| o.id == "expand" && o.category == "algebra")
    );
    assert!(
        ops.iter()
            .any(|o| o.id == "integrate" && o.category == "calculus")
    );
    assert!(ops.iter().any(|o| o.id == "solve" && o.category == "solve"));
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
    assert_eq!(resp.status, Status::Ok);
}

#[test]
fn compute_matrix_roundtrip() {
    let resp = compute(&req("matrix([1, 2], [3, 4])", "simplify", &[]));
    assert!(resp.success, "matrix compute failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
}

#[test]
fn compute_provenance_operation() {
    let resp = compute(&req("x", "provenance", &[]));
    assert!(resp.success);
    assert_eq!(resp.status, Status::Ok);
    assert!(resp.extra.contains_key("total"));
}

#[test]
fn compute_differentiate_operation() {
    let resp = compute(&req("x^2", "differentiate", &["x"]));
    assert!(resp.success, "differentiate failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
}

#[test]
fn compute_expand_cas_operation() {
    let resp = compute(&req("(x + y)^4", "expand", &[]));
    assert!(resp.success, "expand failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
    assert!(resp.result.contains("x") && resp.result.contains("y"));
}

#[test]
fn compute_factor_cas_operation() {
    let resp = compute(&req("x^2 - 1", "factor", &[]));
    assert!(resp.success, "factor failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
    assert!(resp.result.contains("x - 1") || resp.result.contains("x + 1"));
}

#[test]
fn compute_ratsimp_cas_operation() {
    let resp = compute(&req("(x^2 - 1)/(x - 1)", "ratsimp", &[]));
    assert!(resp.success, "ratsimp failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
    assert!(resp.result.contains("x + 1"));
}

#[test]
fn compute_partfrac_cas_operation() {
    let resp = compute(&req("1/(x^2 - 1)", "partfrac", &["x"]));
    assert!(resp.success, "partfrac failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
}

#[test]
fn compute_integrate_cas_operation() {
    let resp = compute(&req("x^2", "integrate", &["x"]));
    assert!(resp.success, "integrate failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
    assert_eq!(resp.verified, Some(true));
}

#[test]
fn compute_integrate_gaussian_unevaluated() {
    let resp = compute(&req("exp(-x^2)", "integrate", &["x"]));
    assert!(resp.success);
    assert_eq!(resp.status, Status::Unevaluated);
    assert!(!resp.warnings.is_empty());
}

#[test]
fn compute_defint_cas_operations() {
    let resp_val = compute(&req_with_params(
        "x^2",
        "defint",
        &[
            ("variable", serde_json::json!("x")),
            ("lower", serde_json::json!("0")),
            ("upper", serde_json::json!("1")),
        ],
    ));
    assert!(resp_val.success, "defint failed: {:?}", resp_val.error);
    assert_eq!(resp_val.status, Status::Ok);
    assert_eq!(resp_val.result, "1/3");

    let resp_div = compute(&req_with_params(
        "1/x^2",
        "defint",
        &[
            ("variable", serde_json::json!("x")),
            ("lower", serde_json::json!("-1")),
            ("upper", serde_json::json!("1")),
        ],
    ));
    assert!(resp_div.success);
    assert_eq!(resp_div.status, Status::Divergent);

    let resp_inf = compute(&req_with_params(
        "exp(-x)",
        "defint",
        &[
            ("variable", serde_json::json!("x")),
            ("lower", serde_json::json!("0")),
            ("upper", serde_json::json!("inf")),
        ],
    ));
    assert!(resp_inf.success);
    assert_eq!(resp_inf.status, Status::Ok);
    assert_eq!(resp_inf.result, "1");
}

#[test]
fn compute_limit_cas_operations() {
    let resp_zero = compute(&req_with_params(
        "sin(x)/x",
        "limit",
        &[
            ("variable", serde_json::json!("x")),
            ("point", serde_json::json!("0")),
        ],
    ));
    assert!(resp_zero.success, "limit failed: {:?}", resp_zero.error);
    assert_eq!(resp_zero.status, Status::Ok);
    assert_eq!(resp_zero.result, "1");

    let resp_inf = compute(&req_with_params(
        "(1 + 1/x)^x",
        "limit",
        &[
            ("variable", serde_json::json!("x")),
            ("point", serde_json::json!("inf")),
        ],
    ));
    assert!(resp_inf.success);
    assert_eq!(resp_inf.status, Status::Ok);
    assert_eq!(resp_inf.result, "%e");
}

#[test]
fn compute_solve_cas_operation() {
    let resp = compute(&req("x^2 - 1", "solve", &["x"]));
    assert!(resp.success, "solve failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
    assert_eq!(resp.outputs.len(), 2);
    assert_eq!(resp.verified, Some(true));

    let resp_exp = compute(&req("exp(x) - 2", "solve", &["x"]));
    assert!(resp_exp.success);
    assert_eq!(resp_exp.status, Status::Ok);
    assert!(resp_exp.result.contains("ln(2)"));
}

#[test]
fn compute_ode2_cas_operation() {
    let resp = compute(&req_with_params(
        "diff(y, x) + y",
        "ode2",
        &[
            ("dependent", serde_json::json!("y")),
            ("variable", serde_json::json!("x")),
        ],
    ));
    assert!(resp.success, "ode2 failed: {:?}", resp.error);
    assert_eq!(resp.status, Status::Ok);
    assert!(resp.extra.contains_key("constants"));
}

#[test]
fn compute_parse_endpoint() {
    let parse_req = ParseRequest {
        expression: "sin(x) + cos(y)".into(),
        input_format: "maxima".into(),
    };
    let parse_resp = parse(&parse_req);
    assert!(parse_resp.ok);
    assert!(parse_resp.variables.contains(&"x".to_string()));
    assert!(parse_resp.variables.contains(&"y".to_string()));
    assert!(parse_resp.latex.is_some());
}
