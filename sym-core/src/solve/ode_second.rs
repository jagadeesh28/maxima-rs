//! Second-order constant-coefficient linear ODE solver (Phase 30 / Phase 50).

use super::ode::OdeSolution;
use crate::expr::Expr;
use crate::poly::expand;
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
        _ => {
            #[cfg(feature = "assume")]
            {
                crate::assume::Assumptions::new().csign(e) == crate::assume::Sign::Pos
            }
            #[cfg(not(feature = "assume"))]
            {
                false
            }
        }
    }
}

fn constants2() -> Vec<String> {
    vec!["_C1".to_string(), "_C2".to_string()]
}

fn normalize_leading_sign(a: &Expr, b: &Expr, c: &Expr) -> (Expr, Expr, Expr) {
    if let Expr::Const(r) = a
        && r.is_negative()
    {
        return (
            expand(&Expr::neg(a.clone())),
            expand(&Expr::neg(b.clone())),
            expand(&Expr::neg(c.clone())),
        );
    }
    if let Expr::Neg(inner) = a {
        return (
            (**inner).clone(),
            expand(&Expr::neg(b.clone())),
            expand(&Expr::neg(c.clone())),
        );
    }
    (a.clone(), b.clone(), c.clone())
}

/// Return fundamental solution pair `(y1, y2)` for `a·y'' + b·y' + c·y = 0`.
pub(crate) fn solve_second_bases(a: &Expr, b: &Expr, c: &Expr, x: &str) -> Option<(Expr, Expr)> {
    let a_f = expand(a);
    let b_f = expand(b);
    let c_f = expand(c);
    if a_f.is_zero() {
        return None;
    }
    let (a, b, c) = normalize_leading_sign(&a_f, &b_f, &c_f);
    let xv = Expr::var(x);

    // Handle c == 0: y'' = 0 -> (1, x); a*y'' + b*y' = 0 -> (1, exp(-(b/a)*x))
    if c.is_zero() {
        if b.is_zero() {
            return Some((Expr::int(1), xv));
        }
        let rate = Expr::neg(Expr::div(b, a));
        return Some((Expr::int(1), Expr::exp(Expr::mul(rate, xv))));
    }

    if b.is_zero() {
        if looks_positive(&a) && looks_positive(&c) {
            let q = Expr::sqrt(Expr::div(c, a));
            let qx = Expr::mul(q, xv);
            return Some((Expr::cos(qx.clone()), Expr::sin(qx)));
        }
        let q = Expr::sqrt(Expr::neg(Expr::div(c, a)));
        let qx = Expr::mul(q, xv);
        return Some((Expr::exp(qx.clone()), Expr::exp(Expr::neg(qx))));
    }

    let disc = expand(&Expr::sub(
        Expr::pow(b.clone(), Expr::int(2)),
        Expr::mul(Expr::int(4), Expr::mul(a.clone(), c)),
    ));
    let two_a = expand(&Expr::mul(Expr::int(2), a));
    let neg_b = expand(&Expr::neg(b));

    if let Expr::Const(dr) = &disc {
        if dr.is_negative() {
            let p = Expr::div(neg_b, two_a.clone());
            let q = Expr::div(Expr::sqrt(Expr::constant(-dr)), two_a);
            let px = Expr::mul(p, xv.clone());
            let qx = Expr::mul(q, xv);
            let e_px = Expr::exp(px);
            return Some((
                Expr::mul(e_px.clone(), Expr::cos(qx.clone())),
                Expr::mul(e_px, Expr::sin(qx)),
            ));
        }
        if dr.is_zero() {
            let r = Expr::div(neg_b, two_a);
            let e_rx = Expr::exp(Expr::mul(r, xv.clone()));
            return Some((e_rx.clone(), Expr::mul(xv, e_rx)));
        }
        let sq = Expr::sqrt(Expr::constant(dr.clone()));
        let r1 = Expr::div(Expr::add(neg_b.clone(), sq.clone()), two_a.clone());
        let r2 = Expr::div(Expr::sub(neg_b, sq), two_a);
        return Some((
            Expr::exp(Expr::mul(r1, xv.clone())),
            Expr::exp(Expr::mul(r2, xv)),
        ));
    }

    // Symbolic discriminant via Assumptions::csign
    #[cfg(feature = "assume")]
    {
        use crate::assume::{Assumptions, Sign};
        match Assumptions::new().csign(&disc) {
            Sign::Pos => {
                let sq = Expr::sqrt(disc);
                let r1 = Expr::div(Expr::add(neg_b.clone(), sq.clone()), two_a.clone());
                let r2 = Expr::div(Expr::sub(neg_b, sq), two_a);
                return Some((
                    Expr::exp(Expr::mul(r1, xv.clone())),
                    Expr::exp(Expr::mul(r2, xv)),
                ));
            }
            Sign::Zero => {
                let r = Expr::div(neg_b, two_a);
                let e_rx = Expr::exp(Expr::mul(r, xv.clone()));
                return Some((e_rx.clone(), Expr::mul(xv, e_rx)));
            }
            Sign::Neg => {
                let p = Expr::div(neg_b, two_a.clone());
                let q = Expr::div(Expr::sqrt(Expr::neg(disc)), two_a);
                let e_px = Expr::exp(Expr::mul(p, xv.clone()));
                let qx = Expr::mul(q, xv);
                return Some((
                    Expr::mul(e_px.clone(), Expr::cos(qx.clone())),
                    Expr::mul(e_px, Expr::sin(qx)),
                ));
            }
            _ => {}
        }
    }

    None
}

/// Solve `a·y'' + b·y' + c·y = 0`.
pub(crate) fn solve_second(a: &Expr, b: &Expr, c: &Expr, x: &str) -> Option<OdeSolution> {
    let (y1, y2) = solve_second_bases(a, b, c, x)?;
    let solution = Expr::add(
        Expr::mul(Expr::var("_C1"), y1),
        Expr::mul(Expr::var("_C2"), y2),
    );
    Some(OdeSolution {
        solution,
        constants: constants2(),
    })
}
