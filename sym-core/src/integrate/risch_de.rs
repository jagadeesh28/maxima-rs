//! Risch differential equation solver (minimal subset for Phase 25).
//!
//! Solves `y' + f·y = g` for two elementary cases: `f = 0` (direct integration
//! `y = ∫ g`) and `g = 0` (the trivial solution `y = 0`). General field-tower
//! solving is a planned refinement.

use super::integrate;
use crate::expr::Expr;

/// Solve `y' + f·y = g` for `y`, when a simple closed form is available.
pub fn solve_risch_de(f: &Expr, g: &Expr, var: &str) -> Option<Expr> {
    if f.is_zero() {
        return Some(integrate(g, var));
    }
    if g.is_zero() {
        return Some(Expr::int(0));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f_zero_reduces_to_integration() {
        let x = Expr::var("x");
        let g = Expr::var("x");
        let y = solve_risch_de(&Expr::int(0), &g, "x").unwrap();
        assert_eq!(y, Expr::div(Expr::pow(x, Expr::int(2)), Expr::int(2)));
    }

    #[test]
    fn g_zero_yields_zero() {
        let y = solve_risch_de(&Expr::var("a"), &Expr::int(0), "x").unwrap();
        assert_eq!(y, Expr::int(0));
    }
}
