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
pub use sym_core::latex::to_latex;
pub use sym_core::rational::{BigRational, Rat};

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

// ── CAS operations (Phases 22–31) ─────────────────────────────────
#[cfg(feature = "poly")]
pub use sym_core::poly::{expand, factor, partfrac, ratsimp};

#[cfg(feature = "integrate")]
pub use sym_core::integrate::{
    IntegrationResult, contains_integral, defint::DefiniteResult, defint::defint, integrate,
    integrate_result, is_proven_non_elementary,
};

#[cfg(feature = "series")]
pub use sym_core::series::taylor;

#[cfg(feature = "limit")]
pub use sym_core::limit::limit;

#[cfg(feature = "solve")]
pub use sym_core::solve::{AlgebraicSolutions, OdeSolution, ode2, solve};

#[cfg(all(feature = "parser", feature = "cas"))]
pub use sym_core::parser::maxima::{CasCommand, execute_cas, parse_maxima_cas};

#[cfg(feature = "bfloat")]
pub use sym_core::bfloat::{BfloatContext, BfloatWrapper, DEFAULT_FPPREC, bfloat, bfloat_from_str};

#[cfg(feature = "matrix")]
pub use sym_core::matrix::{
    Matrix, MatrixError, adjoint, charpoly, determinant, eigenvalues, invert, trace,
};

#[cfg(feature = "complex")]
pub use sym_core::complex::{
    cabs, carg, conjugate, imagpart, polarform, polarform_expr, realpart, rectform, rectform_expr,
    residue,
};

#[cfg(feature = "specfn")]
pub use sym_core::specfn::{SpecialFnKind, airy, bessel, diff_special_fn, gamma, orthopoly};

#[cfg(feature = "transform")]
pub use sym_core::transform::{bc2, bc2_ode, ic1, ic1_ode, ic2, ic2_ode, ilt, laplace};

#[cfg(feature = "assume")]
pub use sym_core::assume::{
    Assumptions, Comparison, Fact, PredicateResult, PropertyKind, Sign, asksign, assume, csign,
    forget, is,
};
