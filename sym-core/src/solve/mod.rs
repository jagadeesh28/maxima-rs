//! Ordinary differential equation solvers (Phase 30).

pub mod alg;
pub mod isolate;
pub mod ode;
pub mod ode_first;
pub mod ode_second;

pub use alg::{SolveStatus, linsolve};
pub use ode::{OdeSolution, ode2};

use crate::expr::Expr;

/// A set of exact algebraic solutions for an equation.
/// The second element indicates whether the factorization/solution was complete.
#[derive(Debug, Clone, PartialEq)]
pub struct AlgebraicSolutions(pub Vec<Expr>, pub bool);

impl AlgebraicSolutions {
    /// Create a new solution set.
    pub fn new(roots: Vec<Expr>, is_complete: bool) -> Self {
        Self(roots, is_complete)
    }

    /// Whether all roots were found (factorization completed).
    pub fn is_complete(&self) -> bool {
        self.1
    }

    /// Number of solutions found.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// True if no solutions were found.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consume and return the solution vector.
    pub fn into_vec(self) -> Vec<Expr> {
        self.0
    }
}

/// Solve the equation `expr = 0` for `var`, returning exact roots.
pub fn solve(expr: &Expr, var: &str) -> AlgebraicSolutions {
    if let Some((roots, is_complete)) = alg::solve_polynomial(expr, var) {
        return AlgebraicSolutions::new(roots, is_complete);
    }
    if let Some(root) = isolate::isolate(expr, var) {
        return AlgebraicSolutions::new(vec![root], true);
    }
    AlgebraicSolutions::new(Vec::new(), false)
}

/// Solve `expr = 0` for `var` and return both roots and [`SolveStatus`].
pub fn solve_classified(expr: &Expr, var: &str) -> (Vec<Expr>, SolveStatus) {
    if let Some((roots, status)) = alg::solve_polynomial_classified(expr, var) {
        return (roots, status);
    }
    if let Some(root) = isolate::isolate(expr, var) {
        return (vec![root], SolveStatus::Finite);
    }
    (Vec::new(), SolveStatus::Partial)
}
