//! Core computation service for the maxima-rs webUI.
//!
//! Translates JSON API requests into maxima-rs pipeline operations
//! (parse, simplify, differentiate, CAS algorithms, CSE, codegen, evaluate) and back.

use std::collections::HashMap;
use std::time::Instant;

use maxima_rs::codegen::{CodegenConfig, FloatType};
use maxima_rs::cse::eliminate_common_subexprs;
use maxima_rs::prelude::*;
use maxima_rs::rules::RuleConfig;
use maxima_rs::saturate::SaturationConfig;

use crate::api::{
    ComputeRequest, ComputeResponse, ErrorSpan, FileRuleCount, Operation, Output, ParamSpec,
    ParseRequest, ParseResponse, RuleEntry, RulesResponse, Status,
};

static PARAM_VAR: &[ParamSpec] = &[ParamSpec {
    name: "variable",
    param_type: "variable",
    required: true,
    default: Some("x"),
    accepts: None,
}];

static PARAM_VARS: &[ParamSpec] = &[ParamSpec {
    name: "variables",
    param_type: "variables",
    required: false,
    default: None,
    accepts: None,
}];

static PARAM_DEFINT: &[ParamSpec] = &[
    ParamSpec {
        name: "variable",
        param_type: "variable",
        required: true,
        default: Some("x"),
        accepts: None,
    },
    ParamSpec {
        name: "lower",
        param_type: "point",
        required: true,
        default: Some("0"),
        accepts: Some(&["minf", "-inf"]),
    },
    ParamSpec {
        name: "upper",
        param_type: "point",
        required: true,
        default: Some("1"),
        accepts: Some(&["inf"]),
    },
];

static PARAM_LIMIT: &[ParamSpec] = &[
    ParamSpec {
        name: "variable",
        param_type: "variable",
        required: true,
        default: Some("x"),
        accepts: None,
    },
    ParamSpec {
        name: "point",
        param_type: "point",
        required: true,
        default: Some("0"),
        accepts: Some(&["inf", "minf"]),
    },
];

static PARAM_TAYLOR: &[ParamSpec] = &[
    ParamSpec {
        name: "variable",
        param_type: "variable",
        required: true,
        default: Some("x"),
        accepts: None,
    },
    ParamSpec {
        name: "center",
        param_type: "point",
        required: false,
        default: Some("0"),
        accepts: None,
    },
    ParamSpec {
        name: "order",
        param_type: "int",
        required: false,
        default: Some("5"),
        accepts: None,
    },
];

static PARAM_ODE2: &[ParamSpec] = &[
    ParamSpec {
        name: "dependent",
        param_type: "variable",
        required: true,
        default: Some("y"),
        accepts: None,
    },
    ParamSpec {
        name: "variable",
        param_type: "variable",
        required: true,
        default: Some("x"),
        accepts: None,
    },
];

static PARAM_CODEGEN: &[ParamSpec] = &[
    ParamSpec {
        name: "variables",
        param_type: "variables",
        required: false,
        default: None,
        accepts: None,
    },
    ParamSpec {
        name: "float_type",
        param_type: "enum",
        required: false,
        default: Some("f64"),
        accepts: Some(&["f64", "f32"]),
    },
];

static PARAM_CHARPOLY: &[ParamSpec] = &[ParamSpec {
    name: "variable",
    param_type: "variable",
    required: false,
    default: Some("%lambda"),
    accepts: None,
}];

static PARAM_RESIDUE: &[ParamSpec] = &[
    ParamSpec {
        name: "variable",
        param_type: "variable",
        required: true,
        default: Some("z"),
        accepts: None,
    },
    ParamSpec {
        name: "point",
        param_type: "point",
        required: true,
        default: Some("0"),
        accepts: None,
    },
];

static PARAM_EMPTY: &[ParamSpec] = &[];

