//! Linear & non-linear ODE dispatch and coefficient extraction (Phase 30 / Phase 50).
//!
//! Recognises linear ODEs in `y(x)` up to second order (including `x`-dependent
//! coefficients in 1st-order ODEs and non-homogeneous forcing `f(x)`), as well as
//! separable and exact 1st-order ODEs.

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

fn independent_of_y(e: &Expr, y: &str) -> bool {
    !e.variables().iter().any(|v| v == y) && !contains_derivative(e)
}

fn contains_derivative(e: &Expr) -> bool {
    match e {
        Expr::Derivative(_, _) => true,
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            contains_derivative(a) || contains_derivative(b)
        }
        Expr::Neg(a) | Expr::Sin(a) | Expr::Cos(a) | Expr::Exp(a) | Expr::Ln(a) | Expr::Sqrt(a) => {
            contains_derivative(a)
        }
        _ => false,
    }
}

/// Extract `(order, coeff)` terms where `order == 255` denotes a forcing term independent of `y`.
fn linear_terms(e: &Expr, y: &str, x: &str) -> Option<Vec<(u8, Expr)>> {
    if let Some(order) = atom_order(e, y, x) {
        return Some(vec![(order, Expr::int(1))]);
    }
    if independent_of_y(e, y) {
        return Some(vec![(255, e.clone())]);
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
            if let Some(o) = atom_order(a, y, x)
                && independent_of_y(b, y)
            {
                return Some(vec![(o, (**b).clone())]);
            }
            if let Some(o) = atom_order(b, y, x)
                && independent_of_y(a, y)
            {
                return Some(vec![(o, (**a).clone())]);
            }
            None
        }
        Expr::Div(a, b) => {
            if let Some(o) = atom_order(a, y, x)
                && independent_of_y(b, y)
            {
                return Some(vec![(o, Expr::div(Expr::int(1), (**b).clone()))]);
            }
            None
        }
        _ => None,
    }
}

/// Solve a first- or second-order ODE `ode = 0` for `y(x)`.
pub fn ode2(ode: &Expr, y: &str, x: &str) -> Option<OdeSolution> {
    if let Some(terms) = linear_terms(ode, y, x) {
        let mut coeffs: [Option<Expr>; 3] = [None, None, None];
        let mut forcing: Option<Expr> = None;
        for (o, co) in terms {
            if o == 255 {
                forcing = Some(match forcing.take() {
                    None => Expr::neg(co),
                    Some(prev) => Expr::sub(prev, co),
                });
            } else {
                let o = o as usize;
                coeffs[o] = Some(match coeffs[o].take() {
                    None => co,
                    Some(prev) => Expr::add(prev, co),
                });
            }
        }
        let q = forcing.unwrap_or_else(|| Expr::int(0));
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
            return ode_first::solve_first_linear(&a, &b, &q, x);
        }
        let a = coeffs[2].clone()?;
        if a.is_zero() {
            return None;
        }
        let b = coeffs[1].clone().unwrap_or_else(|| Expr::int(0));
        let c = coeffs[0].clone().unwrap_or_else(|| Expr::int(0));
        // 2nd-order constant-coefficient homogeneous or non-homogeneous (Variation of Parameters)
        if a.variables().iter().any(|v| v == x)
            || b.variables().iter().any(|v| v == x)
            || c.variables().iter().any(|v| v == x)
        {
            return None;
        }
        if q.is_zero() {
            return ode_second::solve_second(&a, &b, &c, x);
        }
        return solve_second_nonhomogeneous(&a, &b, &c, &q, x);
    }

    ode_first::solve_separable_or_exact(ode, y, x)
}

