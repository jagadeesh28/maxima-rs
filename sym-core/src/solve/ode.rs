//! Linear ODE dispatch and coefficient extraction (Phase 30).
//!
//! Recognises linear ODEs in `y(x)` up to second order with coefficients that are
//! independent of `y` and `x` (symbolic parameters such as `ω` are allowed), then
//! dispatches to the first- or second-order solver.

use super::ode_first;
use super::ode_second;
use crate::expr::Expr;

/// A solved ordinary differential equation.
#[derive(Debug, Clone, PartialEq)]
pub struct OdeSolution {
    /// The general solution as an [`Expr`] involving the integration constants.
    pub solution: Expr,
    /// Names of the integration constants (`_C1`, `_C2`, ...).
    pub constants: Vec<String>,
}

fn atom_order(e: &Expr, y: &str, x: &str) -> Option<u8> {
    match e {
        Expr::Var(n) if n.as_str() == y => Some(0),
        Expr::Derivative(inner, var) => {
            let Expr::Var(v) = var.as_ref() else {
                return None;
            };
            if v.as_str() != x {
                return None;
            }
            match inner.as_ref() {
                Expr::Var(n) if n.as_str() == y => Some(1),
                Expr::Derivative(i2, v2) => {
                    let Expr::Var(vv) = v2.as_ref() else {
                        return None;
                    };
                    if vv.as_str() == x && matches!(i2.as_ref(), Expr::Var(n) if n.as_str() == y) {
                        Some(2)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn independent_of(e: &Expr, y: &str, x: &str) -> bool {
    !e.variables().iter().any(|v| v == y || v == x)
}

fn linear_terms(e: &Expr, y: &str, x: &str) -> Option<Vec<(u8, Expr)>> {
    if let Some(order) = atom_order(e, y, x) {
        return Some(vec![(order, Expr::int(1))]);
    }
    match e {
        Expr::Add(a, b) => {
            let mut t = linear_terms(a, y, x)?;
            t.extend(linear_terms(b, y, x)?);
            Some(t)
        }
        Expr::Sub(a, b) => {
            let mut t = linear_terms(a, y, x)?;
            for (o, c) in linear_terms(b, y, x)? {
                t.push((o, Expr::neg(c)));
            }
            Some(t)
        }
        Expr::Neg(a) => Some(
            linear_terms(a, y, x)?
                .into_iter()
                .map(|(o, c)| (o, Expr::neg(c)))
                .collect(),
        ),
        Expr::Mul(a, b) => {
            if let Some(o) = atom_order(a, y, x) {
                if independent_of(b, y, x) {
                    return Some(vec![(o, (**b).clone())]);
                }
                return None;
            }
            if let Some(o) = atom_order(b, y, x) {
                if independent_of(a, y, x) {
                    return Some(vec![(o, (**a).clone())]);
                }
                return None;
            }
            None
        }
        _ => None,
    }
}

/// Solve a first- or second-order linear ODE `ode = 0` for `y(x)`.
pub fn ode2(ode: &Expr, y: &str, x: &str) -> Option<OdeSolution> {
    let terms = linear_terms(ode, y, x)?;
    let mut coeffs: [Option<Expr>; 3] = [None, None, None];
    for (o, co) in terms {
        let o = o as usize;
        coeffs[o] = Some(match coeffs[o].take() {
            None => co,
            Some(prev) => Expr::add(prev, co),
        });
    }
    let order = if coeffs[2].is_some() {
        2usize
    } else if coeffs[1].is_some() {
        1usize
    } else {
        return None;
    };
    if order == 1 {
        let a = coeffs[1].clone()?;
        if a.is_zero() {
            return None;
        }
        let b = coeffs[0].clone().unwrap_or_else(|| Expr::int(0));
        ode_first::solve_first(&a, &b, x)
    } else {
        let a = coeffs[2].clone()?;
        if a.is_zero() {
            return None;
        }
        let b = coeffs[1].clone().unwrap_or_else(|| Expr::int(0));
        let c = coeffs[0].clone().unwrap_or_else(|| Expr::int(0));
        ode_second::solve_second(&a, &b, &c, x)
    }
}
