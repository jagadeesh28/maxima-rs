//! API request/response models for the maxima-rs webUI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of a computation outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Ok,
    Partial,
    Unevaluated,
    Divergent,
    NoSolution,
    Error,
}

/// A single formatted output item from an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    pub label: String,
    pub result: String,
    pub result_latex: String,
}

impl Output {
    pub fn new(
        label: impl Into<String>,
        result: impl Into<String>,
        result_latex: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            result: result.into(),
            result_latex: result_latex.into(),
        }
    }
}

/// A byte span within an expression identifying an error location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorSpan {
    pub start: usize,
    pub end: usize,
}

/// Specification of a single parameter required or accepted by an operation.
#[derive(Debug, Clone, Serialize)]
pub struct ParamSpec {
    pub name: &'static str,
    #[serde(rename = "type")]
    pub param_type: &'static str,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepts: Option<&'static [&'static str]>,
}

/// Incoming computation request from the webUI.
#[derive(Debug, Clone, Deserialize)]
pub struct ComputeRequest {
    /// The mathematical expression (Maxima syntax or LaTeX).
    pub expression: String,
    /// "maxima" (default) or "latex".
    #[serde(default = "default_input_format")]
    pub input_format: String,
    /// One of: simplify, parse_maxima, parse_latex, differentiate, gradient,
    /// jacobian, hessian, cse, rust_code, evaluate, provenance, expand, factor,
    /// ratsimp, partfrac, integrate, defint, limit, taylor, solve, ode2, rust_gradient.
    pub operation: String,
    /// Variables for differentiation / gradient / jacobian / hessian / rust_code.
    #[serde(default)]
    pub variables: Vec<String>,
    /// Single variable shorthand (used by differentiate).
    #[serde(default)]
    pub variable: String,
    /// "f64" (default) or "f32" — target float type for codegen/eval.
    #[serde(default = "default_float_type")]
    pub float_type: String,
    /// Enable fast-math e-graph simplifications (default: true).
    #[serde(default = "default_fast_math")]
    pub fast_math: bool,
    /// Variable -> numeric value for the evaluate operation.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Typed parameter bag for operations (e.g. variable, lower, upper, order, center, dependent).
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

fn default_input_format() -> String {
    "maxima".into()
}

fn default_float_type() -> String {
    "f64".into()
}

fn default_fast_math() -> bool {
    true
}

/// Outgoing computation response.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ComputeResponse {
    pub success: bool,
    pub operation: String,
    pub status: Status,
    #[serde(default)]
    pub outputs: Vec<Output>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Primary result (display form).
    pub result: String,
    /// Primary result as LaTeX (KaTeX-rendered by the frontend).
    pub result_latex: String,
    /// Extra structured results serialized as JSON values (for backward compatibility).
    #[serde(default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
    /// Rust source code (operation = "rust_code" or "rust_gradient").
    pub rust_code: Option<String>,
    /// Error message when success == false.
    pub error: Option<String>,
    /// Error span location when parse or syntax error occurs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_span: Option<ErrorSpan>,
    /// Server-side processing time in milliseconds.
    pub elapsed_ms: u64,
}

impl ComputeResponse {
    pub fn ok() -> Self {
        Self {
            success: true,
            status: Status::Ok,
            ..Default::default()
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            status: Status::Error,
            error: Some(msg.into()),
            ..Default::default()
        }
    }
}

/// Incoming expression parse request for live typeset and variable detection.
#[derive(Debug, Clone, Deserialize)]
pub struct ParseRequest {
    pub expression: String,
    #[serde(default = "default_input_format")]
    pub input_format: String,
}

/// Outgoing expression parse response.
#[derive(Debug, Clone, Serialize)]
pub struct ParseResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxima: Option<String>,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_span: Option<ErrorSpan>,
}

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub crate_name: String,
}

/// List of available operations (returned by GET /api/operations).
#[derive(Debug, Clone, Serialize)]
pub struct Operation {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub needs_variables: bool,
    pub needs_env: bool,
    pub params: &'static [ParamSpec],
}

/// One provenanced rewrite rule exposed by `GET /api/rules`.
#[derive(Debug, Clone, Serialize)]
pub struct RuleEntry {
    pub name: &'static str,
    pub source_file: &'static str,
    pub source_fn: &'static str,
    pub identity: &'static str,
    pub fast_math_only: bool,
}

/// Number of rules citing a given Maxima Lisp file.
#[derive(Debug, Clone, Serialize)]
pub struct FileRuleCount {
    pub file: &'static str,
    pub count: usize,
}

/// Response payload for `GET /api/rules` (1:1 Maxima coverage report).
#[derive(Debug, Clone, Serialize)]
pub struct RulesResponse {
    pub total: usize,
    pub by_file: Vec<FileRuleCount>,
    pub rules: Vec<RuleEntry>,
}

/// One composable function/operator exposed by `GET /api/functions`.
#[derive(Debug, Clone, Serialize)]
pub struct FunctionCatalogEntry {
    /// Function or operator name (`sin`, `+`, ...).
    pub name: String,
    /// Number of operands/arguments (unary = 1, binary = 2).
    pub arity: u8,
    /// Category: `"unary"` or `"binary"`.
    pub kind: &'static str,
    /// Maxima-syntax composition template (placeholders `{}`).
    pub maxima_template: String,
    /// LaTeX-syntax composition template (placeholders `{}`).
    pub latex_template: String,
}

/// Response payload for `GET /api/functions` (composable function catalog).
#[derive(Debug, Clone, Serialize)]
pub struct FunctionCatalog {
    pub functions: Vec<FunctionCatalogEntry>,
}
