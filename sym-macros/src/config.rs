//! Attribute and configuration parser for symbolic macros.

use sym_core::codegen::FloatType;
use sym_core::saturate::SaturationConfig;
use syn::{Attribute, Expr, Lit, Meta, Type};

/// Configuration options extracted from macro attributes or arguments.
#[derive(Debug, Clone)]
pub struct MacroConfig {
    /// Target floating-point type (default: f64).
    pub float_type: FloatType,
    /// Fast-math enablement.
    pub fast_math: bool,
    /// Saturation limits and rule settings.
    pub saturation: SaturationConfig,
}

impl Default for MacroConfig {
    fn default() -> Self {
        Self {
            float_type: FloatType::F64,
            fast_math: true,
            saturation: SaturationConfig::default(),
        }
    }
}

impl MacroConfig {
    /// Extract `#[fast_math = ...]`, `#[node_limit = ...]`, and `#[time_limit_ms = ...]`
    /// from outer attributes, returning the remaining non-macro attributes.
    pub fn from_attrs(
        attrs: &[Attribute],
        params: &[(syn::Ident, Type)],
        ret_type: Option<&Type>,
    ) -> syn::Result<(Self, Vec<Attribute>)> {
        let mut cfg = Self::default();
        let mut remaining = Vec::new();

        for attr in attrs {
            if attr.path().is_ident("fast_math") {
                if let Meta::NameValue(nv) = &attr.meta
                    && let Expr::Lit(lit) = &nv.value
                    && let Lit::Bool(b) = &lit.lit
                {
                    cfg.fast_math = b.value();
                    continue;
                }
                return Err(syn::Error::new_spanned(
                    attr,
                    "expected `#[fast_math = true]` or `#[fast_math = false]`",
                ));
            }
            if attr.path().is_ident("node_limit") {
                if let Meta::NameValue(nv) = &attr.meta
                    && let Expr::Lit(lit) = &nv.value
                    && let Lit::Int(i) = &lit.lit
                {
                    cfg.saturation.node_limit = i.base10_parse()?;
                    continue;
                }
                return Err(syn::Error::new_spanned(
                    attr,
                    "expected `#[node_limit = <integer>]`",
                ));
            }
            if attr.path().is_ident("time_limit_ms") {
                if let Meta::NameValue(nv) = &attr.meta
                    && let Expr::Lit(lit) = &nv.value
                    && let Lit::Int(i) = &lit.lit
                {
                    let ms: u64 = i.base10_parse()?;
                    cfg.saturation.time_limit_secs = (ms as f64) / 1000.0;
                    continue;
                }
                return Err(syn::Error::new_spanned(
                    attr,
                    "expected `#[time_limit_ms = <integer>]`",
                ));
            }
            remaining.push(attr.clone());
        }

        if params.iter().any(|(_, ty)| type_is_f32(ty)) || ret_type.is_some_and(type_is_f32) {
            cfg.float_type = FloatType::F32;
        }

        Ok((cfg, remaining))
    }
}

fn type_is_f32(ty: &Type) -> bool {
    match ty {
        Type::Path(p) => p.path.is_ident("f32"),
        Type::Tuple(t) => t.elems.iter().any(type_is_f32),
        Type::Array(a) => type_is_f32(&a.elem),
        _ => false,
    }
}
