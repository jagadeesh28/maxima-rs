//! # sym-core
//!
//! Core symbolic mathematics engine for maxima-rs.
//!
//! Provides the foundational expression AST, rational constant arithmetic,
//! analytical differentiation, e-graph equality saturation, cost-optimal
//! extraction, common subexpression elimination, and code generation.
//!
//! This crate is usable standalone (without the proc-macro frontend) for
//! build scripts, CLI tools, or programmatic symbolic computation.

pub mod cancel;
pub mod expr;
pub mod latex;
pub mod matrix;
pub mod rational;
pub mod tensor;

pub use cancel::{CancelToken, is_cancelled, with_cancel_token};
pub use expr::Expr;
pub use latex::to_latex;
pub use rational::{BigRational, Rat};

#[cfg(feature = "diff")]
pub mod diff;

#[cfg(feature = "egraph")]
pub mod lang;

#[cfg(feature = "egraph")]
pub mod provenance;

#[cfg(feature = "egraph")]
pub mod rules;

#[cfg(feature = "egraph")]
pub mod cost;

#[cfg(feature = "egraph")]
pub mod saturate;

#[cfg(feature = "egraph")]
pub mod extract;

#[cfg(feature = "cse")]
pub mod cse;

#[cfg(feature = "codegen")]
pub mod codegen;

#[cfg(feature = "parser")]
pub mod parser;

#[cfg(feature = "pipeline")]
pub mod pipeline;

#[cfg(feature = "poly")]
pub mod poly;

#[cfg(feature = "integrate")]
pub mod integrate;

#[cfg(feature = "series")]
pub mod series;

#[cfg(feature = "limit")]
pub mod limit;

#[cfg(feature = "solve")]
pub mod solve;

#[cfg(feature = "cas")]
pub mod maxima_ops;

#[cfg(feature = "bfloat")]
pub mod bfloat;

#[cfg(feature = "bfloat")]
pub use bfloat::{BfloatContext, BfloatWrapper, DEFAULT_FPPREC, bfloat, bfloat_from_str};

#[cfg(feature = "complex")]
pub mod complex;

#[cfg(feature = "complex")]
pub use complex::{
    cabs, carg, conjugate, imagpart, polarform, polarform_expr, realpart, rectform, rectform_expr,
    residue,
};

#[cfg(feature = "specfn")]
pub mod specfn;

#[cfg(feature = "specfn")]
pub use specfn::SpecialFnKind;

#[cfg(feature = "transform")]
pub mod transform;

#[cfg(feature = "transform")]
pub use transform::{bc2, bc2_ode, ic1, ic1_ode, ic2, ic2_ode, ilt, laplace};

#[cfg(feature = "assume")]
pub mod assume;

#[cfg(feature = "assume")]
pub use assume::{
    Assumptions, Comparison, Fact, PredicateResult, PropertyKind, Sign, asksign, assume, csign,
    forget, is,
};
