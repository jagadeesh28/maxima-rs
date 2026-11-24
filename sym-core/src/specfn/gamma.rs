//! Gamma, Beta, Psi, and Error functions.
//!
//! Citations:
//! - Maxima 5.50: `src/specfn.lisp`, `src/comm.lisp:481`, `src/combin.lisp:41-43`.
//! - Abramowitz & Stegun: Handbook of Mathematical Functions, Ch. 6, 7.

use crate::diff::differentiate;
use crate::expr::Expr;
use crate::rational::{BigRational, Rat};
use crate::specfn::SpecialFnKind;

/// Evaluate known exact values of the Gamma function $\Gamma(z)$.
///
/// Maxima `src/asum.lisp`, `src/combin.lisp`.
pub fn gamma_eval(z: &Expr) -> Option<Expr> {
    match z {
        Expr::Const(r) if Rat::is_integer(r) => {
            let n = r.to_integer();
            if n <= num_bigint::BigInt::from(0) {
                None // Pole at non-positive integers
            } else if n == num_bigint::BigInt::from(1) || n == num_bigint::BigInt::from(2) {
                Some(Expr::int(1))
            } else {
                // Gamma(n) = (n-1)!
                let n_minus_1 = n - 1;
                let fact = Rat::factorial(&BigRational::from_integer(n_minus_1))?;
                Some(Expr::constant(fact))
            }
        }
        Expr::Const(r) if r == &BigRational::new(1.into(), 2.into()) => {
            // Gamma(1/2) = sqrt(pi)
            Some(Expr::sqrt(Expr::pi()))
        }
        Expr::Const(r) if r == &BigRational::new(3.into(), 2.into()) => {
            // Gamma(3/2) = (1/2)*sqrt(pi)
            Some(Expr::mul(
                Expr::constant(BigRational::new(1.into(), 2.into())),
                Expr::sqrt(Expr::pi()),
            ))
        }
        _ => None,
    }
}

/// Evaluate known exact values of the Beta function $\beta(a, b) = \frac{\Gamma(a)\Gamma(b)}{\Gamma(a+b)}$.
///
/// Maxima `src/combin.lisp:41-43`.
pub fn beta_eval(a: &Expr, b: &Expr) -> Option<Expr> {
    let ga = gamma_eval(a)?;
    let gb = gamma_eval(b)?;
    let sum = match (a, b) {
        (Expr::Const(ra), Expr::Const(rb)) => Expr::constant(Rat::add(ra, rb)),
        _ => Expr::add(a.clone(), b.clone()),
    };
    let g_sum = gamma_eval(&sum)?;
    match (ga, gb, g_sum) {
        (Expr::Const(ra), Expr::Const(rb), Expr::Const(rc)) if !Rat::is_zero(&rc) => {
            Some(Expr::constant(Rat::div(&Rat::mul(&ra, &rb), &rc)?))
        }
        (ea, eb, ec) => Some(Expr::div(Expr::mul(ea, eb), ec)),
    }
}

/// Evaluate known exact values of the Error function $\text{erf}(z)$.
///
/// Maxima `src/comm.lisp:481`.
pub fn erf_eval(z: &Expr) -> Option<Expr> {
    if z.is_zero() {
        Some(Expr::int(0))
    } else {
        None
    }
}

/// Evaluate known exact values of the Complementary Error function $\text{erfc}(z)$.
pub fn erfc_eval(z: &Expr) -> Option<Expr> {
    if z.is_zero() {
        Some(Expr::int(1))
    } else {
        None
    }
}

/// Differentiate $\Gamma(u)$ with respect to `var`:
///
/// $$\frac{d}{dx} \Gamma(u) = \Gamma(u) \cdot \psi_0(u) \cdot \frac{du}{dx}$$
///
/// Maxima 5.50 `src/comm.lisp`, `src/specfn.lisp`.
pub fn diff_gamma(args: &[Expr], var: &str) -> Expr {
    if args.is_empty() {
        return Expr::int(0);
    }
    let u = &args[0];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }
    let gamma_term = Expr::special_fn(SpecialFnKind::Gamma, vec![u.clone()]);
    let psi_term = Expr::special_fn(SpecialFnKind::Psi, vec![Expr::int(0), u.clone()]);
    let d = Expr::mul(gamma_term, psi_term);
    if du.is_one() { d } else { Expr::mul(d, du) }
}

