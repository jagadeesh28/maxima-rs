//! syn-based parser for item-level sym_fn! syntax.

use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, Token, Type, Visibility, braced, bracketed};

use sym_core::expr::Expr;

/// Parsed `sym_fn!` function definition.
#[derive(Debug)]
pub struct SymFnDef {
    pub attrs: Vec<Attribute>,
    pub vis: Visibility,
    pub name: Ident,
    pub params: Vec<(Ident, Type)>,
    pub ret_type: Option<Type>,
    pub expr: Expr,
    pub gradient_vars: Vec<String>,
}

impl Parse for SymFnDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        input.parse::<Token![fn]>()?;
        let name: Ident = input.parse()?;

        // Parameter list: (x: f64, y: f64)
        let content;
        syn::parenthesized!(content in input);
        let mut params = Vec::new();
        while !content.is_empty() {
            let param_name: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let param_ty: Type = content.parse()?;
            params.push((param_name, param_ty));
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        // Return type: -> ...
        let ret_type = if input.peek(Token![->]) {
            input.parse::<Token![->]>()?;
            Some(input.parse()?)
        } else {
            None
        };

        // Body block: { expr: ..., gradient: [...] }
        let body_content;
        braced!(body_content in input);

        let mut expr_opt = None;
        let mut gradient_vars = Vec::new();

        while !body_content.is_empty() {
            let key: Ident = body_content.parse()?;
            body_content.parse::<Token![:]>()?;

            if key == "expr" {
                let syn_expr: syn::Expr = body_content.parse()?;
                expr_opt = Some(convert_syn_expr(&syn_expr)?);
            } else if key == "gradient" {
                let grad_content;
                bracketed!(grad_content in body_content);
                let idents: Punctuated<Ident, Token![,]> =
                    grad_content.parse_terminated(Ident::parse, Token![,])?;
                gradient_vars = idents.into_iter().map(|id| id.to_string()).collect();
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    "Unknown field in sym_fn body; expected 'expr' or 'gradient'",
                ));
            }

            if body_content.peek(Token![,]) {
                body_content.parse::<Token![,]>()?;
            }
        }

        let expr = expr_opt.ok_or_else(|| {
            syn::Error::new(input.span(), "Missing required 'expr:' in sym_fn body")
        })?;

        Ok(Self {
            attrs,
            vis,
            name,
            params,
            ret_type,
            expr,
            gradient_vars,
        })
    }
}

