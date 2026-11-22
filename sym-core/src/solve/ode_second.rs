//! Second-order constant-coefficient linear ODE solver.

use super::ode::OdeSolution;
use crate::expr::Expr;
use crate::rational::Rat;
use num_traits::{Signed, ToPrimitive, Zero};

fn looks_positive(e: &Expr) -> bool {
    match e {
        Expr::Const(r) => !r.is_negative() && !r.is_zero(),
        Expr::Pow(_, exp) => {
            matches!(exp.as_ref(), Expr::Const(r)
                if Rat::is_integer(r)
                    && r.numer().to_i64().map(|n| n % 2 == 0).unwrap_or(false))
        }
        Expr::Mul(a, b) => looks_positive(a) && looks_positive(b),
        _ => false,
    }
}

fn constants2() -> Vec<String> {
    vec!["_C1".to_string(), "_C2".to_string()]
}

/// Solve `a·y'' + b·y' + c·y = 0`.
pub(crate) fn solve_second(a: &Expr, b: &Expr, c: &Expr, x: &str) -> Option<OdeSolution> {
    if a.is_zero() {
        return None;
    }
    let xv = Expr::var(x);
    if b.is_zero() {
        if looks_positive(a) && looks_positive(c) {
            // Oscillatory: y = C1 cos(q x) + C2 sin(q x), q = sqrt(c/a)
            let q = Expr::sqrt(Expr::div(c.clone(), a.clone()));
            let qx = Expr::mul(q, xv);
            let solution = Expr::add(
                Expr::mul(Expr::var("_C1"), Expr::cos(qx.clone())),
                Expr::mul(Expr::var("_C2"), Expr::sin(qx)),
            );
            return Some(OdeSolution {
                solution,
                constants: constants2(),
            });
        }
        // Real symmetric roots: y = C1 e^{q x} + C2 e^{-q x}, q = sqrt(-c/a)
        let q = Expr::sqrt(Expr::neg(Expr::div(c.clone(), a.clone())));
        let qx = Expr::mul(q, xv);
        let solution = Expr::add(
            Expr::mul(Expr::var("_C1"), Expr::exp(qx.clone())),
            Expr::mul(Expr::var("_C2"), Expr::exp(Expr::neg(qx))),
        );
        return Some(OdeSolution {
            solution,
            constants: constants2(),
        });
    }

    let disc = Expr::sub(
        Expr::pow(b.clone(), Expr::int(2)),
        Expr::mul(Expr::int(4), Expr::mul(a.clone(), c.clone())),
    );
    let Expr::Const(dr) = &disc else {
        return None;
    };
    let two_a = Expr::mul(Expr::int(2), a.clone());
    if dr.is_negative() {
        let p = Expr::neg(Expr::div(b.clone(), two_a.clone()));
        let q = Expr::div(Expr::sqrt(Expr::constant(-dr)), two_a);
        let px = Expr::mul(p, xv.clone());
        let qx = Expr::mul(q, xv);
        let inner = Expr::add(
            Expr::mul(Expr::var("_C1"), Expr::cos(qx.clone())),
            Expr::mul(Expr::var("_C2"), Expr::sin(qx)),
        );
        return Some(OdeSolution {
            solution: Expr::mul(Expr::exp(px), inner),
            constants: constants2(),
        });
    }
    let neg_b = Expr::neg(b.clone());
    if dr.is_zero() {
        let r = Expr::div(neg_b, two_a);
        let poly = Expr::add(Expr::var("_C1"), Expr::mul(Expr::var("_C2"), xv.clone()));
        return Some(OdeSolution {
            solution: Expr::mul(poly, Expr::exp(Expr::mul(r, xv))),
            constants: constants2(),
        });
    }
    let sq = Expr::sqrt(Expr::constant(dr.clone()));
    let r1 = Expr::div(Expr::add(neg_b.clone(), sq.clone()), two_a.clone());
    let r2 = Expr::div(Expr::sub(neg_b, sq), two_a);
    let solution = Expr::add(
        Expr::mul(Expr::var("_C1"), Expr::exp(Expr::mul(r1, xv.clone()))),
        Expr::mul(Expr::var("_C2"), Expr::exp(Expr::mul(r2, xv))),
    );
    Some(OdeSolution {
        solution,
        constants: constants2(),
    })
}
