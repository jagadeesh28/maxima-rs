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
        fast_math: true,
        env: HashMap::new(),
        params: HashMap::new(),
        session_context: None,
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
        fast_math: true,
        env: HashMap::new(),
        params: param_map,
        session_context: None,
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

#[test]
fn compute_phase50_new_operations_and_cas_dispatch() {
    // 1. Direct operations: laplace, ilt, adjoint, transpose, bfloat, orthopoly, assume, is, csign
    let lap = compute(&req_with_params(
        "sin(2*t)",
        "laplace",
        &[("t", "t".into()), ("s", "s".into())],
    ));
    assert!(lap.success, "laplace failed: {:?}", lap.error);

    let inv_lap = compute(&req_with_params(
        "1 / (s - 3)",
        "ilt",
        &[("s", "s".into()), ("t", "t".into())],
    ));
    assert!(inv_lap.success, "ilt failed: {:?}", inv_lap.error);

    let adj = compute(&req("matrix([1, 2], [3, 4])", "adjoint", &[]));
    assert!(adj.success);
    assert_eq!(adj.result, "[[4, (-2)], [(-3), 1]]");

    let tr = compute(&req("matrix([1, 2], [3, 4])", "transpose", &[]));
    assert!(tr.success);
    assert_eq!(tr.result, "[[1, 3], [2, 4]]");

    let bf = compute(&req_with_params("%pi", "bfloat", &[("fpprec", 20.into())]));
    assert!(bf.success);

    let op = compute(&req_with_params(
        "x",
        "orthopoly",
        &[("kind", "legendre_p".into()), ("degree", 2.into())],
    ));
    assert!(op.success);

    let asm = compute(&req("x > 0", "assume", &[]));
    assert!(asm.success);

    let pred = compute(&req("5 > 2", "is", &[]));
    assert!(pred.success);
    assert_eq!(pred.result, "true");

    let cs = compute(&req("exp(%i*%pi)", "csign", &[]));
    assert!(cs.success);
    assert_eq!(cs.result, "neg");

    // 2. Auto-dispatch from Maxima command strings
    let cmd_lap = compute(&req("laplace(cos(3*t), t, s)", "simplify", &[]));
    assert!(cmd_lap.success);
    assert_eq!(cmd_lap.operation, "laplace");

    let cmd_adj = compute(&req("adjoint(matrix([1, 2], [3, 4]))", "simplify", &[]));
    assert!(cmd_adj.success);
    assert_eq!(cmd_adj.operation, "adjoint");
}

#[test]
fn compute_phase51_session_context_continuity_and_bounds() {
    use maxima_web::api::SessionContext;

    let mut ctx = Some(SessionContext::default());

    // 1. Variable binding: a: x^3
    let mut r1 = req("a: x^3", "simplify", &[]);
    r1.session_context = ctx.clone();
    let resp1 = compute(&r1);
    assert!(resp1.success, "assign failed: {:?}", resp1.error);
    assert_eq!(resp1.operation, "assign");
    ctx = resp1.session_context.clone();
    assert_eq!(
        ctx.as_ref()
            .and_then(|c| c.bindings.get("a"))
            .map(String::as_str),
        Some("(x ^ 3)")
    );

    // 2. Differentiate bound variable `a` w.r.t. x -> 3 * x^2
    let mut r2 = req("a", "differentiate", &["x"]);
    r2.session_context = ctx.clone();
    let resp2 = compute(&r2);
    assert!(resp2.success, "diff(a, x) failed: {:?}", resp2.error);
    assert!(resp2.result.contains("3") && resp2.result.contains("x ^ 2"));
    ctx = resp2.session_context.clone();
    assert_eq!(
        ctx.as_ref().and_then(|c| c.last_result.as_deref()),
        Some(resp2.result.as_str())
    );

    // 3. Integrate `%` (last result) w.r.t. x -> x^3
    let mut r3 = req("%", "integrate", &["x"]);
    r3.session_context = ctx.clone();
    let resp3 = compute(&r3);
    assert!(resp3.success, "integrate(%, x) failed: {:?}", resp3.error);
    assert!(resp3.result.contains("x ^ 3"));
    ctx = resp3.session_context.clone();

    // 4. Assume `n > 0` and verify `is(n > 0)` + `integrate(x^n, x)` + `limit(x^n, x, inf)`
    let mut r4 = req("assume(n > 0)", "simplify", &[]);
    r4.session_context = ctx.clone();
    let resp4 = compute(&r4);
    assert!(resp4.success, "assume failed: {:?}", resp4.error);
    ctx = resp4.session_context.clone();
    assert_eq!(ctx.as_ref().map(|c| c.facts.len()), Some(1));

    let mut r5 = req("is(n > 0)", "simplify", &[]);
    r5.session_context = ctx.clone();
    let resp5 = compute(&r5);
    assert!(resp5.success);
    assert_eq!(resp5.result, "true");
    ctx = resp5.session_context.clone();

    let mut r6 = req("x^n", "integrate", &["x"]);
    r6.session_context = ctx.clone();
    let resp6 = compute(&r6);
    assert!(resp6.success);
    assert_eq!(resp6.status, Status::Ok);
    assert!(resp6.result.contains("n + 1"));
    ctx = resp6.session_context.clone();

    // 5. `facts()` and `forget(n > 0)`
    let mut r7 = req("facts()", "simplify", &[]);
    r7.session_context = ctx.clone();
    let resp7 = compute(&r7);
    assert!(resp7.success);
    assert!(resp7.result.contains("n > 0"));
    ctx = resp7.session_context.clone();

    let mut r8 = req("forget(n > 0)", "simplify", &[]);
    r8.session_context = ctx.clone();
    let resp8 = compute(&r8);
    assert!(resp8.success);
    ctx = resp8.session_context.clone();
    assert!(ctx.as_ref().is_some_and(|c| c.facts.is_empty()));

    // 6. `kill(all)` resets bindings, last_result, outputs, and facts
    let mut r9 = req("kill(all)", "simplify", &[]);
    r9.session_context = ctx.clone();
    let resp9 = compute(&r9);
    assert!(resp9.success);
    assert_eq!(resp9.result, "done");
    let cleared = resp9.session_context.expect("session_context returned");
    assert!(cleared.last_result.is_none());
    assert!(cleared.bindings.is_empty());
    assert!(cleared.outputs.is_empty());
    assert!(cleared.facts.is_empty());
    assert_eq!(cleared.next_index, 1);

    // 7. Bounds enforcement: > 32 bindings rejected
    let mut over_ctx = SessionContext::default();
    for i in 0..33 {
        over_ctx.bindings.insert(format!("v{i}"), "1".to_string());
    }
    let mut r_over = req("x + 1", "simplify", &[]);
    r_over.session_context = Some(over_ctx);
    let resp_over = compute(&r_over);
    assert!(!resp_over.success);
    assert_eq!(resp_over.status, Status::Error);
}