/// Convert a syn::Expr (Rust syntax AST) into a sym_core::expr::Expr AST.
pub fn convert_syn_expr(e: &syn::Expr) -> syn::Result<Expr> {
    match e {
        syn::Expr::Path(p) => {
            if let Some(ident) = p.path.get_ident() {
                Ok(Expr::var(&ident.to_string()))
            } else {
                Err(syn::Error::new_spanned(
                    e,
                    "Complex paths are not supported in symbolic expressions",
                ))
            }
        }
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Float(f) => {
                let val: f64 = f.base10_parse()?;
                Ok(Expr::from_f64(val))
            }
            syn::Lit::Int(i) => {
                let val: i64 = i.base10_parse()?;
                Ok(Expr::int(val))
            }
            _ => Err(syn::Error::new_spanned(e, "Unsupported literal type")),
        },
        syn::Expr::Binary(bin) => {
            let left = convert_syn_expr(&bin.left)?;
            let right = convert_syn_expr(&bin.right)?;
            match bin.op {
                syn::BinOp::Add(_) => Ok(Expr::add(left, right)),
                syn::BinOp::Sub(_) => Ok(Expr::sub(left, right)),
                syn::BinOp::Mul(_) => Ok(Expr::mul(left, right)),
                syn::BinOp::Div(_) => Ok(Expr::div(left, right)),
                _ => Err(syn::Error::new_spanned(bin, "Unsupported binary operator")),
            }
        }
        syn::Expr::Unary(un) => {
            let inner = convert_syn_expr(&un.expr)?;
            match un.op {
                syn::UnOp::Neg(_) => Ok(Expr::neg(inner)),
                _ => Err(syn::Error::new_spanned(un, "Unsupported unary operator")),
            }
        }
        syn::Expr::Paren(paren) => convert_syn_expr(&paren.expr),
        syn::Expr::MethodCall(mc) => {
            let receiver = convert_syn_expr(&mc.receiver)?;
            let method = mc.method.to_string();
            match method.as_str() {
                "sin" => Ok(Expr::sin(receiver)),
                "cos" => Ok(Expr::cos(receiver)),
                "tan" => Ok(Expr::tan(receiver)),
                "exp" => Ok(Expr::exp(receiver)),
                "ln" => Ok(Expr::ln(receiver)),
                "sqrt" => Ok(Expr::sqrt(receiver)),
                "powi" | "powf" => {
                    let arg = mc.args.first().ok_or_else(|| {
                        syn::Error::new_spanned(mc, "pow method requires an exponent argument")
                    })?;
                    let exp = convert_syn_expr(arg)?;
                    Ok(Expr::pow(receiver, exp))
                }
                _ => Err(syn::Error::new_spanned(
                    mc,
                    format!("Unsupported method call: {method}"),
                )),
            }
        }
        syn::Expr::Call(call) => {
            if let syn::Expr::Path(p) = &*call.func {
                if let Some(ident) = p.path.get_ident() {
                    let func_name = ident.to_string();
                    let mut args = call.args.iter();
                    match func_name.as_str() {
                        "sin" => {
                            let arg = args.next().ok_or_else(|| {
                                syn::Error::new_spanned(call, "sin requires 1 argument")
                            })?;
                            Ok(Expr::sin(convert_syn_expr(arg)?))
                        }
                        "cos" => {
                            let arg = args.next().ok_or_else(|| {
                                syn::Error::new_spanned(call, "cos requires 1 argument")
                            })?;
                            Ok(Expr::cos(convert_syn_expr(arg)?))
                        }
                        "tan" => {
                            let arg = args.next().ok_or_else(|| {
                                syn::Error::new_spanned(call, "tan requires 1 argument")
                            })?;
                            Ok(Expr::tan(convert_syn_expr(arg)?))
                        }
                        "exp" => {
                            let arg = args.next().ok_or_else(|| {
                                syn::Error::new_spanned(call, "exp requires 1 argument")
                            })?;
                            Ok(Expr::exp(convert_syn_expr(arg)?))
                        }
                        "ln" => {
                            let arg = args.next().ok_or_else(|| {
                                syn::Error::new_spanned(call, "ln requires 1 argument")
                            })?;
                            Ok(Expr::ln(convert_syn_expr(arg)?))
                        }
                        "sqrt" => {
                            let arg = args.next().ok_or_else(|| {
                                syn::Error::new_spanned(call, "sqrt requires 1 argument")
                            })?;
                            Ok(Expr::sqrt(convert_syn_expr(arg)?))
                        }
                        "pow" => {
                            let base = args.next().ok_or_else(|| {
                                syn::Error::new_spanned(call, "pow requires 2 arguments")
                            })?;
                            let exp = args.next().ok_or_else(|| {
                                syn::Error::new_spanned(call, "pow requires 2 arguments")
                            })?;
                            Ok(Expr::pow(convert_syn_expr(base)?, convert_syn_expr(exp)?))
                        }
                        _ => Err(syn::Error::new_spanned(
                            call,
                            format!("Unsupported function call: {func_name}"),
                        )),
                    }
                } else {
                    Err(syn::Error::new_spanned(call, "Unsupported function path"))
                }
            } else {
                Err(syn::Error::new_spanned(call, "Unsupported call expression"))
            }
        }
        _ => Err(syn::Error::new_spanned(
            e,
            "Unsupported expression syntax in symbolic macro",
        )),
    }
}
