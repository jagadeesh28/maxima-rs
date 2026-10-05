//! syn-based parser for inline sym_eval! syntax.

use syn::Expr;
use syn::parse::{Parse, ParseStream};

use crate::parse_sym_fn::convert_syn_expr;

/// Parsed `sym_eval!` invocation.
pub struct SymEvalDef {
    pub expr: sym_core::expr::Expr,
}

impl Parse for SymEvalDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let syn_expr: Expr = input.parse()?;
        let expr = convert_syn_expr(&syn_expr)?;
        Ok(Self { expr })
    }
}