/// Differentiate $\beta(a, b)$ with respect to `var`:
///
/// $$\frac{d}{dx} \beta(a, b) = \beta(a, b) \left( (\psi_0(a) - \psi_0(a+b)) \frac{da}{dx} + (\psi_0(b) - \psi_0(a+b)) \frac{db}{dx} \right)$$
///
/// Maxima 5.50 `src/combin.lisp:41-43`.
pub fn diff_beta(args: &[Expr], var: &str) -> Expr {
    if args.len() < 2 {
        return Expr::int(0);
    }
    let a = &args[0];
    let b = &args[1];
    let da = differentiate(a, var);
    let db = differentiate(b, var);

    if da.is_zero() && db.is_zero() {
        return Expr::int(0);
    }

    let a_plus_b = Expr::add(a.clone(), b.clone());
    let psi_ab = Expr::special_fn(SpecialFnKind::Psi, vec![Expr::int(0), a_plus_b]);
    let beta_term = Expr::special_fn(SpecialFnKind::Beta, vec![a.clone(), b.clone()]);

    let mut inner_terms = Vec::new();
    if !da.is_zero() {
        let psi_a = Expr::special_fn(SpecialFnKind::Psi, vec![Expr::int(0), a.clone()]);
        let diff_a = Expr::sub(psi_a, psi_ab.clone());
        inner_terms.push(if da.is_one() {
            diff_a
        } else {
            Expr::mul(diff_a, da)
        });
    }
    if !db.is_zero() {
        let psi_b = Expr::special_fn(SpecialFnKind::Psi, vec![Expr::int(0), b.clone()]);
        let diff_b = Expr::sub(psi_b, psi_ab);
        inner_terms.push(if db.is_one() {
            diff_b
        } else {
            Expr::mul(diff_b, db)
        });
    }

    let combined = match inner_terms.len() {
        0 => Expr::int(0),
        1 => inner_terms.remove(0),
        _ => Expr::add(inner_terms.remove(0), inner_terms.remove(0)),
    };

    Expr::mul(beta_term, combined)
}

/// Differentiate Polygamma $\psi_n(u)$ with respect to `var`:
///
/// $$\frac{\partial}{\partial u} \psi_n(u) = \psi_{n+1}(u)$$
///
/// Maxima 5.50 `src/comm.lisp:496-501` (`defgrad $psi`).
pub fn diff_psi(args: &[Expr], var: &str) -> Expr {
    if args.is_empty() {
        return Expr::int(0);
    }
    let (n, u) = if args.len() == 1 {
        (Expr::int(0), &args[0])
    } else {
        (args[0].clone(), &args[1])
    };
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }
    let next_n = match &n {
        Expr::Const(r) => Expr::constant(Rat::add(r, &Rat::from_int(1))),
        _ => Expr::add(n, Expr::int(1)),
    };
    let psi_next = Expr::special_fn(SpecialFnKind::Psi, vec![next_n, u.clone()]);
    if du.is_one() {
        psi_next
    } else {
        Expr::mul(psi_next, du)
    }
}

/// Differentiate $\text{erf}(u)$ with respect to `var`:
///
/// $$\frac{d}{dx} \text{erf}(u) = \frac{2}{\sqrt{\pi}} e^{-u^2} \frac{du}{dx}$$
///
/// Maxima 5.50 `src/comm.lisp:481-482`.
pub fn diff_erf(args: &[Expr], var: &str) -> Expr {
    if args.is_empty() {
        return Expr::int(0);
    }
    let u = &args[0];
    let du = differentiate(u, var);
    if du.is_zero() {
        return Expr::int(0);
    }
    // 2 / sqrt(pi) * exp(-u^2) * du
    let coeff = Expr::div(Expr::int(2), Expr::sqrt(Expr::pi()));
    let exp_term = Expr::exp(Expr::neg(Expr::pow(u.clone(), Expr::int(2))));
    let deriv = Expr::mul(coeff, exp_term);
    if du.is_one() {
        deriv
    } else {
        Expr::mul(deriv, du)
    }
}

/// Differentiate $\text{erfc}(u)$ with respect to `var`:
///
/// $$\frac{d}{dx} \text{erfc}(u) = -\frac{2}{\sqrt{\pi}} e^{-u^2} \frac{du}{dx}$$
pub fn diff_erfc(args: &[Expr], var: &str) -> Expr {
    Expr::neg(diff_erf(args, var))
}
