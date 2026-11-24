//! Rust code generation emitting zero-overhead TokenStreams from CSE results.
//!
//! Transforms [`CseResult`] or [`Expr`] into `proc_macro2::TokenStream` that
//! compiles into high-performance native Rust code without runtime dependencies.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[cfg(feature = "cse")]
use crate::cse::CseResult;
use crate::expr::Expr;
use crate::rational::Rat;

/// Floating-point target type for codegen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FloatType {
    /// 64-bit IEEE-754 double precision (default).
    #[default]
    F64,
    /// 32-bit IEEE-754 single precision.
    F32,
}

impl FloatType {
    /// Return the type identifier as a TokenStream (`f64` or `f32`).
    pub fn type_tokens(self) -> TokenStream {
        match self {
            FloatType::F64 => quote!(f64),
            FloatType::F32 => quote!(f32),
        }
    }
}

/// Codegen configuration options.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CodegenConfig {
    /// Target float type (default: `FloatType::F64`).
    pub float_type: FloatType,
}

/// Convert a single [`Expr`] directly into an inline expression [`TokenStream`].
pub fn emit_expr(expr: &Expr, config: &CodegenConfig) -> TokenStream {
    match expr {
        Expr::Var(name) => {
            let ident = format_ident!("{}", name);
            quote!(#ident)
        }
        Expr::Const(r) => {
            let lit_str = match config.float_type {
                FloatType::F64 => Rat::to_f64_literal(r),
                FloatType::F32 => Rat::to_f32_literal(r),
            };
            let lit_tokens: TokenStream = lit_str.parse().unwrap_or_else(|_| quote!(0.0));
            quote!(#lit_tokens)
        }
        #[cfg(feature = "bfloat")]
        Expr::Bfloat(b) => {
            let v = b.to_f64();
            let lit_str = match config.float_type {
                FloatType::F64 => format!("{v:?}"),
                FloatType::F32 => format!("{:?}f32", v as f32),
            };
            let lit_tokens: TokenStream = lit_str.parse().unwrap_or_else(|_| quote!(0.0));
            quote!(#lit_tokens)
        }
        Expr::Add(a, b) => {
            let ea = emit_expr(a, config);
            let eb = emit_expr(b, config);
            quote!((#ea + #eb))
        }
        Expr::Sub(a, b) => {
            let ea = emit_expr(a, config);
            let eb = emit_expr(b, config);
            quote!((#ea - #eb))
        }
        Expr::Mul(a, b) => {
            let ea = emit_expr(a, config);
            let eb = emit_expr(b, config);
            quote!((#ea * #eb))
        }
        Expr::Div(a, b) => {
            let ea = emit_expr(a, config);
            let eb = emit_expr(b, config);
            quote!((#ea / #eb))
        }
        Expr::Pow(base, exp) => {
            let ebase = emit_expr(base, config);
            let eexp = emit_expr(exp, config);
            quote!(#ebase.powf(#eexp))
        }
        Expr::Neg(a) => {
            let ea = emit_expr(a, config);
            quote!(-#ea)
        }
        Expr::Sin(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.sin())
        }
        Expr::Cos(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.cos())
        }
        Expr::Tan(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.tan())
        }
        Expr::Exp(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.exp())
        }
        Expr::Ln(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.ln())
        }
        Expr::Sqrt(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.sqrt())
        }
        Expr::Sinh(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.sinh())
        }
        Expr::Cosh(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.cosh())
        }
        Expr::Tanh(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.tanh())
        }
        Expr::Asin(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.asin())
        }
        Expr::Acos(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.acos())
        }
        Expr::Atan(a) => {
            let ea = emit_expr(a, config);
            quote!(#ea.atan())
        }
        Expr::Pi => {
            let ty = config.float_type.type_tokens();
            quote!(#ty::consts::PI)
        }
        Expr::E => {
            let ty = config.float_type.type_tokens();
            quote!(#ty::consts::E)
        }
        #[cfg(feature = "complex")]
        Expr::I => {
            // Emits complex imaginary unit (0.0, 1.0)
            quote!((0.0, 1.0))
        }
        Expr::Factorial(a) => {
            let ea = emit_expr(a, config);
            let ty = config.float_type.type_tokens();
            quote! {{
                let __n = #ea as u32;
                let mut __acc: #ty = 1.0;
                let mut __k: u32 = 2;
                while __k <= __n {
                    __acc *= __k as #ty;
                    __k += 1;
                }
                __acc
            }}
        }
        Expr::Tensor(name, indices) => {
            let mut s = name.clone();
            for idx in indices {
                s.push_str(if idx.idx_type == crate::tensor::IndexType::Covariant {
                    "_cov_"
                } else {
                    "_con_"
                });
                s.push_str(&idx.name);
            }
            let ident = format_ident!("{}", s);
            quote!(#ident)
        }
        Expr::Derivative(a, b) => {
            let ea = emit_expr(a, config);
            let eb = emit_expr(b, config);
            // Symbolic derivative emitted as a function call `diff(expr, var)`
            quote!(diff(#ea, #eb))
        }
        Expr::Integral(a, b) => {
            let ea = emit_expr(a, config);
            let eb = emit_expr(b, config);
            // Symbolic integral emitted as a function call `integral(expr, var)`
            quote!(integral(#ea, #eb))
        }
        Expr::Sum(a, b, lower, upper) => {
            let ea = emit_expr(a, config);
            let eb = emit_expr(b, config);
            let el = emit_expr(lower, config);
            let eu = emit_expr(upper, config);
            quote!(sum(#ea, #eb, #el, #eu))
        }
        Expr::Matrix(mat) => {
            let mut rows = Vec::new();
            for row in mat {
                let elems: Vec<_> = row.iter().map(|e| emit_expr(e, config)).collect();
                rows.push(quote!([#(#elems),*]));
            }
            // Emits a 2D array: [[a, b], [c, d]]
            quote!([#(#rows),*])
        }
        #[cfg(feature = "specfn")]
        Expr::SpecialFn(kind, args) => {
            let fn_name = format_ident!("{}", kind.as_str());
            let emitted_args: Vec<_> = args.iter().map(|a| emit_expr(a, config)).collect();
            quote!(#fn_name(#(#emitted_args),*))
        }
    }
}

/// Emit a complete Rust block from a [`CseResult`].
#[cfg(feature = "cse")]
pub fn emit_cse_block(cse: &CseResult, config: &CodegenConfig) -> TokenStream {
    let mut bindings_tokens = Vec::with_capacity(cse.bindings.len());
    for b in &cse.bindings {
        let ident = format_ident!("{}", b.name);
        let expr_ts = emit_expr(&b.expr, config);
        bindings_tokens.push(quote! {
            let #ident = #expr_ts;
        });
    }

    let output_exprs: Vec<TokenStream> = cse.outputs.iter().map(|o| emit_expr(o, config)).collect();

    let ret = match output_exprs.len() {
        0 => quote!(()),
        1 => {
            let first = &output_exprs[0];
            quote!(#first)
        }
        _ => quote!((#(#output_exprs),*)),
    };

    quote! {
        #(#bindings_tokens)*
        #ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_simple_expr() {
        let x = Expr::var("x");
        let expr = Expr::add(x, Expr::int(1));
        let tokens = emit_expr(&expr, &CodegenConfig::default());
        let s = tokens.to_string();
        assert!(s.contains('+'));
        assert!(s.contains('x'));
    }

    #[test]
    #[cfg(feature = "cse")]
    fn test_emit_cse_block() {
        let cse = crate::cse::eliminate_common_subexprs(&[
            Expr::add(Expr::sin(Expr::var("x")), Expr::int(1)),
            Expr::mul(Expr::sin(Expr::var("x")), Expr::int(2)),
        ]);
        let tokens = emit_cse_block(&cse, &CodegenConfig::default());
        let s = tokens.to_string();
        assert!(s.contains("let _t"));
    }

    #[test]
    fn test_emit_new_nodes() {
        let cfg = CodegenConfig::default();
        assert!(
            emit_expr(&Expr::sinh(Expr::var("x")), &cfg)
                .to_string()
                .contains("sinh")
        );
        assert!(
            emit_expr(&Expr::atan(Expr::var("x")), &cfg)
                .to_string()
                .contains("atan")
        );
        assert!(
            emit_expr(&Expr::pi(), &cfg)
                .to_string()
                .contains("consts :: PI")
        );
        assert!(
            emit_expr(&Expr::e(), &cfg)
                .to_string()
                .contains("consts :: E")
        );
        let fac = emit_expr(&Expr::factorial(Expr::int(5)), &cfg).to_string();
        assert!(fac.contains("__acc"));
    }
}
