//! High-level ergonomic APIs and fluent Pipeline builder.
//!
//! Provides single-call convenience functions:
//! - [`optimize`]: E-graph saturation + cost extraction on a single expression.
//! - [`optimize_gradient`]: Analytical gradient computation + joint saturation + extraction.
//!
//! Also provides the chainable [`Pipeline`] builder for end-to-end multi-format pipelines.

use proc_macro2::TokenStream;
use quote::quote;

use crate::codegen::{CodegenConfig, FloatType, emit_cse_block};
use crate::cse::{CseResult, eliminate_common_subexprs};
use crate::diff::gradient;
use crate::expr::Expr;
use crate::extract::{extract_all, extract_single};
use crate::saturate::{SaturationConfig, saturate};

#[cfg(feature = "parser")]
use crate::parser::{ParseError, parse_latex, parse_maxima};

/// High-level single expression optimization.
pub fn optimize(expr: &Expr, config: &SaturationConfig) -> Expr {
    let (egraph, roots) = saturate(std::slice::from_ref(expr), config);
    extract_single(&egraph, roots[0])
}

/// Compute analytical gradient and optimize both value and gradient components jointly.
pub fn optimize_gradient(
    expr: &Expr,
    vars: &[&str],
    config: &SaturationConfig,
) -> (Expr, Vec<Expr>) {
    let grads = gradient(expr, vars);
    let mut all_exprs = Vec::with_capacity(1 + grads.len());
    all_exprs.push(expr.clone());
    all_exprs.extend(grads);

    let (egraph, roots) = saturate(&all_exprs, config);
    let extracted = extract_all(&egraph, &roots);

    let opt_val = extracted[0].clone();
    let opt_grad = extracted[1..].to_vec();
    (opt_val, opt_grad)
}

/// Fluent builder for end-to-end symbolic workflows.
#[derive(Debug, Clone, Default)]
pub struct Pipeline {
    exprs: Vec<Expr>,
    config: SaturationConfig,
}

impl Pipeline {
    /// Create a new empty pipeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an expression directly into the pipeline.
    pub fn with_expr(mut self, expr: Expr) -> Self {
        self.exprs = vec![expr];
        self
    }

    /// Set multiple expressions into the pipeline.
    pub fn with_exprs(mut self, exprs: Vec<Expr>) -> Self {
        self.exprs = exprs;
        self
    }

    /// Parse an expression from Maxima string notation.
    #[cfg(feature = "parser")]
    pub fn from_maxima(mut self, s: &str) -> Result<Self, ParseError> {
        let e = parse_maxima(s)?;
        self.exprs = vec![e];
        Ok(self)
    }

    /// Parse an expression from LaTeX notation.
    #[cfg(feature = "parser")]
    pub fn from_latex(mut self, s: &str) -> Result<Self, ParseError> {
        let e = parse_latex(s)?;
        self.exprs = vec![e];
        Ok(self)
    }

    /// Differentiate the primary expression with respect to the given variables,
    /// adding the derivatives to the pipeline expressions.
    pub fn differentiate(mut self, vars: &[&str]) -> Self {
        if let Some(first) = self.exprs.first() {
            let grads = gradient(first, vars);
            self.exprs.extend(grads);
        }
        self
    }

    /// Configure saturation limits or rules.
    pub fn with_config(mut self, config: SaturationConfig) -> Self {
        self.config = config;
        self
    }

    /// Run equality saturation and cost-optimal extraction on all expressions in the pipeline.
    pub fn optimize(mut self) -> Self {
        if !self.exprs.is_empty() {
            let (egraph, roots) = saturate(&self.exprs, &self.config);
            self.exprs = extract_all(&egraph, &roots);
        }
        self
    }

    /// Run Common Subexpression Elimination on the optimized expressions.
    pub fn cse(&self) -> CseResult {
        eliminate_common_subexprs(&self.exprs)
    }

    /// Emit an optimized Rust function body TokenStream using CSE.
    pub fn to_rust_fn(&self, fn_name: &str, float_type: FloatType) -> TokenStream {
        let cse_res = self.cse();
        let codegen_cfg = CodegenConfig { float_type };
        let body = emit_cse_block(&cse_res, &codegen_cfg);
        let fn_ident = quote::format_ident!("{}", fn_name);
        let ty_tokens = float_type.type_tokens();

        quote! {
            pub fn #fn_ident() -> #ty_tokens {
                #body
            }
        }
    }

    /// Retrieve the resulting expressions.
    pub fn expressions(&self) -> &[Expr] {
        &self.exprs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_free_fn() {
        let x = Expr::var("x");
        let expr = Expr::add(x.clone(), Expr::int(0));
        let opt = optimize(&expr, &SaturationConfig::default());
        assert_eq!(opt, x);
    }

    #[test]
    fn test_optimize_gradient_free_fn() {
        // f(x) = x^2, df/dx = 2*x
        let x = Expr::var("x");
        let expr = Expr::pow(x.clone(), Expr::int(2));
        let (_val, grad) = optimize_gradient(&expr, &["x"], &SaturationConfig::default());
        assert_eq!(grad.len(), 1);
        let grad_str = format!("{}", grad[0]);
        assert!(grad_str.contains('x'));
    }

    #[test]
    fn test_pipeline_builder() {
        let pipeline = Pipeline::new()
            .with_expr(Expr::add(Expr::var("x"), Expr::int(0)))
            .optimize();
        assert_eq!(pipeline.expressions(), &[Expr::var("x")]);
    }
}