/// Declarative, authoritative specification of all available operations.
pub static OPERATIONS: &[Operation] = &[
    // ── Algebra ──────────────────────────────────────────────────────────
    Operation {
        id: "simplify",
        name: "Simplify / Optimize",
        category: "algebra",
        description: "E-graph equality saturation + cost-optimal extraction",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "expand",
        name: "Expand",
        category: "algebra",
        description: "Expand polynomial products and powers",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "factor",
        name: "Factor",
        category: "algebra",
        description: "Factor polynomial into irreducible factors over rationals",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "ratsimp",
        name: "Rational Simplify",
        category: "algebra",
        description: "Canonical coprime quotient reduction via polynomial GCD",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "partfrac",
        name: "Partial Fractions",
        category: "algebra",
        description: "Decompose rational function into partial fractions",
        needs_variables: true,
        needs_env: false,
        params: PARAM_VAR,
    },
    // ── Calculus ─────────────────────────────────────────────────────────
    Operation {
        id: "differentiate",
        name: "Differentiate",
        category: "calculus",
        description: "Exact symbolic derivative d/dx with respect to variable",
        needs_variables: true,
        needs_env: false,
        params: PARAM_VAR,
    },
    Operation {
        id: "integrate",
        name: "Indefinite Integral",
        category: "calculus",
        description: "Symbolic antiderivative via Risch/Hermite/LRT algorithms",
        needs_variables: true,
        needs_env: false,
        params: PARAM_VAR,
    },
    Operation {
        id: "defint",
        name: "Definite Integral",
        category: "calculus",
        description: "Definite integral with real pole scanning and improper limits",
        needs_variables: true,
        needs_env: false,
        params: PARAM_DEFINT,
    },
    Operation {
        id: "limit",
        name: "Limit",
        category: "calculus",
        description: "Two-sided limit via Dominik Gruntz's MRV algorithm",
        needs_variables: true,
        needs_env: false,
        params: PARAM_LIMIT,
    },
    Operation {
        id: "taylor",
        name: "Taylor / Series",
        category: "calculus",
        description: "Truncated Taylor/Laurent series asymptotic expansion",
        needs_variables: true,
        needs_env: false,
        params: PARAM_TAYLOR,
    },
    // ── Solvers ──────────────────────────────────────────────────────────
    Operation {
        id: "solve",
        name: "Solve Equation",
        category: "solve",
        description: "Find exact closed-form roots of expr = 0",
        needs_variables: true,
        needs_env: false,
        params: PARAM_VAR,
    },
    Operation {
        id: "ode2",
        name: "Solve ODE",
        category: "solve",
        description: "Solve 1st-order and 2nd-order ordinary differential equations",
        needs_variables: true,
        needs_env: false,
        params: PARAM_ODE2,
    },
    // ── Matrix ───────────────────────────────────────────────────────────
    Operation {
        id: "determinant",
        name: "Determinant",
        category: "matrix",
        description: "Matrix determinant via Bareiss fraction-free algorithm",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "invert",
        name: "Invert Matrix",
        category: "matrix",
        description: "Matrix adjoint inverse",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "charpoly",
        name: "Characteristic Polynomial",
        category: "matrix",
        description: "Characteristic polynomial det(M - λI)",
        needs_variables: true,
        needs_env: false,
        params: PARAM_CHARPOLY,
    },
    Operation {
        id: "trace",
        name: "Matrix Trace",
        category: "matrix",
        description: "Sum of diagonal elements",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "eigenvalues",
        name: "Eigenvalues",
        category: "matrix",
        description: "Exact eigenvalues from characteristic polynomial roots",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    // ── Complex ──────────────────────────────────────────────────────────
    Operation {
        id: "rectform",
        name: "Rectangular Form",
        category: "complex",
        description: "Decompose into real and imaginary parts u + %i*v",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "polarform",
        name: "Polar Form",
        category: "complex",
        description: "Convert into polar form r * %e^(%i*theta)",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "realpart",
        name: "Real Part",
        category: "complex",
        description: "Extract real component Re(z)",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "imagpart",
        name: "Imaginary Part",
        category: "complex",
        description: "Extract imaginary component Im(z)",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "conjugate",
        name: "Complex Conjugate",
        category: "complex",
        description: "Complex conjugate u - %i*v",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "carg",
        name: "Complex Argument",
        category: "complex",
        description: "Phase angle in (-pi, pi]",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "cabs",
        name: "Complex Absolute Value",
        category: "complex",
        description: "Modulus sqrt(u^2 + v^2)",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "residue",
        name: "Residue",
        category: "complex",
        description: "Laurent series residue at pole z = z0",
        needs_variables: true,
        needs_env: false,
        params: PARAM_RESIDUE,
    },
    // ── Vector ───────────────────────────────────────────────────────────
    Operation {
        id: "gradient",
        name: "Gradient",
        category: "vector",
        description: "Vector of partial derivatives ∇f",
        needs_variables: true,
        needs_env: false,
        params: PARAM_VARS,
    },
    Operation {
        id: "jacobian",
        name: "Jacobian",
        category: "vector",
        description: "Jacobian matrix of partial derivatives",
        needs_variables: true,
        needs_env: false,
        params: PARAM_VARS,
    },
    Operation {
        id: "hessian",
        name: "Hessian",
        category: "vector",
        description: "Hessian matrix of second-order partial derivatives",
        needs_variables: true,
        needs_env: false,
        params: PARAM_VARS,
    },
    // ── Codegen ──────────────────────────────────────────────────────────
    Operation {
        id: "cse",
        name: "Common Subexpressions (CSE)",
        category: "codegen",
        description: "Factor repeated subcomputations into let-bindings",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "rust_code",
        name: "Generate Rust Scalar",
        category: "codegen",
        description: "Zero-overhead optimized Rust scalar function (CSE + codegen)",
        needs_variables: true,
        needs_env: false,
        params: PARAM_CODEGEN,
    },
    Operation {
        id: "rust_gradient",
        name: "Generate Rust Gradient",
        category: "codegen",
        description: "Combined value and gradient function with joint CSE",
        needs_variables: true,
        needs_env: false,
        params: PARAM_CODEGEN,
    },
    // ── Reference ────────────────────────────────────────────────────────
    Operation {
        id: "evaluate",
        name: "Evaluate Numeric",
        category: "reference",
        description: "Substitute variable values and compute a number",
        needs_variables: false,
        needs_env: true,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "provenance",
        name: "Maxima Rule Coverage",
        category: "reference",
        description: "1:1 Maxima LISP citations and per-file rule counts",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "parse_maxima",
        name: "Parse Maxima",
        category: "reference",
        description: "Parse a Maxima-syntax expression into the symbolic AST",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
    Operation {
        id: "parse_latex",
        name: "Parse LaTeX",
        category: "reference",
        description: "Parse a LaTeX math formula into the symbolic AST",
        needs_variables: false,
        needs_env: false,
        params: PARAM_EMPTY,
    },
];

/// Returns the declarative list of available operations.
pub fn operations_catalog() -> Vec<Operation> {
    OPERATIONS.to_vec()
}

/// Run a single compute operation, mutating [`ComputeResponse`] in place.
fn run_operation(
    req: &ComputeRequest,
    resp: &mut ComputeResponse,
) -> Result<(), (String, Option<ErrorSpan>)> {
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
        let desc = format!(
            "{} provenanced rules across {} Maxima Lisp files",
            catalog.total,
            catalog.by_file.len()
        );
        let latex_desc = format!("{}\\text{{ rules}}", catalog.total);
        resp.result = desc.clone();
        resp.result_latex = latex_desc.clone();
        resp.extra.insert("result".into(), desc.clone().into());
        resp.extra
            .insert("result_latex".into(), latex_desc.clone().into());
        resp.outputs = vec![Output::new("rules", desc, latex_desc)];
        resp.status = Status::Ok;
        resp.elapsed_ms = start.elapsed().as_millis() as u64;
        return Ok(());
    }

    validate_request_bounds(req).map_err(|e| (e, None))?;

    // For jacobian, allow vector `[f1, f2, ...]` inputs in Maxima format.
    let (expr, vector_exprs) = if req.operation == "jacobian"
        && req.input_format != "latex"
        && is_bracket_vector(&req.expression)
    {
        let vec_exprs = parse_bracket_vector(&req.expression)?;
        let first = vec_exprs.first().cloned().unwrap_or_else(|| Expr::int(0));
        (first, Some(vec_exprs))
    } else {
        (parse_input(req)?, None)
    };

    let detected: Vec<String> = if let Some(ref vec_exprs) = vector_exprs {
        let mut set = std::collections::BTreeSet::new();
        for e in vec_exprs {
            for v in e.variables() {
                set.insert(v);
            }
        }
        set.into_iter().collect()
    } else {
        expr.variables()
    };
    resp.extra
        .insert("variables".into(), serde_json::json!(detected));

    let cas_cmd = if req.input_format != "latex" {
        maxima_rs::parser::maxima::parse_maxima_cas(&req.expression).ok()
    } else {
        None
    };
    let op_name = if req.operation == "simplify" || req.operation.is_empty() {
        if let Some(ref cmd) = cas_cmd {
            match cmd {
                maxima_rs::parser::maxima::CasCommand::Defint(..) => "defint",
                maxima_rs::parser::maxima::CasCommand::Integrate(..) => "integrate",
                maxima_rs::parser::maxima::CasCommand::Taylor(..) => "taylor",
                maxima_rs::parser::maxima::CasCommand::Limit(..) => "limit",
                maxima_rs::parser::maxima::CasCommand::Solve(..) => "solve",
                maxima_rs::parser::maxima::CasCommand::Ode2(..) => "ode2",
                maxima_rs::parser::maxima::CasCommand::Partfrac(..) => "partfrac",
                maxima_rs::parser::maxima::CasCommand::Expand(..) => "expand",
                maxima_rs::parser::maxima::CasCommand::Factor(..) => "factor",
                maxima_rs::parser::maxima::CasCommand::Ratsimp(..) => "ratsimp",
            }
        } else {
            req.operation.as_str()
        }
    } else {
        req.operation.as_str()
    };
    resp.operation = op_name.to_string();

    let sat_cfg = saturation_config(req);

    match op_name {
        "simplify" | "optimize" => {
            let opt = optimize(&expr, &sat_cfg);
            let res_str = opt.to_string();
            let res_latex = latex(&opt);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("simplified", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "expand" => {
            let expanded = expand(&expr);
            let res_str = expanded.to_string();
            let res_latex = latex(&expanded);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("expanded", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "factor" => {
            let factored = factor(&expr);
            let res_str = factored.to_string();
            let res_latex = latex(&factored);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("factored", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "ratsimp" => {
            let simplified = ratsimp(&expr).map_err(|e| (format!("ratsimp error: {e}"), None))?;
            let res_str = simplified.to_string();
            let res_latex = latex(&simplified);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("ratsimp", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "partfrac" => {
            let detected_v =
                if let Some(maxima_rs::parser::maxima::CasCommand::Partfrac(_, ref v)) = cas_cmd {
                    Some(v.clone())
                } else {
                    None
                };
            let v = if let Some(v_str) = get_str_param(req, "variable") {
                v_str.to_string()
            } else if let Some(v_str) = detected_v {
                v_str
            } else {
                param_var(req, "variable", &detected).map_err(|e| (e, None))?
            };
            let decomp = partfrac(&expr, &v).map_err(|e| (format!("partfrac error: {e}"), None))?;
            let res_str = decomp.to_string();
            let res_latex = latex(&decomp);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new(format!("partfrac({v})"), res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "parse_maxima" | "parse_latex" => {
            let res_str = expr.to_string();
            let res_latex = latex(&expr);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("parsed", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "differentiate" => {
            let vars = resolve_vars(req, &detected).map_err(|e| (e, None))?;
            if vars.is_empty() {
                return Err(("differentiate requires at least one variable".into(), None));
            }
            let derivs: Vec<Expr> = vars.iter().map(|v| differentiate(&expr, v)).collect();
            let outputs: Vec<Output> = derivs
                .iter()
                .zip(vars.iter())
                .map(|(d, v)| {
                    Output::new(
                        format!("d/d{v}"),
                        d.to_string(),
                        format!("\\frac{{d}}{{d {v}}} = {}", latex(d)),
                    )
                })
                .collect();
            let out_json: Vec<serde_json::Value> = derivs
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
            if let Some(first) = outputs.first() {
                resp.result = first.result.clone();
                resp.result_latex = first.result_latex.clone();
            }
            resp.outputs = outputs;
            resp.extra
                .insert("results".into(), serde_json::to_value(&out_json).unwrap());
            resp.status = Status::Ok;
        }
        "integrate" => {
            let detected_v =
                if let Some(maxima_rs::parser::maxima::CasCommand::Integrate(_, ref v)) = cas_cmd {
                    Some(v.clone())
                } else {
                    None
                };
            let v = if let Some(v_str) = get_str_param(req, "variable") {
                v_str.to_string()
            } else if let Some(v_str) = detected_v {
                v_str
            } else {
                param_var(req, "variable", &detected).map_err(|e| (e, None))?
            };
            match integrate_result(&expr, &v) {
                IntegrationResult::Solved(res) => {
                    let res_str = res.to_string();
                    let res_latex = latex(&res);
                    resp.result = res_str.clone();
                    resp.result_latex = res_latex.clone();
                    resp.extra.insert("result".into(), res_str.clone().into());
                    resp.extra
                        .insert("result_latex".into(), res_latex.clone().into());
                    resp.status = Status::Ok;
                    let is_verified = verify_integral(&expr, &res, &v);
                    resp.verified = Some(is_verified);
                    if is_verified {
                        resp.verification =
                            Some("numerical derivative matches integrand (d/dv F ≈ f)".into());
                    }
                    resp.outputs = vec![Output::new(format!("∫ d{v}"), res_str, res_latex)];
                }
                IntegrationResult::ProvenNonElementary => {
                    let res_str = format!("integrate({}, {})", expr, v);
                    let res_latex = format!("\\int {} \\, d{}", latex(&expr), v);
                    resp.result = res_str.clone();
                    resp.result_latex = res_latex.clone();
                    resp.extra.insert("result".into(), res_str.clone().into());
                    resp.extra
                        .insert("result_latex".into(), res_latex.clone().into());
                    resp.status = Status::Unevaluated;
                    resp.warnings.push(
                        "proven non-elementary: no elementary antiderivative exists (Liouville's theorem)".into(),
                    );
                    resp.verified = Some(false);
                    resp.outputs = vec![Output::new(format!("∫ d{v}"), res_str, res_latex)];
                }
                IntegrationResult::Unsupported(reason) => {
                    let res_str = format!("integrate({}, {})", expr, v);
                    let res_latex = format!("\\int {} \\, d{}", latex(&expr), v);
                    resp.result = res_str.clone();
                    resp.result_latex = res_latex.clone();
                    resp.extra.insert("result".into(), res_str.clone().into());
                    resp.extra
                        .insert("result_latex".into(), res_latex.clone().into());
                    resp.status = Status::Unevaluated;
                    resp.warnings.push(format!("unsupported: {reason}"));
                    resp.verified = Some(false);
                    resp.outputs = vec![Output::new(format!("∫ d{v}"), res_str, res_latex)];
                }
            }
        }
        "defint" => {
            let mut detected_var = None;
            let mut detected_a = None;
            let mut detected_b = None;
            if let Some(maxima_rs::parser::maxima::CasCommand::Defint(
                _,
                ref v,
                ref a_expr,
                ref b_expr,
            )) = cas_cmd
            {
                detected_var = Some(v.clone());
                detected_a = Some(a_expr.to_string());
                detected_b = Some(b_expr.to_string());
            }

            let v = if let Some(v_str) = get_str_param(req, "variable") {
                v_str.to_string()
            } else if let Some(v_str) = detected_var {
                v_str
            } else {
                param_var(req, "variable", &detected).map_err(|e| (e, None))?
            };

            let a = if let Some(a_str) = get_str_param(req, "lower") {
                parse_point_str(a_str, &req.input_format)
                    .map_err(|e| (format!("invalid lower limit '{a_str}': {e}"), None))?
            } else if let Some(a_str) = detected_a {
                parse_point_str(&a_str, &req.input_format)
                    .map_err(|e| (format!("invalid lower limit '{a_str}': {e}"), None))?
            } else {
                param_point(req, "lower", Some("0")).map_err(|e| (e, None))?
            };

            let b = if let Some(b_str) = get_str_param(req, "upper") {
                parse_point_str(b_str, &req.input_format)
                    .map_err(|e| (format!("invalid upper limit '{b_str}': {e}"), None))?
            } else if let Some(b_str) = detected_b {
                parse_point_str(&b_str, &req.input_format)
                    .map_err(|e| (format!("invalid upper limit '{b_str}': {e}"), None))?
            } else {
                param_point(req, "upper", Some("1")).map_err(|e| (e, None))?
            };

            let outcome = defint(&expr, &v, &a, &b);
            match outcome {
                DefiniteResult::Value(val) => {
                    let res_str = val.to_string();
                    let res_latex = latex(&val);
                    resp.result = res_str.clone();
                    resp.result_latex = res_latex.clone();
                    resp.extra.insert("result".into(), res_str.clone().into());
                    resp.extra
                        .insert("result_latex".into(), res_latex.clone().into());
                    resp.outputs = vec![Output::new(
                        format!("∫_{{{}}}^{{{}}} d{v}", a, b),
                        res_str,
                        res_latex,
                    )];
                    resp.status = Status::Ok;
                    let is_verified = verify_defint(&expr, &val, &v, &a, &b);
                    resp.verified = Some(is_verified);
                    if is_verified {
                        resp.verification =
                            Some("composite Simpson numerical quadrature matches analytical definite integral".into());
                    }
                }
                DefiniteResult::Divergent => {
                    resp.status = Status::Divergent;
                    resp.result = "divergent".into();
                    resp.result_latex = "\\text{divergent}".into();
                    resp.warnings.push(
                        "integral diverges (real pole in interval or divergent bound)".into(),
                    );
                    resp.outputs = vec![Output::new(
                        format!("∫_{{{}}}^{{{}}} d{v}", a, b),
                        "divergent",
                        "\\text{divergent}",
                    )];
                }
                DefiniteResult::Unevaluated => {
                    resp.status = Status::Unevaluated;
                    resp.result = "unevaluated".into();
                    resp.result_latex = "\\text{unevaluated}".into();
                    resp.warnings
                        .push("could not compute closed-form definite integral".into());
                    resp.outputs = vec![Output::new(
                        format!("∫_{{{}}}^{{{}}} d{v}", a, b),
                        "unevaluated",
                        "\\text{unevaluated}",
                    )];
                }
            }
        }
        "limit" => {
            let (detected_v, detected_pt) =
                if let Some(maxima_rs::parser::maxima::CasCommand::Limit(_, ref v, ref pt)) =
                    cas_cmd
                {
                    (Some(v.clone()), Some(pt.clone()))
                } else {
                    (None, None)
                };
            let v = if let Some(v_str) = get_str_param(req, "variable") {
                v_str.to_string()
            } else if let Some(v_str) = detected_v {
                v_str
            } else {
                param_var(req, "variable", &detected).map_err(|e| (e, None))?
            };
            let pt = if let Some(pt_str) = get_str_param(req, "point") {
                parse_point_str(pt_str, &req.input_format)
                    .map_err(|e| (format!("invalid limit point '{pt_str}': {e}"), None))?
            } else if let Some(p) = detected_pt {
                p
            } else {
                param_point(req, "point", Some("0")).map_err(|e| (e, None))?
            };
            let pt_str = pt.to_string();
            match limit(&expr, &v, &pt) {
                Some(lim) => {
                    let res_str = lim.to_string();
                    let res_latex = latex(&lim);
                    resp.result = res_str.clone();
                    resp.result_latex = res_latex.clone();
                    resp.extra.insert("result".into(), res_str.clone().into());
                    resp.extra
                        .insert("result_latex".into(), res_latex.clone().into());
                    resp.outputs = vec![Output::new(
                        format!("lim({v} → {pt_str})"),
                        res_str,
                        res_latex,
                    )];
                    resp.status = Status::Ok;
                }
                None => {
                    resp.status = Status::NoSolution;
                    resp.result = "no limit found".into();
                    resp.result_latex = "\\text{undefined or indeterminate}".into();
                    resp.warnings
                        .push("could not determine closed-form asymptotic limit".into());
                    resp.outputs = vec![Output::new(
                        format!("lim({v} → {pt_str})"),
                        "no limit found",
                        "\\text{indeterminate}",
                    )];
                }
            }
        }
        "taylor" => {
            let (detected_v, detected_c, detected_order) =
                if let Some(maxima_rs::parser::maxima::CasCommand::Taylor(_, ref v, ref c, o)) =
                    cas_cmd
                {
                    (Some(v.clone()), Some(c.clone()), Some(o))
                } else {
                    (None, None, None)
                };
            let v = if let Some(v_str) = get_str_param(req, "variable") {
                v_str.to_string()
            } else if let Some(v_str) = detected_v {
                v_str
            } else {
                param_var(req, "variable", &detected).map_err(|e| (e, None))?
            };
            let center = if let Some(c_str) = get_str_param(req, "center") {
                parse_point_str(c_str, &req.input_format)
                    .map_err(|e| (format!("invalid center '{c_str}': {e}"), None))?
            } else if let Some(c) = detected_c {
                c
            } else {
                param_point(req, "center", Some("0")).map_err(|e| (e, None))?
            };
            let order = if let Some(ord) = detected_order {
                param_int(req, "order", ord).map_err(|e| (e, None))?
            } else {
                param_int(req, "order", 5).map_err(|e| (e, None))?
            };
            if !(0..=64).contains(&order) {
                return Err((
                    format!("taylor order must be between 0 and 64, got {order}"),
                    None,
                ));
            }
            match taylor(&expr, &v, &center, order) {
                Some(series_expr) => {
                    let (base_str, base_latex) = if center.is_zero() {
                        (v.clone(), v.clone())
                    } else {
                        (
                            format!("({v} - {center})"),
                            format!("\\left({v} - {}\\right)", latex(&center)),
                        )
                    };
                    let res_str = format!("{series_expr} + O({base_str}^{})", order + 1);
                    let res_latex = format!(
                        "{} + \\mathcal{{O}}\\left({base_latex}^{{{}}}\\right)",
                        latex(&series_expr),
                        order + 1
                    );
                    resp.result = res_str.clone();
                    resp.result_latex = res_latex.clone();
                    resp.extra.insert("result".into(), res_str.clone().into());
                    resp.extra
                        .insert("result_latex".into(), res_latex.clone().into());
                    resp.outputs = vec![Output::new(
                        format!("taylor({v}, order={order})"),
                        res_str,
                        res_latex,
                    )];
                    resp.status = Status::Ok;
                }
                None => {
                    resp.status = Status::NoSolution;
                    resp.result = "expansion failed".into();
                    resp.result_latex = "\\text{expansion failed}".into();
                    resp.warnings
                        .push("could not expand series around specified center".into());
                    resp.outputs =
                        vec![Output::new("taylor", "expansion failed", "\\text{failed}")];
                }
            }
        }
        "solve" => {
            let detected_v =
                if let Some(maxima_rs::parser::maxima::CasCommand::Solve(_, ref v)) = cas_cmd {
                    Some(v.clone())
                } else {
                    None
                };
            let v = if let Some(v_str) = get_str_param(req, "variable") {
                v_str.to_string()
            } else if let Some(v_str) = detected_v {
                v_str
            } else {
                param_var(req, "variable", &detected).map_err(|e| (e, None))?
            };
            let solution_set = solve(&expr, &v);
            let is_complete = solution_set.is_complete();
            let roots = solution_set.into_vec();
            if roots.is_empty() {
                resp.status = Status::NoSolution;
                resp.result = "no closed-form roots found".into();
                resp.result_latex = "\\emptyset".into();
                resp.outputs = vec![Output::new(v.clone(), "no roots", "\\emptyset")];
            } else {
                let all_verified = roots.iter().all(|r| verify_root(&expr, &v, r));
                let outputs: Vec<Output> = roots
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        Output::new(
                            format!("{v}{}", subscript(i + 1)),
                            r.to_string(),
                            format!("{v}_{{{}}} = {}", i + 1, latex(r)),
                        )
                    })
                    .collect();
                let joined_str = roots
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                let joined_latex = roots.iter().map(latex).collect::<Vec<_>>().join(", ");
                resp.result = joined_str.clone();
                resp.result_latex = joined_latex.clone();
                resp.extra.insert("result".into(), joined_str.into());
                resp.extra
                    .insert("result_latex".into(), joined_latex.into());
                resp.outputs = outputs;
                if is_complete {
                    resp.status = Status::Ok;
                } else {
                    resp.status = Status::Partial;
                    resp.warnings.push(
                        "partial factorization: higher-degree irreducible factors remain".into(),
                    );
                }
                resp.verified = Some(all_verified);
                if all_verified {
                    resp.verification =
                        Some("all found roots back-substitute to zero residual".into());
                }
            }
        }
        "ode2" => {
            let (detected_dep, detected_indep) =
                if let Some(maxima_rs::parser::maxima::CasCommand::Ode2(_, ref dep, ref indep)) =
                    cas_cmd
                {
                    (Some(dep.clone()), Some(indep.clone()))
                } else {
                    (None, None)
                };
            let dep = if let Some(d_str) = get_str_param(req, "dependent") {
                d_str.to_string()
            } else if let Some(d_str) = detected_dep {
                d_str
            } else {
                param_var(req, "dependent", &["y".to_string()]).map_err(|e| (e, None))?
            };
            let indep = if let Some(i_str) = get_str_param(req, "variable") {
                i_str.to_string()
            } else if let Some(i_str) = detected_indep {
                i_str
            } else {
                param_var(req, "variable", &detected).map_err(|e| (e, None))?
            };
            match ode2(&expr, &dep, &indep) {
                Some(sol) => {
                    let res_str = format!("{dep} = {}", sol.solution);
                    let res_latex = format!("{dep} = {}", latex(&sol.solution));
                    resp.result = res_str.clone();
                    resp.result_latex = res_latex.clone();
                    resp.extra.insert("result".into(), res_str.clone().into());
                    resp.extra
                        .insert("result_latex".into(), res_latex.clone().into());
                    resp.extra.insert(
                        "constants".into(),
                        serde_json::to_value(&sol.constants).unwrap(),
                    );
                    resp.outputs = vec![Output::new(format!("{dep}({indep})"), res_str, res_latex)];
                    resp.status = Status::Ok;
                    let is_verified = verify_ode(&expr, &sol.solution, &dep, &indep);
                    resp.verified = Some(is_verified);
                    if is_verified {
                        resp.verification = Some(
                            "ODE solution satisfies differential equation at sample points".into(),
                        );
                    }
                }
                None => {
                    resp.status = Status::NoSolution;
                    resp.result = "no ODE solution found".into();
                    resp.result_latex = "\\text{no solution}".into();
                    resp.warnings
                        .push("ODE form not recognized or non-integrable".into());
                    resp.outputs = vec![Output::new(
                        format!("{dep}({indep})"),
                        "no solution",
                        "\\text{no solution}",
                    )];
                }
            }
        }
        "gradient" => {
            let vars = resolve_vars(req, &detected).map_err(|e| (e, None))?;
            if vars.is_empty() {
                return Err(("gradient requires at least one variable".into(), None));
            }
            let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            let grads = gradient(&expr, &var_refs);
            let outputs: Vec<Output> = grads
                .iter()
                .zip(vars.iter())
                .map(|(g, v)| {
                    Output::new(
                        format!("∂f/∂{v}"),
                        g.to_string(),
                        format!("\\frac{{\\partial f}}{{\\partial {v}}} = {}", latex(g)),
                    )
                })
                .collect();
            let out_json: Vec<serde_json::Value> = grads
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
            if let Some(first) = outputs.first() {
                resp.result = first.result.clone();
                resp.result_latex = first.result_latex.clone();
            }
            resp.outputs = outputs;
            resp.extra
                .insert("results".into(), serde_json::to_value(&out_json).unwrap());
            resp.status = Status::Ok;
        }
        "jacobian" | "hessian" => {
            let vars = resolve_vars(req, &detected).map_err(|e| (e, None))?;
            if vars.is_empty() {
                return Err((
                    format!("{} requires at least one variable", req.operation),
                    None,
                ));
            }
            let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            let matrix = match req.operation.as_str() {
                "jacobian" => {
                    if let Some(ref vec_exprs) = vector_exprs {
                        jacobian(vec_exprs, &var_refs)
                    } else if let Expr::Matrix(ref rows) = expr {
                        let flat: Vec<Expr> = rows.iter().flatten().cloned().collect();
                        jacobian(&flat, &var_refs)
                    } else {
                        jacobian(std::slice::from_ref(&expr), &var_refs)
                    }
                }
                _ => hessian(&expr, &var_refs),
            };
            let matrix_latex = format_matrix_latex(&matrix);
            let matrix_str = format_matrix_string(&matrix);
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
            resp.result = matrix_str.clone();
            resp.result_latex = matrix_latex.clone();
            resp.extra
                .insert("result".into(), matrix_str.clone().into());
            resp.extra
                .insert("result_latex".into(), matrix_latex.clone().into());
            resp.extra
                .insert("matrix".into(), serde_json::to_value(&rows).unwrap());
            resp.extra
                .insert("vars".into(), serde_json::to_value(&vars).unwrap());
            resp.outputs = vec![Output::new(req.operation.clone(), matrix_str, matrix_latex)];
            resp.status = Status::Ok;
        }
        "cse" => {
            let opt = optimize(&expr, &sat_cfg);
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
            let out_latex: Vec<String> = cse.outputs.iter().map(latex).collect();
            resp.extra
                .insert("bindings".into(), serde_json::to_value(&bindings).unwrap());
            resp.extra
                .insert("outputs".into(), serde_json::to_value(&outputs).unwrap());
            resp.extra.insert(
                "outputs_latex".into(),
                serde_json::to_value(&out_latex).unwrap(),
            );
            let res_str = outputs.join(", ");
            let res_latex = out_latex.join(", ");
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("cse_output", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "rust_code" => {
            let float_type = parse_float_type(req).map_err(|e| (e, None))?;
            let vars = resolve_vars(req, &detected).map_err(|e| (e, None))?;
            validate_rust_idents(&vars).map_err(|e| (e, None))?;
            let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            let opt = optimize(&expr, &sat_cfg);
            let code = emit_rust_fn(&opt, &var_refs, "compute", float_type);
            resp.rust_code = Some(code.clone());
            let res_str = opt.to_string();
            let res_latex = latex(&opt);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.extra
                .insert("variables".into(), serde_json::json!(vars));
            resp.outputs = vec![Output::new("rust_code", code, res_latex)];
            resp.status = Status::Ok;
        }
        "rust_gradient" => {
            let float_type = parse_float_type(req).map_err(|e| (e, None))?;
            let vars = resolve_vars(req, &detected).map_err(|e| (e, None))?;
            if vars.is_empty() {
                return Err(("rust_gradient requires at least one variable".into(), None));
            }
            validate_rust_idents(&vars).map_err(|e| (e, None))?;
            let var_refs: Vec<&str> = vars.iter().map(|s| s.as_str()).collect();
            let (opt_val, opt_grads) = optimize_gradient(&expr, &var_refs, &sat_cfg);
            let mut all_exprs = Vec::with_capacity(1 + opt_grads.len());
            all_exprs.push(opt_val.clone());
            all_exprs.extend(opt_grads.clone());

            let cse = eliminate_common_subexprs(&all_exprs);
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
            let val_ts = maxima_rs::codegen::emit_expr(&cse.outputs[0], &cfg);
            let grad_elems = cse.outputs[1..]
                .iter()
                .map(|e| maxima_rs::codegen::emit_expr(e, &cfg).to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let params = vars
                .iter()
                .map(|v| format!("{}: {}", v, ty))
                .collect::<Vec<_>>()
                .join(", ");
            let n = vars.len();
            let code = format!(
                "pub fn compute_gradient({params}) -> ({ty}, [{ty}; {n}]) {{\n{bindings}        ({val_ts}, [{grad_elems}])\n    }}"
            );
            resp.rust_code = Some(code.clone());
            let res_str = format!(
                "value: {}, gradient: [{}]",
                opt_val,
                opt_grads
                    .iter()
                    .map(|g| g.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let res_latex = format!(
                "f = {}, \\quad \\nabla f = \\begin{{pmatrix}} {} \\end{{pmatrix}}",
                latex(&opt_val),
                opt_grads
                    .iter()
                    .map(latex)
                    .collect::<Vec<_>>()
                    .join(" \\\\ ")
            );
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.into());
            resp.extra.insert("result_latex".into(), res_latex.into());
            resp.outputs = vec![Output::new(
                "rust_gradient",
                code,
                resp.result_latex.clone(),
            )];
            resp.status = Status::Ok;
        }
        "evaluate" => {
            let env = parse_env(&req.env).map_err(|e| (e, None))?;
            let val = expr.eval(&env).ok_or_else(|| {
                (
                    "evaluation failed (undefined variable or domain error)".to_string(),
                    None,
                )
            })?;
            let res_str = format!("{:.10}", val);
            let res_latex = latex_const(val);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("value", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "determinant" => {
            let mat = Matrix::from_expr(&expr).map_err(|e| (e.to_string(), None))?;
            let det = determinant(&mat).map_err(|e| (e.to_string(), None))?;
            let res_str = det.to_string();
            let res_latex = latex(&det);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("determinant", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "invert" => {
            let mat = Matrix::from_expr(&expr).map_err(|e| (e.to_string(), None))?;
            let inv = invert(&mat).map_err(|e| (e.to_string(), None))?;
            let inv_expr = inv.to_expr();
            let res_str = inv_expr.to_string();
            let res_latex = latex(&inv_expr);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("invert", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "charpoly" => {
            let mat = Matrix::from_expr(&expr).map_err(|e| (e.to_string(), None))?;
            let v = param_var(req, "variable", &[]).unwrap_or_else(|_| "%lambda".to_string());
            let cp = charpoly(&mat, &v).map_err(|e| (e.to_string(), None))?;
            let res_str = cp.to_string();
            let res_latex = latex(&cp);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new(format!("charpoly({v})"), res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "trace" => {
            let mat = Matrix::from_expr(&expr).map_err(|e| (e.to_string(), None))?;
            let tr = trace(&mat).map_err(|e| (e.to_string(), None))?;
            let res_str = tr.to_string();
            let res_latex = latex(&tr);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("trace", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "eigenvalues" => {
            let mat = Matrix::from_expr(&expr).map_err(|e| (e.to_string(), None))?;
            let evs = eigenvalues(&mat).map_err(|e| (e.to_string(), None))?;
            let res_str = format!(
                "[{}]",
                evs.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let res_latex = format!(
                "\\left[ {} \\right]",
                evs.iter().map(latex).collect::<Vec<_>>().join(", ")
            );
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("eigenvalues", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "rectform" => {
            let res = rectform_expr(&expr);
            let res_str = res.to_string();
            let res_latex = latex(&res);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("rectform", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "polarform" => {
            let res = polarform_expr(&expr);
            let res_str = res.to_string();
            let res_latex = latex(&res);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("polarform", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "realpart" => {
            let res = realpart(&expr);
            let res_str = res.to_string();
            let res_latex = latex(&res);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("realpart", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "imagpart" => {
            let res = imagpart(&expr);
            let res_str = res.to_string();
            let res_latex = latex(&res);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("imagpart", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "conjugate" => {
            let res = conjugate(&expr);
            let res_str = res.to_string();
            let res_latex = latex(&res);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("conjugate", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "carg" => {
            let res = carg(&expr);
            let res_str = res.to_string();
            let res_latex = latex(&res);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("carg", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "cabs" => {
            let res = cabs(&expr);
            let res_str = res.to_string();
            let res_latex = latex(&res);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new("cabs", res_str, res_latex)];
            resp.status = Status::Ok;
        }
        "residue" => {
            let v = param_var(req, "variable", &[]).unwrap_or_else(|_| "z".to_string());
            let pt = param_point(req, "point", Some("0")).map_err(|e| (e, None))?;
            let res = residue(&expr, &v, &pt).map_err(|e| (e, None))?;
            let res_str = res.to_string();
            let res_latex = latex(&res);
            resp.result = res_str.clone();
            resp.result_latex = res_latex.clone();
            resp.extra.insert("result".into(), res_str.clone().into());
            resp.extra
                .insert("result_latex".into(), res_latex.clone().into());
            resp.outputs = vec![Output::new(
                format!("residue({v}={pt})"),
                res_str,
                res_latex,
            )];
            resp.status = Status::Ok;
        }
        other => return Err((format!("unknown operation: {other}"), None)),
    }

    resp.elapsed_ms = start.elapsed().as_millis() as u64;
    Ok(())
}

/// Compute a request, producing a [`ComputeResponse`].
pub fn compute(req: &ComputeRequest) -> ComputeResponse {
    let mut resp = ComputeResponse::ok();
    resp.operation = req.operation.clone();
    if let Err((e, span)) = run_operation(req, &mut resp) {
        resp.success = false;
        resp.status = Status::Error;
        resp.error = Some(e);
        resp.error_span = span;
    }
    resp
}

/// Parse an expression and return live typeset and detected variables.
pub fn parse(req: &ParseRequest) -> ParseResponse {
    let parsed = match req.input_format.as_str() {
        "latex" => parse_latex(&req.expression),
        _ => {
            if let Ok(cmd) = maxima_rs::parser::maxima::parse_maxima_cas(&req.expression) {
                match cmd {
                    maxima_rs::parser::maxima::CasCommand::Defint(e, v, a, b) => {
                        let latex_str = format!(
                            "\\int_{{{}}}^{{{}}} {} \\, d{}",
                            latex(&a),
                            latex(&b),
                            latex(&e),
                            v
                        );
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("defint({}, {}, {}, {})", e, v, a, b)),
                            variables: vec![v],
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Integrate(e, v) => {
                        let latex_str = format!("\\int {} \\, d{}", latex(&e), v);
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("integrate({}, {})", e, v)),
                            variables: vec![v],
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Limit(e, v, pt) => {
                        let latex_str =
                            format!("\\lim_{{{} \\to {}}} {}", v, latex(&pt), latex(&e));
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("limit({}, {}, {})", e, v, pt)),
                            variables: vec![v],
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Taylor(e, v, c, n) => {
                        let latex_str = format!(
                            "\\text{{taylor}}\\left({}, {}, {}, {}\\right)",
                            latex(&e),
                            v,
                            latex(&c),
                            n
                        );
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("taylor({}, {}, {}, {})", e, v, c, n)),
                            variables: vec![v],
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Solve(e, v) => {
                        let latex_str =
                            format!("\\text{{solve}}\\left({} = 0, {}\\right)", latex(&e), v);
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("solve({}, {})", e, v)),
                            variables: vec![v],
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Partfrac(e, v) => {
                        let latex_str =
                            format!("\\text{{partfrac}}\\left({}, {}\\right)", latex(&e), v);
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("partfrac({}, {})", e, v)),
                            variables: vec![v],
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Ode2(e, dep, indep) => {
                        let latex_str = format!(
                            "\\text{{ode2}}\\left({}, {}, {}\\right)",
                            latex(&e),
                            dep,
                            indep
                        );
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("ode2({}, {}, {})", e, dep, indep)),
                            variables: vec![dep, indep],
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Expand(e) => {
                        let latex_str = format!("\\text{{expand}}\\left({}\\right)", latex(&e));
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("expand({})", e)),
                            variables: e.variables(),
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Factor(e) => {
                        let latex_str = format!("\\text{{factor}}\\left({}\\right)", latex(&e));
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("factor({})", e)),
                            variables: e.variables(),
                            error: None,
                            error_span: None,
                        };
                    }
                    maxima_rs::parser::maxima::CasCommand::Ratsimp(e) => {
                        let latex_str = format!("\\text{{ratsimp}}\\left({}\\right)", latex(&e));
                        return ParseResponse {
                            ok: true,
                            latex: Some(latex_str),
                            maxima: Some(format!("ratsimp({})", e)),
                            variables: e.variables(),
                            error: None,
                            error_span: None,
                        };
                    }
                }
            } else {
                parse_maxima(&req.expression)
            }
        }
    };

    match parsed {
        Ok(expr) => ParseResponse {
            ok: true,
            latex: Some(latex(&expr)),
            maxima: Some(expr.to_string()),
            variables: expr.variables(),
            error: None,
            error_span: None,
        },
        Err(e) => {
            let span = extract_error_span(&e, &req.expression);
            ParseResponse {
                ok: false,
                latex: None,
                maxima: None,
                variables: vec![],
                error: Some(format!("{e}")),
                error_span: Some(span),
            }
        }
    }
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

fn extract_error_span(err: &ParseError, input: &str) -> ErrorSpan {
    match err {
        ParseError::Syntax { remaining, .. } => {
            let start = input.len().saturating_sub(remaining.len());
            ErrorSpan {
                start,
                end: input.len(),
            }
        }
        ParseError::UnexpectedToken { token, position } => {
            let start = *position;
            let end = (start + token.len()).min(input.len());
            ErrorSpan { start, end }
        }
        ParseError::EmptyInput => ErrorSpan { start: 0, end: 0 },
        ParseError::RecursionLimitExceeded => ErrorSpan {
            start: 0,
            end: input.len(),
        },
    }
}

fn parse_input(req: &ComputeRequest) -> Result<Expr, (String, Option<ErrorSpan>)> {
    match req.input_format.as_str() {
        "latex" => parse_latex(&req.expression).map_err(|e| {
            let span = extract_error_span(&e, &req.expression);
            (format!("LaTeX parse error: {e}"), Some(span))
        }),
        _ => {
            if let Ok(cmd) = maxima_rs::parser::maxima::parse_maxima_cas(&req.expression) {
                match cmd {
                    maxima_rs::parser::maxima::CasCommand::Defint(e, _, _, _) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Integrate(e, _) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Taylor(e, _, _, _) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Solve(e, _) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Partfrac(e, _) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Limit(e, _, _) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Expand(e) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Factor(e) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Ratsimp(e) => Ok(e),
                    maxima_rs::parser::maxima::CasCommand::Ode2(e, _, _) => Ok(e),
                }
            } else {
                parse_maxima(&req.expression).map_err(|e| {
                    let span = extract_error_span(&e, &req.expression);
                    (format!("Maxima parse error: {e}"), Some(span))
                })
            }
        }
    }
}

fn validate_request_bounds(req: &ComputeRequest) -> Result<(), String> {
    if req.variables.len() > 16 {
        return Err(format!(
            "too many variables ({}; maximum is 16)",
            req.variables.len()
        ));
    }
    if req.env.len() > 64 {
        return Err(format!(
            "too many environment bindings ({}; maximum is 64)",
            req.env.len()
        ));
    }
    if req.params.len() > 32 {
        return Err(format!(
            "too many parameters ({}; maximum is 32)",
            req.params.len()
        ));
    }
    for (k, v) in &req.params {
        if k.len() > 64 {
            return Err(format!("parameter name '{k}' exceeds 64 bytes"));
        }
        if let Some(s) = v.as_str()
            && s.len() > 1024
        {
            return Err(format!("parameter '{k}' exceeds 1024 bytes"));
        }
    }
    Ok(())
}

fn saturation_config(req: &ComputeRequest) -> SaturationConfig {
    SaturationConfig {
        rule_config: RuleConfig {
            fast_math: req.fast_math,
            ..RuleConfig::default()
        },
        ..SaturationConfig::default()
    }
}

fn is_bracket_vector(input: &str) -> bool {
    let trimmed = input.trim();
    trimmed.starts_with('[') && trimmed.ends_with(']')
}

fn parse_bracket_vector(input: &str) -> Result<Vec<Expr>, (String, Option<ErrorSpan>)> {
    let trimmed = input.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or("")
        .trim();
    if inner.is_empty() {
        return Err(("empty vector expression '[]'".into(), None));
    }
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in inner.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(inner[start..idx].trim());
                start = idx + 1;
            }
            _ => {}
        }
    }
    parts.push(inner[start..].trim());
    let mut exprs = Vec::with_capacity(parts.len());
    for part in parts {
        if part.is_empty() {
            return Err(("empty element in vector expression".into(), None));
        }
        let e = parse_maxima(part).map_err(|e| {
            let span = extract_error_span(&e, part);
            (
                format!("Maxima parse error in vector element: {e}"),
                Some(span),
            )
        })?;
        exprs.push(e);
    }
    Ok(exprs)
}

fn is_rust_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "try"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
    )
}

fn validate_rust_idents(vars: &[String]) -> Result<(), String> {
    for v in vars {
        let mut chars = v.chars();
        let Some(first) = chars.next() else {
            return Err("empty Rust identifier is not allowed".into());
        };
        if !(first.is_ascii_alphabetic() || first == '_') || v == "_" {
            return Err(format!("invalid Rust identifier '{v}'"));
        }
        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("invalid Rust identifier '{v}'"));
        }
        if is_rust_keyword(v) {
            return Err(format!(
                "Rust keyword '{v}' cannot be used as an identifier"
            ));
        }
    }
    Ok(())
}

fn get_str_param<'a>(req: &'a ComputeRequest, key: &str) -> Option<&'a str> {
    let val = req.params.get(key)?;
    let s = val.as_str()?;
    let trimmed = s.trim();
    if !trimmed.is_empty() {
        Some(trimmed)
    } else {
        None
    }
}

fn param_var(req: &ComputeRequest, key: &str, detected: &[String]) -> Result<String, String> {
    if let Some(v) = get_str_param(req, key) {
        return Ok(v.to_string());
    }
    if key == "variable" {
        if !req.variable.trim().is_empty() {
            return Ok(req.variable.trim().to_string());
        }
        if !req.variables.is_empty() {
            return Ok(req.variables[0].trim().to_string());
        }
        if let Some(first) = detected.first() {
            return Ok(first.clone());
        }
    }
    Err(format!(
        "missing required parameter '{key}' for operation '{}'",
        req.operation
    ))
}

fn parse_point_str(val_str: &str, input_format: &str) -> Result<Expr, String> {
    match val_str.trim() {
        "inf" | "\\infty" | "+inf" | "+infinity" => Ok(Expr::var("inf")),
        "minf" | "-\\infty" | "-inf" | "-infinity" => Ok(Expr::var("minf")),
        other => {
            let parsed = match input_format {
                "latex" => parse_latex(other)
                    .map_err(|e| format!("invalid point (LaTeX parse error: {e})"))?,
                _ => parse_maxima(other)
                    .map_err(|e| format!("invalid point (Maxima parse error: {e})"))?,
            };
            Ok(fold_const_expr(parsed))
        }
    }
}

fn param_point(req: &ComputeRequest, key: &str, default: Option<&str>) -> Result<Expr, String> {
    let val_str = if let Some(v) = get_str_param(req, key) {
        v
    } else if let Some(d) = default {
        d
    } else {
        return Err(format!(
            "missing required parameter '{key}' for operation '{}'",
            req.operation
        ));
    };

    parse_point_str(val_str, &req.input_format).map_err(|e| format!("{e} for '{key}'"))
}

fn fold_const_expr(e: Expr) -> Expr {
    use num_traits::Zero;
    match e {
        Expr::Const(_) => e,
        Expr::Neg(a) => {
            let a = fold_const_expr(*a);
            match a {
                Expr::Const(r) => Expr::constant(-r),
                other => Expr::Neg(Box::new(other)),
            }
        }
        Expr::Add(a, b) => {
            let a = fold_const_expr(*a);
            let b = fold_const_expr(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x + y),
                _ => Expr::Add(Box::new(a), Box::new(b)),
            }
        }
        Expr::Sub(a, b) => {
            let a = fold_const_expr(*a);
            let b = fold_const_expr(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x - y),
                _ => Expr::Sub(Box::new(a), Box::new(b)),
            }
        }
        Expr::Mul(a, b) => {
            let a = fold_const_expr(*a);
            let b = fold_const_expr(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x * y),
                _ => Expr::Mul(Box::new(a), Box::new(b)),
            }
        }
        Expr::Div(a, b) => {
            let a = fold_const_expr(*a);
            let b = fold_const_expr(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) if !y.is_zero() => Expr::constant(x / y),
                _ => Expr::Div(Box::new(a), Box::new(b)),
            }
        }
        other => other,
    }
}

fn param_int(req: &ComputeRequest, key: &str, default: i64) -> Result<i64, String> {
    if let Some(val) = req.params.get(key) {
        if let Some(n) = val.as_i64() {
            return Ok(n);
        }
        if let Some(n) = val
            .as_str()
            .map(str::trim)
            .and_then(|s| s.parse::<i64>().ok())
        {
            return Ok(n);
        }
        return Err(format!("param '{key}' must be an integer, got {val}"));
    }
    Ok(default)
}

fn resolve_vars(req: &ComputeRequest, detected: &[String]) -> Result<Vec<String>, String> {
    let vars: Vec<String> = if !req.variables.is_empty() {
        req.variables.clone()
    } else if let Some(val) = req.params.get("variables") {
        if let Some(arr) = val.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect()
        } else if let Some(s) = val.as_str() {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    } else if !req.variable.trim().is_empty() {
        vec![req.variable.trim().to_string()]
    } else if let Some(v) = get_str_param(req, "variable") {
        vec![v.to_string()]
    } else {
        detected.to_vec()
    };

    if vars.len() > 16 {
        return Err(format!(
            "too many variables ({}; maximum is 16)",
            vars.len()
        ));
    }
    Ok(vars)
}

fn parse_float_type(req: &ComputeRequest) -> Result<FloatType, String> {
    let raw = get_str_param(req, "float_type").unwrap_or(req.float_type.as_str());
    match raw.trim() {
        "f64" | "" => Ok(FloatType::F64),
        "f32" => Ok(FloatType::F32),
        other => Err(format!(
            "invalid float_type '{other}'; expected 'f64' or 'f32'"
        )),
    }
}

fn parse_env(env: &HashMap<String, String>) -> Result<HashMap<String, f64>, String> {
    let mut out = HashMap::new();
    for (k, v) in env {
        let f: f64 = v
            .parse()
            .map_err(|_| format!("invalid numeric value for variable '{k}': {v}"))?;
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

fn verify_integral(integrand: &Expr, antideriv: &Expr, var: &str) -> bool {
    let deriv = differentiate(antideriv, var);
    let test_points = [0.25, 0.5, 0.75, 1.25];
    let mut tested = 0;
    for &pt in &test_points {
        let mut env = HashMap::new();
        env.insert(var.to_string(), pt);
        if let (Some(v1), Some(v2)) = (integrand.eval(&env), deriv.eval(&env))
            && v1.is_finite()
            && v2.is_finite()
        {
            if (v1 - v2).abs() > 1e-3 {
                return false;
            }
            tested += 1;
        }
    }
    tested > 0
}

fn verify_root(expr: &Expr, var: &str, root: &Expr) -> bool {
    let empty_env = HashMap::new();
    if let Some(r_num) = root.eval(&empty_env) {
        let mut env = HashMap::new();
        env.insert(var.to_string(), r_num);
        if let Some(val) = expr.eval(&env) {
            return val.abs() < 1e-4;
        }
    }
    let sub = substitute(expr, var, root);
    let opt = optimize(&sub, &SaturationConfig::default());
    opt.is_zero()
}

fn substitute(e: &Expr, var: &str, rep: &Expr) -> Expr {
    match e {
        Expr::Var(v) if v == var => rep.clone(),
        Expr::Add(a, b) => Expr::add(substitute(a, var, rep), substitute(b, var, rep)),
        Expr::Sub(a, b) => Expr::sub(substitute(a, var, rep), substitute(b, var, rep)),
        Expr::Mul(a, b) => Expr::mul(substitute(a, var, rep), substitute(b, var, rep)),
        Expr::Div(a, b) => Expr::div(substitute(a, var, rep), substitute(b, var, rep)),
        Expr::Pow(a, b) => Expr::pow(substitute(a, var, rep), substitute(b, var, rep)),
        Expr::Neg(a) => Expr::neg(substitute(a, var, rep)),
        Expr::Sin(a) => Expr::sin(substitute(a, var, rep)),
        Expr::Cos(a) => Expr::cos(substitute(a, var, rep)),
        Expr::Tan(a) => Expr::tan(substitute(a, var, rep)),
        Expr::Exp(a) => Expr::exp(substitute(a, var, rep)),
        Expr::Ln(a) => Expr::ln(substitute(a, var, rep)),
        Expr::Sqrt(a) => Expr::sqrt(substitute(a, var, rep)),
        Expr::Sinh(a) => Expr::sinh(substitute(a, var, rep)),
        Expr::Cosh(a) => Expr::cosh(substitute(a, var, rep)),
        Expr::Tanh(a) => Expr::tanh(substitute(a, var, rep)),
        Expr::Asin(a) => Expr::asin(substitute(a, var, rep)),
        Expr::Acos(a) => Expr::acos(substitute(a, var, rep)),
        Expr::Atan(a) => Expr::atan(substitute(a, var, rep)),
        Expr::Factorial(a) => Expr::factorial(substitute(a, var, rep)),
        other => other.clone(),
    }
}

fn eval_bound(e: &Expr) -> Option<f64> {
    let empty = HashMap::new();
    e.eval(&empty)
}

fn is_pos_inf(e: &Expr) -> bool {
    match e {
        Expr::Var(s) => s == "inf" || s == "infinity",
        _ => false,
    }
}

fn composite_simpson(integrand: &Expr, var: &str, a: f64, b: f64, n: usize) -> Option<f64> {
    if n == 0 || !n.is_multiple_of(2) {
        return None;
    }
    let h = (b - a) / (n as f64);
    let mut env = HashMap::new();

    env.insert(var.to_string(), a);
    let fa = integrand.eval(&env)?;
    if !fa.is_finite() {
        return None;
    }

    env.insert(var.to_string(), b);
    let fb = integrand.eval(&env)?;
    if !fb.is_finite() {
        return None;
    }

    let mut sum = fa + fb;
    for i in 1..n {
        let x = a + (i as f64) * h;
        env.insert(var.to_string(), x);
        let fx = integrand.eval(&env)?;
        if !fx.is_finite() {
            return None;
        }
        if i % 2 == 1 {
            sum += 4.0 * fx;
        } else {
            sum += 2.0 * fx;
        }
    }

    Some(sum * h / 3.0)
}

fn improper_quad_inf(integrand: &Expr, var: &str, a: f64) -> Option<f64> {
    let n = 200;
    let t_min = 0.0005;
    let t_max = 0.9995;
    let dt = (t_max - t_min) / (n as f64);
    let mut sum = 0.0;
    let mut env = HashMap::new();

    for i in 0..n {
        let t = t_min + (i as f64 + 0.5) * dt;
        let one_minus_t = 1.0 - t;
        let x = a + t / one_minus_t;
        let dx_dt = 1.0 / (one_minus_t * one_minus_t);

        env.insert(var.to_string(), x);
        let fx = integrand.eval(&env)?;
        if !fx.is_finite() {
            return None;
        }
        sum += fx * dx_dt * dt;
    }

    Some(sum)
}

fn verify_defint(integrand: &Expr, result: &Expr, var: &str, a: &Expr, b: &Expr) -> bool {
    if let (Some(a_num), Some(b_num)) = (eval_bound(a), eval_bound(b)) {
        if (b_num - a_num).abs() < 1e-12
            && let Some(res_num) = eval_bound(result)
        {
            return res_num.abs() < 1e-6;
        }
        if a_num < b_num
            && let Some(num_int) = composite_simpson(integrand, var, a_num, b_num, 100)
            && let Some(res_num) = eval_bound(result)
        {
            let diff = (num_int - res_num).abs();
            let scale = 1.0 + res_num.abs();
            return diff / scale < 2e-2;
        } else if a_num > b_num
            && let Some(num_int) = composite_simpson(integrand, var, b_num, a_num, 100)
            && let Some(res_num) = eval_bound(result)
        {
            let diff = (-num_int - res_num).abs();
            let scale = 1.0 + res_num.abs();
            return diff / scale < 2e-2;
        }
    } else if is_pos_inf(b)
        && let Some(a_num) = eval_bound(a)
        && let Some(num_int) = improper_quad_inf(integrand, var, a_num)
        && let Some(res_num) = eval_bound(result)
    {
        let diff = (num_int - res_num).abs();
        let scale = 1.0 + res_num.abs();
        return diff / scale < 2e-2;
    }
    false
}

fn substitute_ode(e: &Expr, y: &Expr, yp: &Expr, ypp: &Expr, dep: &str, indep: &str) -> Expr {
    match e {
        Expr::Derivative(inner, var) => {
            if let Expr::Var(v) = var.as_ref()
                && v == indep
            {
                match inner.as_ref() {
                    Expr::Var(n) if n == dep => return yp.clone(),
                    Expr::Derivative(i2, v2) => {
                        if let Expr::Var(vv) = v2.as_ref()
                            && vv == indep
                            && matches!(i2.as_ref(), Expr::Var(n) if n == dep)
                        {
                            return ypp.clone();
                        }
                    }
                    _ => {}
                }
            }
            Expr::Derivative(
                Box::new(substitute_ode(inner, y, yp, ypp, dep, indep)),
                var.clone(),
            )
        }
        Expr::Var(n) if n == dep => y.clone(),
        Expr::Add(a, b) => Expr::add(
            substitute_ode(a, y, yp, ypp, dep, indep),
            substitute_ode(b, y, yp, ypp, dep, indep),
        ),
        Expr::Sub(a, b) => Expr::sub(
            substitute_ode(a, y, yp, ypp, dep, indep),
            substitute_ode(b, y, yp, ypp, dep, indep),
        ),
        Expr::Mul(a, b) => Expr::mul(
            substitute_ode(a, y, yp, ypp, dep, indep),
            substitute_ode(b, y, yp, ypp, dep, indep),
        ),
        Expr::Div(a, b) => Expr::div(
            substitute_ode(a, y, yp, ypp, dep, indep),
            substitute_ode(b, y, yp, ypp, dep, indep),
        ),
        Expr::Pow(a, b) => Expr::pow(
            substitute_ode(a, y, yp, ypp, dep, indep),
            substitute_ode(b, y, yp, ypp, dep, indep),
        ),
        Expr::Neg(a) => Expr::neg(substitute_ode(a, y, yp, ypp, dep, indep)),
        other => other.clone(),
    }
}

fn verify_ode(ode_expr: &Expr, solution: &Expr, dep: &str, indep: &str) -> bool {
    let yp = differentiate(solution, indep);
    let ypp = differentiate(&yp, indep);
    let residual = substitute_ode(ode_expr, solution, &yp, &ypp, dep, indep);

    let test_points = [0.2, 0.5, 0.8];
    let mut tested = 0;
    for &pt in &test_points {
        let mut env = HashMap::new();
        env.insert(indep.to_string(), pt);
        env.insert("_C1".to_string(), 1.0);
        env.insert("_C2".to_string(), 1.0);
        env.insert("C1".to_string(), 1.0);
        env.insert("C2".to_string(), 1.0);
        for v in residual.variables() {
            env.entry(v).or_insert(1.0);
        }
        if let Some(val) = residual.eval(&env)
            && val.is_finite()
        {
            if val.abs() > 1e-3 {
                return false;
            }
            tested += 1;
        }
    }
    tested > 0
}

fn subscript(i: usize) -> String {
    let s = i.to_string();
    s.chars()
        .map(|c| match c {
            '0' => '₀',
            '1' => '₁',
            '2' => '₂',
            '3' => '₃',
            '4' => '₄',
            '5' => '₅',
            '6' => '₆',
            '7' => '₇',
            '8' => '₈',
            '9' => '₉',
            _ => c,
        })
        .collect()
}

fn format_matrix_string(m: &[Vec<Expr>]) -> String {
    let rows: Vec<String> = m
        .iter()
        .map(|r| {
            let cells: Vec<String> = r.iter().map(|c| c.to_string()).collect();
            format!("[{}]", cells.join(", "))
        })
        .collect();
    format!("[{}]", rows.join(", "))
}

fn format_matrix_latex(m: &[Vec<Expr>]) -> String {
    let rows: Vec<String> = m
        .iter()
        .map(|r| {
            let cells: Vec<String> = r.iter().map(latex).collect();
            cells.join(" & ")
        })
        .collect();
    format!(
        "\\begin{{pmatrix}} {} \\end{{pmatrix}}",
        rows.join(" \\\\ ")
    )
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CseBinding {
    pub name: String,
    pub expr: String,
    pub latex: String,
}

pub fn latex(e: &Expr) -> String {
    maxima_rs::to_latex(e)
}

fn latex_const(v: f64) -> String {
    if v == v.floor() && v.is_finite() {
        format!("{}", v as i64)
    } else {
        format!("{:.6}", v)
    }
}

/// Catalog of composable functions/operators for the webUI recursive composer.
pub fn functions_catalog() -> crate::api::FunctionCatalog {
    use crate::api::{FunctionCatalog, FunctionCatalogEntry};

    let mut functions = Vec::new();

    for name in [
        "sin", "cos", "tan", "sinh", "cosh", "tanh", "asin", "acos", "atan", "exp", "ln", "sqrt",
    ] {
        functions.push(FunctionCatalogEntry {
            name: name.to_string(),
            arity: 1,
            kind: "unary",
            maxima_template: format!("{name}({{}})"),
            latex_template: format!("\\{name}({{}})"),
        });
    }
    functions.push(FunctionCatalogEntry {
        name: "factorial".to_string(),
        arity: 1,
        kind: "unary",
        maxima_template: "({})!".to_string(),
        latex_template: "({})!".to_string(),
    });
    for op in ["+", "-", "*", "/", "^"] {
        functions.push(FunctionCatalogEntry {
            name: op.to_string(),
            arity: 2,
            kind: "binary",
            maxima_template: format!("({{}}) {op} ({{}})"),
            latex_template: format!("({{}}) {op} ({{}})"),
        });
    }
    for name in ["expand", "factor", "ratsimp"] {
        functions.push(FunctionCatalogEntry {
            name: name.to_string(),
            arity: 1,
            kind: "cas",
            maxima_template: format!("{name}({{}})"),
            latex_template: format!("{name}({{}})"),
        });
    }
    for name in ["diff", "integrate", "solve"] {
        functions.push(FunctionCatalogEntry {
            name: name.to_string(),
            arity: 2,
            kind: "cas",
            maxima_template: format!("{name}({{}}, x)"),
            latex_template: if name == "diff" {
                "\\frac{d}{dx}({})".to_string()
            } else {
                format!("{name}({{}}, x)")
            },
        });
    }
    functions.push(FunctionCatalogEntry {
        name: "limit".to_string(),
        arity: 3,
        kind: "cas",
        maxima_template: "limit({}, x, 0)".to_string(),
        latex_template: "\\lim_{x \\to 0} {}".to_string(),
    });
    functions.push(FunctionCatalogEntry {
        name: "defint".to_string(),
        arity: 4,
        kind: "cas",
        maxima_template: "defint({}, x, 0, 1)".to_string(),
        latex_template: "\\int_{0}^{1} {} \\, dx".to_string(),
    });
    functions.push(FunctionCatalogEntry {
        name: "ode2".to_string(),
        arity: 3,
        kind: "cas",
        maxima_template: "ode2({}, y, x)".to_string(),
        latex_template: "ode2({}, y, x)".to_string(),
    });
    functions.push(FunctionCatalogEntry {
        name: "taylor".to_string(),
        arity: 2,
        kind: "cas",
        maxima_template: "taylor({}, x, 0, 6)".to_string(),
        latex_template: "taylor({}, x, 0, 6)".to_string(),
    });

    FunctionCatalog { functions }
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
            fast_math: true,
            env: HashMap::new(),
            params: HashMap::new(),
        }
    }

    fn result_of(resp: &ComputeResponse) -> &str {
        &resp.result
    }

    #[test]
    fn test_simplify_operation() {
        let resp = compute(&request("sin(x)^2 + cos(x)^2", "simplify"));
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(result_of(&resp), "1");
        assert_eq!(resp.status, Status::Ok);
    }

    #[test]
    fn test_optimize_alias_operation() {
        let resp = compute(&request("x + 0", "optimize"));
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(result_of(&resp), "x");
        assert_eq!(resp.status, Status::Ok);
    }

    #[test]
    fn test_expand_operation() {
        let resp = compute(&request("(x + y)^2", "expand"));
        assert!(resp.success, "error: {:?}", resp.error);
        assert!(resp.result.contains("x") && resp.result.contains("y"));
        assert_eq!(resp.status, Status::Ok);
    }

    #[test]
    fn test_factor_operation() {
        let resp = compute(&request("x^2 - 1", "factor"));
        assert!(resp.success, "error: {:?}", resp.error);
        assert!(resp.result.contains("(x - 1)") || resp.result.contains("(x + 1)"));
        assert_eq!(resp.status, Status::Ok);
    }

    #[test]
    fn test_ratsimp_operation() {
        let resp = compute(&request("(x^2 - 1)/(x - 1)", "ratsimp"));
        assert!(resp.success, "error: {:?}", resp.error);
        assert!(resp.result.contains("x + 1"));
        assert_eq!(resp.status, Status::Ok);
    }

    #[test]
    fn test_partfrac_operation() {
        let mut req = request("1/(x^2 - 1)", "partfrac");
        req.variable = "x".into();
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
    }

    #[test]
    fn test_integrate_operation() {
        let mut req = request("x^2", "integrate");
        req.variable = "x".into();
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.verified, Some(true));
    }

    #[test]
    fn test_integrate_unevaluated() {
        let mut req = request("exp(-x^2)", "integrate");
        req.variable = "x".into();
        let resp = compute(&req);
        assert!(resp.success);
        assert_eq!(resp.status, Status::Unevaluated);
        assert!(!resp.warnings.is_empty());
    }

    #[test]
    fn test_defint_operation() {
        let mut req = request("x^2", "defint");
        req.params.insert("variable".into(), serde_json::json!("x"));
        req.params.insert("lower".into(), serde_json::json!("0"));
        req.params.insert("upper".into(), serde_json::json!("1"));
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.result, "1/3");
    }

    #[test]
    fn test_defint_divergent() {
        let mut req = request("1/x^2", "defint");
        req.params.insert("variable".into(), serde_json::json!("x"));
        req.params.insert("lower".into(), serde_json::json!("-1"));
        req.params.insert("upper".into(), serde_json::json!("1"));
        let resp = compute(&req);
        assert!(resp.success);
        assert_eq!(resp.status, Status::Divergent);
    }

    #[test]
    fn test_defint_embedded_command() {
        let req = request("defint(1/x^2, x, 1, 2)", "defint");
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.result, "1/2");
    }

    #[test]
    fn test_cas_embedded_commands_dispatch() {
        // integrate(x^3, x)
        let resp = compute(&request("integrate(x^3, x)", "simplify"));
        assert!(resp.success, "integrate error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert!(resp.result.contains("x ^ 4") || resp.result.contains("x^4"));

        // limit(sin(x)/x, x, 0)
        let resp = compute(&request("limit(sin(x)/x, x, 0)", "simplify"));
        assert!(resp.success, "limit error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.result, "1");

        // taylor(sin(x), x, 0, 5)
        let resp = compute(&request("taylor(sin(x), x, 0, 5)", "simplify"));
        assert!(resp.success, "taylor error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);

        // solve(x^2 - 4, x)
        let resp = compute(&request("solve(x^2 - 4, x)", "simplify"));
        assert!(resp.success, "solve error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.outputs.len(), 2);

        // ode2(diff(y, x) + y, y, x)
        let resp = compute(&request("ode2(diff(y, x) + y, y, x)", "simplify"));
        assert!(resp.success, "ode2 error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert!(resp.extra.contains_key("constants"));

        // partfrac(1/(x^2 - 1), x)
        let resp = compute(&request("partfrac(1/(x^2 - 1), x)", "simplify"));
        assert!(resp.success, "partfrac error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
    }

    #[test]
    fn test_limit_operation() {
        let mut req = request("sin(x)/x", "limit");
        req.params.insert("variable".into(), serde_json::json!("x"));
        req.params.insert("point".into(), serde_json::json!("0"));
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.result, "1");
    }

    #[test]
    fn test_taylor_operation() {
        let mut req = request("sin(x)", "taylor");
        req.params.insert("variable".into(), serde_json::json!("x"));
        req.params.insert("order".into(), serde_json::json!(5));
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
    }

    #[test]
    fn test_solve_operation() {
        let mut req = request("x^2 - 1", "solve");
        req.variable = "x".into();
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.outputs.len(), 2);
        assert_eq!(resp.verified, Some(true));
    }

    #[test]
    fn test_solve_no_solution() {
        let mut req = request("x^2 + 1", "solve");
        req.variable = "x".into();
        let resp = compute(&req);
        assert!(resp.success);
        assert_eq!(resp.status, Status::NoSolution);
    }

    #[test]
    fn test_ode2_operation() {
        let mut req = request("diff(y, x) + y", "ode2");
        req.params
            .insert("dependent".into(), serde_json::json!("y"));
        req.params.insert("variable".into(), serde_json::json!("x"));
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.verified, Some(true));
        assert!(resp.extra.contains_key("constants"));
    }

    #[test]
    fn test_ode2_hard_stop_verification() {
        let mut req = request("diff(y, x) + 2*y", "ode2");
        req.params
            .insert("dependent".into(), serde_json::json!("y"));
        req.params.insert("variable".into(), serde_json::json!("x"));
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert_eq!(resp.verified, Some(true));
        assert!(
            resp.result.contains("_C1")
                && resp.result.contains("exp")
                && (resp.result.contains("-2") || resp.result.contains("(-(2 / 1))")),
            "resp.result was: {}",
            resp.result
        );
    }

    #[test]
    fn test_integrate_proven_non_elementary() {
        let mut req = request("exp(-x^2)", "integrate");
        req.params.insert("variable".into(), serde_json::json!("x"));
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Unevaluated);
        assert_eq!(resp.verified, Some(false));
        assert!(
            resp.warnings
                .iter()
                .any(|w| w.contains("proven non-elementary")),
            "expected proven non-elementary warning, got {:?}",
            resp.warnings
        );
    }

    #[test]
    fn test_solve_partial_status() {
        // x * (x^2 + 1) = x^3 + x has root x = 0, leaving irreducible x^2 + 1
        let mut req = request("x^3 + x", "solve");
        req.variable = "x".into();
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Partial);
        assert_eq!(resp.verified, Some(true));
        assert_eq!(resp.outputs.len(), 1);
        assert!(
            resp.warnings
                .iter()
                .any(|w| w.contains("partial factorization")),
            "expected partial factorization warning, got {:?}",
            resp.warnings
        );
    }

    #[test]
    fn test_taylor_nonzero_center() {
        let mut req = request("sin(x)", "taylor");
        req.params.insert("variable".into(), serde_json::json!("x"));
        req.params.insert("center".into(), serde_json::json!("1"));
        req.params.insert("order".into(), serde_json::json!(2));
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert!(
            resp.result.contains("O((x - 1)^3)"),
            "expected O((x - 1)^3) in result, got: {}",
            resp.result
        );
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
    fn test_rust_gradient_operation() {
        let mut req = request("x^2 + y^2", "rust_gradient");
        req.variables = vec!["x".into(), "y".into()];
        let resp = compute(&req);
        assert!(resp.success, "error: {:?}", resp.error);
        assert_eq!(resp.status, Status::Ok);
        assert!(resp.rust_code.is_some());
        assert!(resp.rust_code.unwrap().contains("compute_gradient"));
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
    fn test_parse_endpoint() {
        let req = ParseRequest {
            expression: "sin(x) + y".into(),
            input_format: "maxima".into(),
        };
        let resp = parse(&req);
        assert!(resp.ok);
        assert!(resp.variables.contains(&"x".to_string()));
        assert!(resp.variables.contains(&"y".to_string()));
        assert!(resp.latex.is_some());

        let bad_req = ParseRequest {
            expression: "sin(x +".into(),
            input_format: "maxima".into(),
        };
        let bad_resp = parse(&bad_req);
        assert!(!bad_resp.ok);
        assert!(bad_resp.error_span.is_some());
    }

    #[test]
    fn test_unknown_operation_is_error() {
        let resp = compute(&request("x", "does-not-exist"));
        assert!(!resp.success);
        assert_eq!(resp.status, Status::Error);
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
