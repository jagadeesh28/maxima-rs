//! Ergonomic prelude for `maxima-rs`.
//!
//! Conveniently imports core expression types, differentiation functions,
//! high-level pipelines, multi-format parsers, and procedural macros.
//!
//! # Usage
//!
//! ```rust
//! use maxima_rs::prelude::*;
//! ```

// Core AST & exact rational constants
pub use sym_core::expr::Expr;
pub use sym_core::rational::Rat;

// Analytical calculus
#[cfg(feature = "diff")]
pub use sym_core::diff::{differentiate, gradient, hessian, jacobian};

// Multi-format parsers
#[cfg(feature = "parser")]
pub use sym_core::parser::{ParseError, parse_latex, parse_maxima};

// High-level pipeline and optimization
#[cfg(feature = "pipeline")]
pub use sym_core::pipeline::{Pipeline, optimize, optimize_gradient};

// Code generation target types
#[cfg(feature = "codegen")]
pub use sym_core::codegen::FloatType;

// Procedural macros
#[cfg(feature = "macros")]
pub use sym_macros::{sym_eval, sym_fn};
