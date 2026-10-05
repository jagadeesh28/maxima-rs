//! Procedural macro code expansion orchestrator.
//!
//! Orchestrates the pipeline:
//! 1. Parse Rust tokens into symbolic AST [`Expr`].
//! 2. Differentiate if gradient variables are requested.
//! 3. Inject expressions into e-graph for equality saturation.
//! 4. Extract cost-optimal expressions.
//! 5. Run Common Subexpression Elimination (CSE).
//! 6. Codegen zero-overhead Rust function bodies.

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use sym_core::codegen::{CodegenConfig, emit_cse_block, emit_expr};
use sym_core::cse::eliminate_common_subexprs;
use sym_core::diff::gradient;
use sym_core::extract::{extract_all, extract_single};
use sym_core::saturate::saturate;

use crate::config::MacroConfig;
use crate::parse_sym_eval::SymEvalDef;
use crate::parse_sym_fn::SymFnDef;

/// Expand `sym_fn!` invocation into a Rust function item.
pub fn expand_sym_fn(input: TokenStream) -> TokenStream {
    let def: SymFnDef = match syn::parse(input) {
        Ok(d) => d,
        Err(err) => return err.to_compile_error().into(),
    };

    let config = MacroConfig::default();
    let mut sat_config = config.saturation.clone();
    sat_config.rule_config.fast_math = config.fast_math;
    let codegen_config = CodegenConfig {
        float_type: config.float_type,
    };

    let attrs = &def.attrs;
    let vis = &def.vis;
    let fn_name = &def.name;
    let params: Vec<_> = def.params.iter().map(|(p, ty)| quote!(#p: #ty)).collect();

    if def.gradient_vars.is_empty() {
        // Single output expression optimization
        let (egraph, roots) = saturate(std::slice::from_ref(&def.expr), &sat_config);
        let opt_expr = extract_single(&egraph, roots[0]);
        let cse = eliminate_common_subexprs(&[opt_expr]);
        let body = emit_cse_block(&cse, &codegen_config);

        let ret_type = match &def.ret_type {
            Some(ty) => quote!(#ty),
            None => config.float_type.type_tokens(),
        };

        let expanded = quote! {
            #(#attrs)*
            #vis fn #fn_name(#(#params),*) -> #ret_type {
                #body
            }
        };
        expanded.into()
    } else {
        // Multi-output joint optimization (value + gradients)
        let grad_vars_ref: Vec<&str> = def.gradient_vars.iter().map(|s| s.as_str()).collect();
        let grads = gradient(&def.expr, &grad_vars_ref);

        let mut all_exprs = Vec::with_capacity(1 + grads.len());
        all_exprs.push(def.expr.clone());
        all_exprs.extend(grads);

        let (egraph, roots) = saturate(&all_exprs, &sat_config);
        let extracted = extract_all(&egraph, &roots);
        let cse = eliminate_common_subexprs(&extracted);

        // Emit let bindings for shared subterms
        let mut bindings = Vec::with_capacity(cse.bindings.len());
        for b in &cse.bindings {
            let id = format_ident!("{}", b.name);
            let rhs = emit_expr(&b.expr, &codegen_config);
            bindings.push(quote! {
                let #id = #rhs;
            });
        }

        let val_expr = emit_expr(&cse.outputs[0], &codegen_config);
        let grad_exprs: Vec<_> = cse.outputs[1..]
            .iter()
            .map(|g| emit_expr(g, &codegen_config))
            .collect();
        let num_grads = grad_exprs.len();
        let ty_tokens = config.float_type.type_tokens();

        let ret_type = match &def.ret_type {
            Some(ty) => quote!(#ty),
            None => quote!((#ty_tokens, [#ty_tokens; #num_grads])),
        };

        let expanded = quote! {
            #(#attrs)*
            #vis fn #fn_name(#(#params),*) -> #ret_type {
                #(#bindings)*
                (#val_expr, [#(#grad_exprs),*])
            }
        };
        expanded.into()
    }
}

/// Expand `sym_eval!` invocation into an inline Rust expression block.
pub fn expand_sym_eval(input: TokenStream) -> TokenStream {
    let def: SymEvalDef = match syn::parse(input) {
        Ok(d) => d,
        Err(err) => return err.to_compile_error().into(),
    };

    let config = MacroConfig::default();
    let mut sat_config = config.saturation.clone();
    sat_config.rule_config.fast_math = config.fast_math;
    let codegen_config = CodegenConfig {
        float_type: config.float_type,
    };

    let (egraph, roots) = saturate(std::slice::from_ref(&def.expr), &sat_config);
    let opt_expr = extract_single(&egraph, roots[0]);
    let cse = eliminate_common_subexprs(&[opt_expr]);
    let body = emit_cse_block(&cse, &codegen_config);

    let expanded = quote! {
        {
            #body
        }
    };
    expanded.into()
}
