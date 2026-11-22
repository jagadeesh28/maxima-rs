//! Expression expansion: distribute products over sums and expand integer powers.

use super::Poly;
use crate::expr::Expr;
use crate::rational::Rat;
use num_traits::ToPrimitive;

/// Expand `expr` into a fully distributed polynomial form when possible.
///
/// Products over sums and non-negative integer powers are expanded and identical
/// monomials are combined. Non-polynomial expressions (e.g. `sin(x) * (1 + x)`)
/// fall back to structural distribution without combining like terms.
///
/// The result is a plain [`Expr`] and is **not** passed through e-graph
/// extraction, so expansion is never undone by cost-based simplification.
pub fn expand(expr: &Expr) -> Expr {
    match Poly::from_expr(expr) {
        Ok(poly) => poly.to_expr(),
        Err(_) => structural_expand(expr),
    }
}

fn structural_expand(expr: &Expr) -> Expr {
    match expr {
        Expr::Add(a, b) => Expr::add(structural_expand(a), structural_expand(b)),
        Expr::Sub(a, b) => Expr::sub(structural_expand(a), structural_expand(b)),
        Expr::Mul(a, b) => distribute(structural_expand(a), structural_expand(b)),
        Expr::Neg(a) => Expr::neg(structural_expand(a)),
        Expr::Pow(base, exp) => expand_pow(structural_expand(base), exp),
        Expr::Div(a, b) => Expr::div(structural_expand(a), structural_expand(b)),
        other => other.clone(),
    }
}

fn expand_pow(base: Expr, exp: &Expr) -> Expr {
    if let Some(n) = integer_exponent(exp) {
        return match n {
            0 => Expr::int(1),
            1 => base,
            _ => {
                let mut acc = base.clone();
                for _ in 1..n {
                    acc = distribute(acc, base.clone());
                }
                acc
            }
        };
    }
    Expr::pow(base, structural_expand(exp))
}

/// Extract a non-negative integer exponent from a constant exponent expression.
fn integer_exponent(exp: &Expr) -> Option<u32> {
    match exp {
        Expr::Const(r) if Rat::is_integer(r) => r.numer().to_u32(),
        _ => None,
    }
}

fn distribute(a: Expr, b: Expr) -> Expr {
    match (&a, &b) {
        (Expr::Add(x, y), _) => {
            let (x, y) = ((**x).clone(), (**y).clone());
            Expr::add(distribute(x, b.clone()), distribute(y, b))
        }
        (Expr::Sub(x, y), _) => {
            let (x, y) = ((**x).clone(), (**y).clone());
            Expr::sub(distribute(x, b.clone()), distribute(y, b))
        }
        (_, Expr::Add(x, y)) => {
            let (x, y) = ((**x).clone(), (**y).clone());
            Expr::add(distribute(a.clone(), x), distribute(a, y))
        }
        (_, Expr::Sub(x, y)) => {
            let (x, y) = ((**x).clone(), (**y).clone());
            Expr::sub(distribute(a.clone(), x), distribute(a, y))
        }
        _ => Expr::mul(a, b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn eval(e: &Expr, pairs: &[(&str, f64)]) -> f64 {
        let mut env = HashMap::new();
        for (k, v) in pairs {
            env.insert((*k).to_string(), *v);
        }
        e.eval(&env).expect("evaluation failed")
    }

    #[test]
    fn expand_polynomial_power() {
        let x = Expr::var("x");
        let y = Expr::var("y");
        let e = Expr::pow(Expr::add(x, y), Expr::int(4));
        let out = expand(&e);
        for (a, b) in [(1.0, 1.0), (2.0, -3.0), (0.5, 1.5)] {
            let before = eval(&e, &[("x", a), ("y", b)]);
            let after = eval(&out, &[("x", a), ("y", b)]);
            assert!((before - after).abs() < 1e-9, "{before} vs {after}");
        }
    }

    #[test]
    fn expand_constant_folds() {
        let e = Expr::mul(Expr::int(2), Expr::int(3));
        assert_eq!(expand(&e), Expr::int(6));
    }

    #[test]
    fn expand_non_polynomial_distributes() {
        let x = Expr::var("x");
        let e = Expr::mul(Expr::sin(x.clone()), Expr::add(x, Expr::int(1)));
        let out = expand(&e);
        for xv in [0.3_f64, 1.2, 2.5] {
            let b = eval(&e, &[("x", xv)]);
            let a = eval(&out, &[("x", xv)]);
            assert!((a - b).abs() < 1e-9);
        }
    }
}
