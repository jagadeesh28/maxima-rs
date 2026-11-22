//! Ordinary differential equation solvers (Phase 30).

pub mod alg;
pub mod isolate;
pub mod ode;
pub mod ode_first;
pub mod ode_second;

pub use ode::{OdeSolution, ode2};

use crate::expr::Expr;

/// A set of exact algebraic solutions for an equation.
#[derive(Debug, Clone, PartialEq)]
pub struct AlgebraicSolutions(pub Vec<Expr>);

impl AlgebraicSolutions {
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
    if let Some(roots) = alg::solve_polynomial(expr, var) {
        return AlgebraicSolutions(roots);
    }
    if let Some(root) = isolate::isolate(expr, var) {
        return AlgebraicSolutions(vec![root]);
    }
    AlgebraicSolutions(Vec::new())
}
