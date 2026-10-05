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

pub mod expr;
pub mod rational;
pub mod tensor;

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
