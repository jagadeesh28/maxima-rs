//! API request/response models for the maxima-rs webUI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Incoming computation request from the webUI.
#[derive(Debug, Clone, Deserialize)]
pub struct ComputeRequest {
    /// The mathematical expression (Maxima syntax or LaTeX).
    pub expression: String,
    /// "maxima" (default) or "latex".
    #[serde(default = "default_input_format")]
    pub input_format: String,
    /// One of: simplify, parse_maxima, parse_latex, differentiate, gradient,
    /// jacobian, hessian, cse, rust_code, evaluate.
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
    /// Variable -> numeric value for the evaluate operation.
    #[serde(default)]
    pub env: HashMap<String, String>,
}

fn default_input_format() -> String {
    "maxima".into()
}

fn default_float_type() -> String {
    "f64".into()
}

/// Outgoing computation response.
#[derive(Debug, Clone, Serialize, Default)]
pub struct ComputeResponse {
    pub success: bool,
    pub operation: String,
    /// Primary result (display form).
    pub result: String,
    /// Primary result as LaTeX (KaTeX-rendered by the frontend).
    pub result_latex: String,
    /// Extra structured results serialized as JSON values.
    #[serde(default)]
    pub extra: serde_json::Map<String, serde_json::Value>,
    /// Rust source code (operation = "rust_code").
    pub rust_code: Option<String>,
    /// Error message when success == false.
    pub error: Option<String>,
    /// Server-side processing time in milliseconds.
    pub elapsed_ms: u64,
}

impl ComputeResponse {
    pub fn ok() -> Self {
        Self {
            success: true,
            ..Default::default()
        }
    }
}

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub crate_name: String,
}

/// List of available operations (returned by GET /api/operations).
#[derive(Serialize)]
pub struct Operation {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub needs_variables: bool,
    pub needs_env: bool,
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
    /// Function or operator name (`sin`, `expand`, `+`, ...).
    pub name: String,
    /// Number of operands/arguments (unary = 1, binary = 2, CAS = 1–3).
    pub arity: u8,
    /// Category: `"unary"`, `"binary"`, or `"cas"`.
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
