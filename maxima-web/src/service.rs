//! Core computation service for the maxima-rs webUI.
//!
//! Translates JSON API requests into maxima-rs pipeline operations
//! (parse, simplify, differentiate, CSE, codegen, evaluate) and back.

use std::collections::HashMap;
use std::time::Instant;

use maxima_rs::codegen::{CodegenConfig, FloatType};
use maxima_rs::cse::eliminate_common_subexprs;
use maxima_rs::prelude::*;
use maxima_rs::saturate::SaturationConfig;

use crate::api::{ComputeResponse, FileRuleCount, RuleEntry, RulesResponse};

/// Run a single compute operation, mutating [`ComputeResponse`] in place.
/// Returns a `String` error on failure; the caller decides how to report it.
fn run_operation(
    req: &crate::api::ComputeRequest,
    resp: &mut ComputeResponse,
) -> Result<(), String> {
    let start = Instant::now();

    // Provenance does not require an input expression.
    if req.operation == "provenance" {
        let catalog = rules_catalog();
        resp.extra.insert("total".into(), catalog.total.into());
        resp.extra.insert(
            "by_file".into(),
            serde_json::to_value(&catalog.by_file).unwrap(),
        );
        resp.extra.insert(
            "rules".into(),
            serde_json::to_value(&catalog.rules).unwrap(),
        );
        resp.extra
            .insert("variables".into(), serde_json::Value::Array(vec![]));
        resp.extra.insert(
            "result".into(),
            format!(
                "{} provenanced rules across {} Maxima Lisp files",
                catalog.total,
                catalog.by_file.len()
            )
            .into(),
        );
        resp.extra.insert(
            "result_latex".into(),
            format!("{}\\text{{ rules}}", catalog.total).into(),
        );
        resp.elapsed_ms = start.elapsed().as_millis() as u64;
        return Ok(());
    }

    let expr = parse_input(req)?;
    let detected: Vec<String> = expr.variables();

    match req.operation.as_str() {
        "simplify" | "optimize" => {
            let opt = optimize(&expr, &SaturationConfig::default());
            resp.extra.insert("result".into(), opt.to_string().into());
            resp.extra.insert("result_latex".into(), latex(&opt).into());
            resp.extra
                .insert("variables".into(), serde_json::to_value(&detected).unwrap());
        }
        "parse_maxima" | "parse_latex" => {
            resp.extra.insert("result".into(), expr.to_string().into());
            resp.extra
                .insert("result_latex".into(), latex(&expr).into());
            resp.extra
                .insert("variables".into(), serde_json::to_value(&detected).unwrap());
        }
        "differentiate" => {
            let vars = resolve_vars(req, &detected);
            if vars.is_empty() {
                return Err("differentiate requires at least one variable".into());
            }
            let derivs: Vec<Expr> = vars.iter().map(|v| differentiate(&expr, v)).collect();
            let out: Vec<serde_json::Value> = derivs
                .iter()
                .zip(vars.iter())
                .map(|(d, v)| {
                    serde_json::json!({
                        "variable": v,
                        "result": d.to_string(),
                        "result_latex": latex(d),
                    })
                })
                .collect();
            resp.extra
                .insert("results".into(), serde_json::to_value(&out).unwrap());
            resp.extra
                .insert("variables".into(), serde_json::to_value(&detected).unwrap());
        }
        "gradient" => {
            let vars = resolve_vars(req, &detected);
            if vars.is_empty() {
                return Err("gradient requires at least one variable".into());
            }
            let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            let grads = gradient(&expr, &var_refs);
            let out: Vec<serde_json::Value> = grads
                .iter()
                .zip(vars.iter())
                .map(|(g, v)| {
                    serde_json::json!({
                        "variable": v,
                        "result": g.to_string(),
                        "result_latex": latex(g),
                    })
                })
                .collect();
            resp.extra
                .insert("results".into(), serde_json::to_value(&out).unwrap());
            resp.extra
                .insert("variables".into(), serde_json::to_value(&detected).unwrap());
        }
        "jacobian" | "hessian" => {
            let vars = resolve_vars(req, &detected);
            if vars.len() < 2 {
                return Err(format!("{} requires at least two variables", req.operation));
            }
            let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            let matrix = match req.operation.as_str() {
                "jacobian" => jacobian(&[expr], &var_refs),
                _ => hessian(&expr, &var_refs),
            };
            let rows: Vec<Vec<serde_json::Value>> = matrix
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|e| {
                            serde_json::json!({
                                "result": e.to_string(),
                                "result_latex": latex(e),
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            resp.extra
                .insert("matrix".into(), serde_json::to_value(&rows).unwrap());
            resp.extra
                .insert("variables".into(), serde_json::to_value(&detected).unwrap());
            resp.extra
                .insert("vars".into(), serde_json::to_value(&vars).unwrap());
        }
        "cse" => {
            let opt = optimize(&expr, &SaturationConfig::default());
            let cse = eliminate_common_subexprs(&[opt]);
            let bindings: Vec<CseBinding> = cse
                .bindings
                .iter()
                .map(|b| CseBinding {
                    name: b.name.clone(),
                    expr: b.expr.to_string(),
                    latex: latex(&b.expr),
                })
                .collect();
            let outputs: Vec<String> = cse.outputs.iter().map(|e| e.to_string()).collect();
            resp.extra
                .insert("bindings".into(), serde_json::to_value(&bindings).unwrap());
            resp.extra
                .insert("outputs".into(), serde_json::to_value(&outputs).unwrap());
            resp.extra.insert(
                "outputs_latex".into(),
                serde_json::to_value(cse.outputs.iter().map(latex).collect::<Vec<_>>()).unwrap(),
            );
            resp.extra
                .insert("result".into(), outputs.join(", ").into());
            resp.extra
                .insert("variables".into(), serde_json::to_value(&detected).unwrap());
        }
        "rust_code" => {
            let float_type = parse_float_type(&req.float_type);
            let vars = resolve_vars(req, &detected);
            let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            let opt = optimize(&expr, &SaturationConfig::default());
            let code = emit_rust_fn(&opt, &var_refs, "compute", float_type);
            resp.rust_code = Some(code);
            resp.extra.insert("result".into(), opt.to_string().into());
            resp.extra.insert("result_latex".into(), latex(&opt).into());
            resp.extra
                .insert("variables".into(), serde_json::to_value(&vars).unwrap());
        }
        "evaluate" => {
            let env = parse_env(&req.env)?;
            let val = expr.eval(&env).ok_or_else(|| {
                "evaluation failed (undefined variable or domain error)".to_string()
            })?;
            resp.extra
                .insert("result".into(), format!("{:.10}", val).into());
            resp.extra
                .insert("result_latex".into(), latex_const(val).into());
            resp.extra
                .insert("variables".into(), serde_json::to_value(&detected).unwrap());
        }
        other => return Err(format!("unknown operation: {other}")),
    }

    resp.elapsed_ms = start.elapsed().as_millis() as u64;
    Ok(())
}

pub fn compute(req: &crate::api::ComputeRequest) -> ComputeResponse {
    let mut resp = ComputeResponse::ok();
    resp.operation = req.operation.clone();
    if let Err(e) = run_operation(req, &mut resp) {
        resp.success = false;
        resp.error = Some(e);
    }
    resp
}

/// Build the 1:1 Maxima rule coverage report surfaced by `GET /api/rules`.
pub fn rules_catalog() -> RulesResponse {
    let provenances = maxima_rs::rules::all_rule_provenances();
    let total = provenances.len();

    let mut rules: Vec<RuleEntry> = provenances
        .into_iter()
        .map(|(name, prov)| RuleEntry {
            name,
            source_file: prov.source_file,
            source_fn: prov.source_fn,
            identity: prov.identity,
            fast_math_only: prov.fast_math_only,
        })
        .collect();
    rules.sort_by(|a, b| {
        a.source_file
            .cmp(b.source_file)
            .then_with(|| a.name.cmp(b.name))
    });

    let mut counts: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for rule in &rules {
        *counts.entry(rule.source_file).or_insert(0) += 1;
    }
    let by_file = counts
        .into_iter()
        .map(|(file, count)| FileRuleCount { file, count })
        .collect();

    RulesResponse {
        total,
        by_file,
        rules,
    }
}

fn parse_input(req: &crate::api::ComputeRequest) -> Result<Expr, String> {
    match req.input_format.as_str() {
        "latex" => parse_latex(&req.expression).map_err(|e| format!("LaTeX parse error: {e}")),
        _ => parse_maxima(&req.expression).map_err(|e| format!("Maxima parse error: {e}")),
    }
}

fn resolve_vars(req: &crate::api::ComputeRequest, detected: &[String]) -> Vec<String> {
    if !req.variables.is_empty() {
        return req.variables.clone();
    }
    if !req.variable.is_empty() {
        return vec![req.variable.clone()];
    }
    detected.to_vec()
}

fn parse_float_type(s: &str) -> FloatType {
    match s {
        "f32" => FloatType::F32,
        _ => FloatType::F64,
    }
}

fn parse_env(env: &HashMap<String, String>) -> Result<HashMap<String, f64>, String> {
    let mut out = HashMap::new();
    for (k, v) in env {
        let f: f64 = v
            .parse()
            .map_err(|_| format!("invalid numeric value for variable '{}': {}", k, v))?;
        out.insert(k.clone(), f);
    }
    Ok(out)
}

fn emit_rust_fn(expr: &Expr, vars: &[&str], fn_name: &str, float_type: FloatType) -> String {
    let cse = eliminate_common_subexprs(std::slice::from_ref(expr));
    let cfg = CodegenConfig { float_type };
    let ty = match float_type {
        FloatType::F64 => "f64",
        FloatType::F32 => "f32",
    };

    let mut bindings = String::new();
    for b in &cse.bindings {
        let ts = maxima_rs::codegen::emit_expr(&b.expr, &cfg);
        bindings.push_str(&format!("        let {} = {};\n", b.name, ts));
    }

    let ret_ts = maxima_rs::codegen::emit_expr(&cse.outputs[0], &cfg);

    let params = vars
        .iter()
        .map(|v| format!("{}: {}", v, ty))
        .collect::<Vec<_>>()
        .join(", ");

    format!("pub fn {fn_name}({params}) -> {ty} {{\n{bindings}        {ret_ts}\n    }}")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CseBinding {
    pub name: String,
    pub expr: String,
    pub latex: String,
}

pub fn latex(e: &Expr) -> String {
    term(e)
}

fn latex_const(v: f64) -> String {
    if v == v.floor() && v.is_finite() {
        format!("{}", v as i64)
    } else {
        format!("{:.6}", v)
    }
}

fn term(e: &Expr) -> String {
    let render = |e: &Expr| term(e);
    let wrap = |e: &Expr| {
        if is_atom(e) {
            render(e)
        } else {
            format!("\\left( {} \\right)", render(e))
        }
    };

    match e {
        Expr::Var(name) => escape_latex(name),
        Expr::Const(r) => rational_latex(r),
        Expr::Add(a, b) => format!("{} + {}", wrap(a), wrap(b)),
        Expr::Sub(a, b) => format!("{} - {}", wrap(a), wrap(b)),
        Expr::Mul(a, b) => format!("{} \\, {}", wrap(a), wrap(b)),
        Expr::Div(a, b) => format!("\\frac{{{}}}{{{}}}", wrap(a), wrap(b)),
        Expr::Pow(a, b) => format!("{{{}}}^{{{}}}", wrap(a), wrap(b)),
        Expr::Neg(a) => format!("-{}", wrap(a)),
        Expr::Sin(a) => format!("\\sin{{{}}}", wrap(a)),
        Expr::Cos(a) => format!("\\cos{{{}}}", wrap(a)),
        Expr::Tan(a) => format!("\\tan{{{}}}", wrap(a)),
        Expr::Exp(a) => format!("e^{{{}}}", wrap(a)),
        Expr::Ln(a) => format!("\\ln{{{}}}", wrap(a)),
        Expr::Sqrt(a) => format!("\\sqrt{{{}}}", wrap(a)),
        Expr::Sinh(a) => format!("\\sinh{{{}}}", wrap(a)),
        Expr::Cosh(a) => format!("\\cosh{{{}}}", wrap(a)),
        Expr::Tanh(a) => format!("\\tanh{{{}}}", wrap(a)),
        Expr::Asin(a) => format!("\\arcsin{{{}}}", wrap(a)),
        Expr::Acos(a) => format!("\\arccos{{{}}}", wrap(a)),
        Expr::Atan(a) => format!("\\arctan{{{}}}", wrap(a)),
        Expr::Pi => "\\pi".to_string(),
        Expr::E => "e".to_string(),
        Expr::Factorial(a) => format!("{}!", wrap(a)),
        Expr::Derivative(inner, var) => {
            format!("\\frac{{d}}{{d {}}}{{{}}}", term(var), wrap(inner))
        }
        Expr::Integral(inner, var) => {
            format!("\\int {} \\, d{}", wrap(inner), term(var))
        }
        Expr::Tensor(name, indices) => {
            let mut s = escape_latex(name);
            for idx in indices {
                s.push_str(&format!("_{{{}}}", idx.name));
            }
            s
        }
        Expr::Sum(expr, var, lower, upper) => {
            let v = render(var);
            let l = render(lower);
            let u = render(upper);
            let e = wrap(expr);
            format!("\\sum_{{{}={}}}}}^{{{}}} {{{}}}", v, l, u, e)
        }
        Expr::Matrix(mat) => {
            let rows: Vec<String> = mat
                .iter()
                .map(|row| row.iter().map(render).collect::<Vec<_>>().join(" & "))
                .collect::<Vec<_>>();
            format!("\\begin{{matrix}} {}\\end{{matrix}}", rows.join(" \\\\ "))
        }
    }
}

fn is_atom(e: &Expr) -> bool {
    matches!(e, Expr::Var(_) | Expr::Const(_) | Expr::Pi | Expr::E)
}

fn rational_latex(r: &num_rational::BigRational) -> String {
    use num_traits::{One, Signed};
    if r.denom().is_one() {
        let n = r.numer();
        if n.is_negative() {
            format!("({})", n)
        } else {
            format!("{}", n)
        }
    } else {
        format!("\\frac{{{}}}{{{}}}", r.numer(), r.denom())
    }
}

fn escape_latex(s: &str) -> String {
    s.replace('\\', "\\backslash ")
        .replace('_', "\\_")
        .replace('^', "\\^{}")
        .replace('{', "\\{")
        .replace('}', "\\}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ComputeRequest;

    fn request(expression: &str, operation: &str) -> ComputeRequest {
        ComputeRequest {
            expression: expression.into(),
            input_format: "maxima".into(),
            operation: operation.into(),
            variables: Vec::new(),
            variable: String::new(),
            float_type: "f64".into(),
            env: HashMap::new(),
        }
    }

    fn result_of(resp: &ComputeResponse) -> &str {
        resp.extra
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
    }

    #[test]
    fn test_simplify_operation() {
        let resp = compute(&request("sin(x)^2 + cos(x)^2", "simplify"));
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(result_of(&resp), "1");
    }

    #[test]
    fn test_optimize_alias_operation() {
        let resp = compute(&request("x + 0", "optimize"));
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(result_of(&resp), "x");
    }

    #[test]
    fn test_differentiate_operation() {
        let mut req = request("x^2", "differentiate");
        req.variables = vec!["x".into()];
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        let results = resp
            .extra
            .get("results")
            .and_then(|v| v.as_array())
            .expect("results array");
        assert_eq!(results.len(), 1);
        let derivative = results[0]
            .get("result")
            .and_then(|v| v.as_str())
            .expect("derivative string");
        assert!(
            derivative.contains('x'),
            "unexpected derivative: {derivative}"
        );
    }

    #[test]
    fn test_evaluate_operation() {
        let mut req = request("x + 1", "evaluate");
        req.env.insert("x".into(), "2".into());
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert!(
            result_of(&resp).starts_with('3'),
            "unexpected value: {}",
            result_of(&resp)
        );
    }

    #[test]
    fn test_unknown_operation_is_error() {
        let resp = compute(&request("x", "does-not-exist"));
        assert!(!resp.success);
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_rules_catalog_shape() {
        let catalog = rules_catalog();
        assert!(catalog.total >= 92, "expected >= 92 rules");
        assert_eq!(catalog.total, catalog.rules.len());

        let summed: usize = catalog.by_file.iter().map(|f| f.count).sum();
        assert_eq!(summed, catalog.total);

        for file in [
            "simp.lisp",
            "trigi.lisp",
            "sinh.lisp",
            "nrat4.lisp",
            "sfacts.lisp",
        ] {
            assert!(
                catalog.by_file.iter().any(|f| f.file == file),
                "missing file {file}"
            );
        }

        for rule in &catalog.rules {
            assert!(!rule.name.is_empty());
            assert!(!rule.source_file.is_empty());
            assert!(!rule.source_fn.is_empty());
            assert!(!rule.identity.is_empty());
        }
    }

    #[test]
    fn test_provenance_operation() {
        // Provenance does not require an expression.
        let resp = compute(&request("", "provenance"));
        assert!(resp.success, "error: {:?}", resp.error);
        let total = resp
            .extra
            .get("total")
            .and_then(|v| v.as_u64())
            .expect("total");
        assert!(total >= 92);
        assert!(resp.extra.contains_key("by_file"));
        assert!(resp.extra.contains_key("rules"));
    }
}
