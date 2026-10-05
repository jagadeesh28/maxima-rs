//! # maxima-rs
//!
//! A compile-time symbolic mathematics and code optimization engine in Rust.
//!
//! `maxima-rs` converts classical algebraic knowledge inspired by Macsyma / Maxima
//! into an [`egg`]-powered e-graph equality saturation engine that generates zero-overhead
//! optimized native Rust code via procedural macros.
//!
//! ## Core Features
//!
//! - **Exact Rational Arithmetic**: Constant folding with exact arbitrary-precision rationals.
//! - **Analytical Differentiation**: Deterministic calculus pass (`diff`, `gradient`, `jacobian`, `hessian`).
//! - **1:1 Maxima LISP Rule Citations**: Every simplification rule traces to reference Maxima Lisp sources (`simp.lisp`, `trigi.lisp`, `combin.lisp`, `logarc.lisp`).
//! - **FLOP Cost-Optimal Extraction**: Selects mathematically equivalent forms minimizing runtime cycles.
//! - **Common Subexpression Elimination (CSE)**: Detects repeated operations and factors them into optimal `let` bindings.
//! - **Multi-Format Parsers**: Parse expressions from Maxima string syntax or LaTeX mathematical formula notation.
//! - **Zero-Overhead Proc-Macros**: `sym_fn!` and `sym_eval!` emit plain Rust code that compiles without runtime library overhead.
//!
//! ## Quick Start with `sym_fn!`
//!
//! ```rust
//! use maxima_rs::prelude::*;
//!
//! sym_fn! {
//!     pub fn rosenbrock(x: f64, y: f64) -> (f64, [f64; 2]) {
//!         expr: (1.0 - x).powi(2) + 100.0 * (y - x.powi(2)).powi(2),
//!         gradient: [x, y],
//!     }
//! }
//!
//! fn main() {
//!     let (val, grad) = rosenbrock(1.0, 1.0);
//!     assert_eq!(val, 0.0);
//!     assert_eq!(grad, [0.0, 0.0]);
//! }
//! ```
//!
//! ## High-Level Fluent Pipeline
//!
//! ```rust
//! use maxima_rs::prelude::*;
//!
//! let pipeline = Pipeline::new()
//!     .from_latex(r"\frac{\sin(x)^2 + \cos(x)^2}{x}")
//!     .unwrap()
//!     .optimize();
//!
//! assert_eq!(pipeline.expressions(), &[Expr::div(Expr::int(1), Expr::var("x"))]);
//! ```

pub mod prelude;

// Core library re-exports
pub use sym_core::*;

// Procedural macro re-exports
#[cfg(feature = "macros")]
pub use sym_macros::{sym_eval, sym_fn};