#[cfg(feature = "integrate")]
fn solve_second_nonhomogeneous(
    a: &Expr,
    b: &Expr,
    c: &Expr,
    q: &Expr,
    x: &str,
) -> Option<OdeSolution> {
    use crate::diff::differentiate;
    use crate::integrate::{contains_integral, integrate};
    use crate::poly::{expand, ratsimp};

    let (y1, y2) = ode_second::solve_second_bases(a, b, c, x)?;
    let yh = Expr::add(
        Expr::mul(Expr::var("_C1"), y1.clone()),
        Expr::mul(Expr::var("_C2"), y2.clone()),
    );

    if let Some(yp) = try_undetermined_coeffs(a, b, c, q, x) {
        return Some(OdeSolution {
            solution: Expr::add(yh, yp),
            constants: vec!["_C1".to_string(), "_C2".to_string()],
        });
    }

    let dy1 = differentiate(&y1, x);
    let dy2 = differentiate(&y2, x);
    let w = Expr::sub(Expr::mul(y1.clone(), dy2), Expr::mul(y2.clone(), dy1));
    #[cfg(feature = "egraph")]
    let w_simp = crate::pipeline::optimize(&w, &crate::saturate::SaturationConfig::default());
    #[cfg(not(feature = "egraph"))]
    let w_simp = ratsimp(&w).unwrap_or(w);
    if w_simp.is_zero() {
        return None;
    }
    let g = Expr::div(q.clone(), a.clone());
    let u1_prime = ratsimp(&Expr::neg(Expr::div(
        Expr::mul(y2.clone(), g.clone()),
        w_simp.clone(),
    )))
    .unwrap_or_else(|_| Expr::neg(Expr::div(Expr::mul(y2.clone(), g.clone()), w_simp.clone())));
    let u2_prime = ratsimp(&Expr::div(Expr::mul(y1.clone(), g.clone()), w_simp.clone()))
        .unwrap_or_else(|_| Expr::div(Expr::mul(y1.clone(), g), w_simp));
    let u1 = integrate(&u1_prime, x);
    let u2 = integrate(&u2_prime, x);
    if contains_integral(&u1) || contains_integral(&u2) {
        return None;
    }
    let yp = expand(&Expr::add(Expr::mul(u1, y1), Expr::mul(u2, y2)));
    Some(OdeSolution {
        solution: Expr::add(yh, yp),
        constants: vec!["_C1".to_string(), "_C2".to_string()],
    })
}

#[cfg(feature = "integrate")]
fn extract_linear_x_coeff(arg: &Expr, x: &str) -> Option<Expr> {
    match arg {
        Expr::Var(v) if v == x => Some(Expr::int(1)),
        Expr::Neg(inner) => extract_linear_x_coeff(inner, x).map(Expr::neg),
        Expr::Mul(p, q) => {
            if matches!(p.as_ref(), Expr::Var(v) if v == x) && !q.variables().iter().any(|v| v == x)
            {
                Some((**q).clone())
            } else if matches!(q.as_ref(), Expr::Var(v) if v == x)
                && !p.variables().iter().any(|v| v == x)
            {
                Some((**p).clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(feature = "integrate")]
fn try_undetermined_coeffs(a: &Expr, b: &Expr, c: &Expr, q: &Expr, x: &str) -> Option<Expr> {
    use crate::poly::expand;
    match q {
        Expr::Neg(inner) => try_undetermined_coeffs(a, b, c, inner, x).map(Expr::neg),
        Expr::Mul(p, r) if !p.variables().iter().any(|v| v == x) => {
            try_undetermined_coeffs(a, b, c, r, x).map(|yp| Expr::mul((**p).clone(), yp))
        }
        Expr::Mul(p, r) if !r.variables().iter().any(|v| v == x) => {
            try_undetermined_coeffs(a, b, c, p, x).map(|yp| Expr::mul((**r).clone(), yp))
        }
        Expr::Sin(arg) | Expr::Cos(arg) => {
            let w = extract_linear_x_coeff(arg, x)?;
            let diag = expand(&Expr::sub(
                c.clone(),
                Expr::mul(a.clone(), Expr::pow(w.clone(), Expr::int(2))),
            ));
            let off = expand(&Expr::mul(b.clone(), w));
            let det = expand(&Expr::add(
                Expr::pow(diag.clone(), Expr::int(2)),
                Expr::pow(off.clone(), Expr::int(2)),
            ));
            if det.is_zero() {
                return None;
            }
            let (a_coeff, b_coeff) = if matches!(q, Expr::Sin(_)) {
                (Expr::div(Expr::neg(off), det.clone()), Expr::div(diag, det))
            } else {
                (Expr::div(diag, det.clone()), Expr::div(off, det))
            };
            Some(Expr::add(
                Expr::mul(a_coeff, Expr::cos((**arg).clone())),
                Expr::mul(b_coeff, Expr::sin((**arg).clone())),
            ))
        }
        Expr::Exp(arg) => {
            let r = extract_linear_x_coeff(arg, x)?;
            let denom = expand(&Expr::add(
                Expr::add(
                    Expr::mul(a.clone(), Expr::pow(r.clone(), Expr::int(2))),
                    Expr::mul(b.clone(), r),
                ),
                c.clone(),
            ));
            if denom.is_zero() {
                return None;
            }
            Some(Expr::div(q.clone(), denom))
        }
        _ => None,
    }
}

#[cfg(not(feature = "integrate"))]
fn solve_second_nonhomogeneous(
    _a: &Expr,
    _b: &Expr,
    _c: &Expr,
    _q: &Expr,
    _x: &str,
) -> Option<OdeSolution> {
    None
}
