//! First-order linear ODE solver (constant coefficients).

use super::ode::OdeSolution;
use crate::expr::Expr;

/// Solve `a·y' + b·y = 0` for `y(x)`: `y = _C1·exp(-(b/a)·x)`.
pub(crate) fn solve_first(a: &Expr, b: &Expr, x: &str) -> Option<OdeSolution> {
    if a.is_zero() {
        return None;
    }
    let rate = Expr::neg(Expr::div(b.clone(), a.clone()));
    let solution = Expr::mul(Expr::var("_C1"), Expr::exp(Expr::mul(rate, Expr::var(x))));
    Some(OdeSolution {
        solution,
        constants: vec!["_C1".to_string()],
    })
}
