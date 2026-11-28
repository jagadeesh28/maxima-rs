//! Maxima 5.50 core CAS operations (`trigexpand`, `trigreduce`, `trigsimp`,
//! `radcan`, `demoivre`, `exponentialize`, `subst`, `float`, `divide`,
//! `quotient`, `remainder`, `num`, `denom`, finite `sum`/`product`).
//!
//! Cites Maxima `src/trigi.lisp`, `src/logarc.lisp`, `src/rat3e.lisp`, and `src/comm.lisp`.

use std::collections::HashMap;

use num_traits::ToPrimitive;

use crate::expr::Expr;
use crate::extract::extract_single;
use crate::poly::gcd::poly_divrem;
use crate::poly::rat::to_fraction;
use crate::poly::{Poly, PolyError, VarContext, expand, ratsimp};
use crate::rational::Rat;
use crate::rules::RuleConfig;
use crate::saturate::{SaturationConfig, saturate};

fn imag_unit() -> Expr {
    #[cfg(feature = "complex")]
    {
        Expr::i()
    }
    #[cfg(not(feature = "complex"))]
    {
        Expr::var("%i")
    }
}

fn egraph_simplify(expr: &Expr, fast_math: bool) -> Expr {
    let cfg = SaturationConfig {
        rule_config: RuleConfig {
            fast_math,
            ..RuleConfig::default()
        },
        ..SaturationConfig::default()
    };
    let (egraph, roots) = saturate(std::slice::from_ref(expr), &cfg);
    extract_single(&egraph, roots[0])
}

/// Depth-guarded AST substitution: replace `var` with `new_val` in `expr`.
///
/// Matches Maxima's `subst(new_val, var, expr)`.
pub fn subst_expr(new_val: &Expr, var: &str, expr: &Expr) -> Expr {
    subst_inner(new_val, var, expr, 0)
}

fn subst_inner(new_val: &Expr, var: &str, expr: &Expr, depth: usize) -> Expr {
    if depth > 256 {
        return expr.clone();
    }
    match expr {
        Expr::Var(name) if name == var => new_val.clone(),
        Expr::Var(_) | Expr::Const(_) | Expr::Pi | Expr::E => expr.clone(),
        #[cfg(feature = "bfloat")]
        Expr::Bfloat(_) => expr.clone(),
        #[cfg(feature = "complex")]
        Expr::I => expr.clone(),
        Expr::Add(a, b) => Expr::add(
            subst_inner(new_val, var, a, depth + 1),
            subst_inner(new_val, var, b, depth + 1),
        ),
        Expr::Sub(a, b) => Expr::sub(
            subst_inner(new_val, var, a, depth + 1),
            subst_inner(new_val, var, b, depth + 1),
        ),
        Expr::Mul(a, b) => Expr::mul(
            subst_inner(new_val, var, a, depth + 1),
            subst_inner(new_val, var, b, depth + 1),
        ),
        Expr::Div(a, b) => Expr::div(
            subst_inner(new_val, var, a, depth + 1),
            subst_inner(new_val, var, b, depth + 1),
        ),
        Expr::Pow(a, b) => Expr::pow(
            subst_inner(new_val, var, a, depth + 1),
            subst_inner(new_val, var, b, depth + 1),
        ),
        Expr::Neg(a) => Expr::neg(subst_inner(new_val, var, a, depth + 1)),
        Expr::Sin(a) => Expr::sin(subst_inner(new_val, var, a, depth + 1)),
        Expr::Cos(a) => Expr::cos(subst_inner(new_val, var, a, depth + 1)),
        Expr::Tan(a) => Expr::tan(subst_inner(new_val, var, a, depth + 1)),
        Expr::Exp(a) => Expr::exp(subst_inner(new_val, var, a, depth + 1)),
        Expr::Ln(a) => Expr::ln(subst_inner(new_val, var, a, depth + 1)),
        Expr::Sqrt(a) => Expr::sqrt(subst_inner(new_val, var, a, depth + 1)),
        Expr::Sinh(a) => Expr::sinh(subst_inner(new_val, var, a, depth + 1)),
        Expr::Cosh(a) => Expr::cosh(subst_inner(new_val, var, a, depth + 1)),
        Expr::Tanh(a) => Expr::tanh(subst_inner(new_val, var, a, depth + 1)),
        Expr::Asin(a) => Expr::asin(subst_inner(new_val, var, a, depth + 1)),
        Expr::Acos(a) => Expr::acos(subst_inner(new_val, var, a, depth + 1)),
        Expr::Atan(a) => Expr::atan(subst_inner(new_val, var, a, depth + 1)),
        Expr::Abs(a) => Expr::abs(subst_inner(new_val, var, a, depth + 1)),
        Expr::Atan2(y, x) => Expr::atan2(
            subst_inner(new_val, var, y, depth + 1),
            subst_inner(new_val, var, x, depth + 1),
        ),
        Expr::Factorial(a) => Expr::factorial(subst_inner(new_val, var, a, depth + 1)),
        Expr::Tensor(n, idx) => Expr::Tensor(n.clone(), idx.clone()),
        Expr::Derivative(a, b) => Expr::derivative(
            subst_inner(new_val, var, a, depth + 1),
            subst_inner(new_val, var, b, depth + 1),
        ),
        Expr::Integral(a, b) => Expr::integral(
            subst_inner(new_val, var, a, depth + 1),
            subst_inner(new_val, var, b, depth + 1),
        ),
        Expr::Sum(a, b, c, d) => Expr::sum(
            subst_inner(new_val, var, a, depth + 1),
            subst_inner(new_val, var, b, depth + 1),
            subst_inner(new_val, var, c, depth + 1),
            subst_inner(new_val, var, d, depth + 1),
        ),
        Expr::Matrix(m) => Expr::matrix(
            m.iter()
                .map(|row| {
                    row.iter()
                        .map(|x| subst_inner(new_val, var, x, depth + 1))
                        .collect()
                })
                .collect(),
        ),
        #[cfg(feature = "specfn")]
        Expr::SpecialFn(kind, args) => Expr::SpecialFn(
            *kind,
            args.iter()
                .map(|x| subst_inner(new_val, var, x, depth + 1))
                .collect(),
        ),
    }
}

fn map_children(expr: &Expr, f: &impl Fn(&Expr) -> Expr) -> Expr {
    match expr {
        Expr::Add(a, b) => Expr::add(f(a), f(b)),
        Expr::Sub(a, b) => Expr::sub(f(a), f(b)),
        Expr::Mul(a, b) => Expr::mul(f(a), f(b)),
        Expr::Div(a, b) => Expr::div(f(a), f(b)),
        Expr::Pow(a, b) => Expr::pow(f(a), f(b)),
        Expr::Neg(a) => Expr::neg(f(a)),
        Expr::Sin(a) => Expr::sin(f(a)),
        Expr::Cos(a) => Expr::cos(f(a)),
        Expr::Tan(a) => Expr::tan(f(a)),
        Expr::Exp(a) => Expr::exp(f(a)),
        Expr::Ln(a) => Expr::ln(f(a)),
        Expr::Sqrt(a) => Expr::sqrt(f(a)),
        Expr::Sinh(a) => Expr::sinh(f(a)),
        Expr::Cosh(a) => Expr::cosh(f(a)),
        Expr::Tanh(a) => Expr::tanh(f(a)),
        Expr::Asin(a) => Expr::asin(f(a)),
        Expr::Acos(a) => Expr::acos(f(a)),
        Expr::Atan(a) => Expr::atan(f(a)),
        Expr::Abs(a) => Expr::abs(f(a)),
        Expr::Atan2(y, x) => Expr::atan2(f(y), f(x)),
        Expr::Factorial(a) => Expr::factorial(f(a)),
        Expr::Matrix(m) => Expr::matrix(m.iter().map(|r| r.iter().map(f).collect()).collect()),
        _ => expr.clone(),
    }
}

fn expand_sin_n(n: u32, u: &Expr) -> Expr {
    if n == 0 {
        return Expr::int(0);
    }
    if n == 1 {
        return Expr::sin(u.clone());
    }
    // sin(n*u) = sin(u)*cos((n-1)*u) + cos(u)*sin((n-1)*u)
    Expr::add(
        Expr::mul(Expr::sin(u.clone()), expand_cos_n(n - 1, u)),
        Expr::mul(Expr::cos(u.clone()), expand_sin_n(n - 1, u)),
    )
}

fn expand_cos_n(n: u32, u: &Expr) -> Expr {
    if n == 0 {
        return Expr::int(1);
    }
    if n == 1 {
        return Expr::cos(u.clone());
    }
    // cos(n*u) = cos(u)*cos((n-1)*u) - sin(u)*sin((n-1)*u)
    Expr::sub(
        Expr::mul(Expr::cos(u.clone()), expand_cos_n(n - 1, u)),
        Expr::mul(Expr::sin(u.clone()), expand_sin_n(n - 1, u)),
    )
}

fn extract_small_int_factor(e: &Expr) -> Option<(i64, Expr)> {
    if let Expr::Mul(a, b) = e {
        if let Expr::Const(r) = a.as_ref()
            && Rat::is_integer(r)
            && let Some(k) = r.numer().to_i64()
            && (2..=8).contains(&k.abs())
        {
            return Some((k, (**b).clone()));
        }
        if let Expr::Const(r) = b.as_ref()
            && Rat::is_integer(r)
            && let Some(k) = r.numer().to_i64()
            && (2..=8).contains(&k.abs())
        {
            return Some((k, (**a).clone()));
        }
    }
    None
}

/// Expand trigonometric and hyperbolic angle sums and integer multiples (`sin(a+b)`, `cos(2*x)`).
pub fn trigexpand(expr: &Expr) -> Expr {
    trigexpand_inner(expr, 0)
}

fn trigexpand_inner(expr: &Expr, depth: usize) -> Expr {
    if depth > 256 {
        return expr.clone();
    }
    match expr {
        Expr::Sin(arg) => {
            let u = trigexpand_inner(arg, depth + 1);
            match &u {
                Expr::Add(a, b) => Expr::add(
                    Expr::mul(
                        trigexpand_inner(&Expr::sin((**a).clone()), depth + 1),
                        trigexpand_inner(&Expr::cos((**b).clone()), depth + 1),
                    ),
                    Expr::mul(
                        trigexpand_inner(&Expr::cos((**a).clone()), depth + 1),
                        trigexpand_inner(&Expr::sin((**b).clone()), depth + 1),
                    ),
                ),
                Expr::Sub(a, b) => Expr::sub(
                    Expr::mul(
                        trigexpand_inner(&Expr::sin((**a).clone()), depth + 1),
                        trigexpand_inner(&Expr::cos((**b).clone()), depth + 1),
                    ),
                    Expr::mul(
                        trigexpand_inner(&Expr::cos((**a).clone()), depth + 1),
                        trigexpand_inner(&Expr::sin((**b).clone()), depth + 1),
                    ),
                ),
                Expr::Neg(a) => Expr::neg(trigexpand_inner(&Expr::sin((**a).clone()), depth + 1)),
                _ => {
                    if let Some((k, base)) = extract_small_int_factor(&u) {
                        let res = expand_sin_n(k.unsigned_abs() as u32, &base);
                        if k < 0 { Expr::neg(res) } else { res }
                    } else {
                        Expr::sin(u)
                    }
                }
            }
        }
        Expr::Cos(arg) => {
            let u = trigexpand_inner(arg, depth + 1);
            match &u {
                Expr::Add(a, b) => Expr::sub(
                    Expr::mul(
                        trigexpand_inner(&Expr::cos((**a).clone()), depth + 1),
                        trigexpand_inner(&Expr::cos((**b).clone()), depth + 1),
                    ),
                    Expr::mul(
                        trigexpand_inner(&Expr::sin((**a).clone()), depth + 1),
                        trigexpand_inner(&Expr::sin((**b).clone()), depth + 1),
                    ),
                ),
                Expr::Sub(a, b) => Expr::add(
                    Expr::mul(
                        trigexpand_inner(&Expr::cos((**a).clone()), depth + 1),
                        trigexpand_inner(&Expr::cos((**b).clone()), depth + 1),
                    ),
                    Expr::mul(
                        trigexpand_inner(&Expr::sin((**a).clone()), depth + 1),
                        trigexpand_inner(&Expr::sin((**b).clone()), depth + 1),
                    ),
                ),
                Expr::Neg(a) => trigexpand_inner(&Expr::cos((**a).clone()), depth + 1),
                _ => {
                    if let Some((k, base)) = extract_small_int_factor(&u) {
                        expand_cos_n(k.unsigned_abs() as u32, &base)
                    } else {
                        Expr::cos(u)
                    }
                }
            }
        }
        Expr::Tan(arg) => {
            let u = trigexpand_inner(arg, depth + 1);
            match &u {
                Expr::Add(a, b) => {
                    let ta = trigexpand_inner(&Expr::tan((**a).clone()), depth + 1);
                    let tb = trigexpand_inner(&Expr::tan((**b).clone()), depth + 1);
                    Expr::div(
                        Expr::add(ta.clone(), tb.clone()),
                        Expr::sub(Expr::int(1), Expr::mul(ta, tb)),
                    )
                }
                Expr::Sub(a, b) => {
                    let ta = trigexpand_inner(&Expr::tan((**a).clone()), depth + 1);
                    let tb = trigexpand_inner(&Expr::tan((**b).clone()), depth + 1);
                    Expr::div(
                        Expr::sub(ta.clone(), tb.clone()),
                        Expr::add(Expr::int(1), Expr::mul(ta, tb)),
                    )
                }
                Expr::Neg(a) => Expr::neg(trigexpand_inner(&Expr::tan((**a).clone()), depth + 1)),
                _ => Expr::tan(u),
            }
        }
        Expr::Sinh(arg) => {
            let u = trigexpand_inner(arg, depth + 1);
            match &u {
                Expr::Add(a, b) => Expr::add(
                    Expr::mul(Expr::sinh((**a).clone()), Expr::cosh((**b).clone())),
                    Expr::mul(Expr::cosh((**a).clone()), Expr::sinh((**b).clone())),
                ),
                Expr::Sub(a, b) => Expr::sub(
                    Expr::mul(Expr::sinh((**a).clone()), Expr::cosh((**b).clone())),
                    Expr::mul(Expr::cosh((**a).clone()), Expr::sinh((**b).clone())),
                ),
                _ => Expr::sinh(u),
            }
        }
        Expr::Cosh(arg) => {
            let u = trigexpand_inner(arg, depth + 1);
            match &u {
                Expr::Add(a, b) => Expr::add(
                    Expr::mul(Expr::cosh((**a).clone()), Expr::cosh((**b).clone())),
                    Expr::mul(Expr::sinh((**a).clone()), Expr::sinh((**b).clone())),
                ),
                Expr::Sub(a, b) => Expr::sub(
                    Expr::mul(Expr::cosh((**a).clone()), Expr::cosh((**b).clone())),
                    Expr::mul(Expr::sinh((**a).clone()), Expr::sinh((**b).clone())),
                ),
                _ => Expr::cosh(u),
            }
        }
        _ => map_children(expr, &|c| trigexpand_inner(c, depth + 1)),
    }
}

/// Reduce products and powers of trigonometric functions into linear combinations
/// of sines and cosines of multiple angles (`sin(x)*cos(x) -> sin(2*x)/2`).
pub fn trigreduce(expr: &Expr) -> Expr {
    let mut cur = expand(expr);
    for _ in 0..4 {
        let next = trigreduce_pass(&cur, 0);
        let next_exp = expand(&next);
        if next_exp == cur {
            return egraph_simplify(&next_exp, true);
        }
        cur = next_exp;
    }
    egraph_simplify(&cur, true)
}

fn trigreduce_pass(expr: &Expr, depth: usize) -> Expr {
    if depth > 256 {
        return expr.clone();
    }
    match expr {
        Expr::Pow(base, exp) => {
            if let Expr::Const(r) = exp.as_ref()
                && Rat::is_integer(r)
                && let Some(n) = r.numer().to_i64()
                && (2..=8).contains(&n)
            {
                match base.as_ref() {
                    Expr::Sin(u) => {
                        let two_u = Expr::mul(Expr::int(2), (**u).clone());
                        let sq = Expr::div(Expr::sub(Expr::int(1), Expr::cos(two_u)), Expr::int(2));
                        if n == 2 {
                            return sq;
                        }
                        return Expr::mul(
                            sq,
                            trigreduce_pass(
                                &Expr::pow((**base).clone(), Expr::int(n - 2)),
                                depth + 1,
                            ),
                        );
                    }
                    Expr::Cos(u) => {
                        let two_u = Expr::mul(Expr::int(2), (**u).clone());
                        let sq = Expr::div(Expr::add(Expr::int(1), Expr::cos(two_u)), Expr::int(2));
                        if n == 2 {
                            return sq;
                        }
                        return Expr::mul(
                            sq,
                            trigreduce_pass(
                                &Expr::pow((**base).clone(), Expr::int(n - 2)),
                                depth + 1,
                            ),
                        );
                    }
                    _ => {}
                }
            }
            Expr::pow(trigreduce_pass(base, depth + 1), (**exp).clone())
        }
        Expr::Mul(a, b) => {
            let la = trigreduce_pass(a, depth + 1);
            let rb = trigreduce_pass(b, depth + 1);
            match (&la, &rb) {
                (Expr::Sin(u), Expr::Cos(v)) | (Expr::Cos(v), Expr::Sin(u)) => Expr::mul(
                    Expr::frac(1, 2),
                    Expr::add(
                        Expr::sin(Expr::add((**u).clone(), (**v).clone())),
                        Expr::sin(Expr::sub((**u).clone(), (**v).clone())),
                    ),
                ),
                (Expr::Cos(u), Expr::Cos(v)) => Expr::mul(
                    Expr::frac(1, 2),
                    Expr::add(
                        Expr::cos(Expr::add((**u).clone(), (**v).clone())),
                        Expr::cos(Expr::sub((**u).clone(), (**v).clone())),
                    ),
                ),
                (Expr::Sin(u), Expr::Sin(v)) => Expr::mul(
                    Expr::frac(1, 2),
                    Expr::sub(
                        Expr::cos(Expr::sub((**u).clone(), (**v).clone())),
                        Expr::cos(Expr::add((**u).clone(), (**v).clone())),
                    ),
                ),
                _ => Expr::mul(la, rb),
            }
        }
        _ => map_children(expr, &|c| trigreduce_pass(c, depth + 1)),
    }
}

/// Simplify trigonometric/hyperbolic expressions by converting `tan`/`tanh` to
/// `sin/cos`/`sinh/cosh`, applying rational simplification, and e-graph Pythagorean identities.
pub fn trigsimp(expr: &Expr) -> Expr {
    let rewritten = tan_to_sin_cos(expr, 0);
    let rat = ratsimp(&rewritten).unwrap_or(rewritten);
    egraph_simplify(&rat, true)
}

fn tan_to_sin_cos(expr: &Expr, depth: usize) -> Expr {
    if depth > 256 {
        return expr.clone();
    }
    match expr {
        Expr::Tan(u) => {
            let inner = tan_to_sin_cos(u, depth + 1);
            Expr::div(Expr::sin(inner.clone()), Expr::cos(inner))
        }
        Expr::Tanh(u) => {
            let inner = tan_to_sin_cos(u, depth + 1);
            Expr::div(Expr::sinh(inner.clone()), Expr::cosh(inner))
        }
        _ => map_children(expr, &|c| tan_to_sin_cos(c, depth + 1)),
    }
}

/// Canonicalize radicals, logarithms, and exponentials (`radcan`).
pub fn radcan(expr: &Expr) -> Expr {
    let simp = egraph_simplify(expr, true);
    let rat = ratsimp(&simp).unwrap_or(simp);
    egraph_simplify(&rat, true)
}

/// Convert trigonometric and hyperbolic functions into exponential form
/// (citing Maxima `logarc.lisp` `$exponentialize`).
pub fn exponentialize(expr: &Expr) -> Expr {
    exponentialize_inner(expr, 0)
}

fn exponentialize_inner(expr: &Expr, depth: usize) -> Expr {
    if depth > 256 {
        return expr.clone();
    }
    let i_unit = imag_unit();
    match expr {
        Expr::Sin(u) => {
            let inner = exponentialize_inner(u, depth + 1);
            let iu = Expr::mul(i_unit.clone(), inner);
            Expr::div(
                Expr::sub(Expr::exp(iu.clone()), Expr::exp(Expr::neg(iu))),
                Expr::mul(Expr::int(2), i_unit),
            )
        }
        Expr::Cos(u) => {
            let inner = exponentialize_inner(u, depth + 1);
            let iu = Expr::mul(i_unit, inner);
            Expr::div(
                Expr::add(Expr::exp(iu.clone()), Expr::exp(Expr::neg(iu))),
                Expr::int(2),
            )
        }
        Expr::Tan(u) => {
            let inner = exponentialize_inner(u, depth + 1);
            let iu = Expr::mul(i_unit.clone(), inner);
            Expr::div(
                Expr::mul(
                    i_unit,
                    Expr::sub(Expr::exp(Expr::neg(iu.clone())), Expr::exp(iu.clone())),
                ),
                Expr::add(Expr::exp(iu.clone()), Expr::exp(Expr::neg(iu))),
            )
        }
        Expr::Sinh(u) => {
            let inner = exponentialize_inner(u, depth + 1);
            Expr::div(
                Expr::sub(Expr::exp(inner.clone()), Expr::exp(Expr::neg(inner))),
                Expr::int(2),
            )
        }
        Expr::Cosh(u) => {
            let inner = exponentialize_inner(u, depth + 1);
            Expr::div(
                Expr::add(Expr::exp(inner.clone()), Expr::exp(Expr::neg(inner))),
                Expr::int(2),
            )
        }
        Expr::Tanh(u) => {
            let inner = exponentialize_inner(u, depth + 1);
            Expr::div(
                Expr::sub(
                    Expr::exp(inner.clone()),
                    Expr::exp(Expr::neg(inner.clone())),
                ),
                Expr::add(Expr::exp(inner.clone()), Expr::exp(Expr::neg(inner))),
            )
        }
        _ => map_children(expr, &|c| exponentialize_inner(c, depth + 1)),
    }
}

/// Convert complex exponentials into trigonometric form via Euler's formula
/// (citing Maxima `logarc.lisp` `$demoivre`).
pub fn demoivre(expr: &Expr) -> Expr {
    demoivre_inner(expr, 0)
}

fn is_imag_atom(e: &Expr) -> bool {
    #[cfg(feature = "complex")]
    if matches!(e, Expr::I) {
        return true;
    }
    matches!(e, Expr::Var(v) if v == "%i")
}

fn split_complex_exp_arg(arg: &Expr) -> (Option<Expr>, Option<Expr>) {
    // Returns (real_part, imag_coeff) if arg has an explicit %i factor
    match arg {
        Expr::Add(a, b) => {
            let (ra, ia) = split_complex_exp_arg(a);
            let (rb, ib) = split_complex_exp_arg(b);
            let r = match (ra, rb) {
                (Some(x), Some(y)) => Some(Expr::add(x, y)),
                (Some(x), None) | (None, Some(x)) => Some(x),
                (None, None) => None,
            };
            let im = match (ia, ib) {
                (Some(x), Some(y)) => Some(Expr::add(x, y)),
                (Some(x), None) | (None, Some(x)) => Some(x),
                (None, None) => None,
            };
            (r, im)
        }
        Expr::Sub(a, b) => {
            let (ra, ia) = split_complex_exp_arg(a);
            let (rb, ib) = split_complex_exp_arg(b);
            let r = match (ra, rb) {
                (Some(x), Some(y)) => Some(Expr::sub(x, y)),
                (Some(x), None) => Some(x),
                (None, Some(y)) => Some(Expr::neg(y)),
                (None, None) => None,
            };
            let im = match (ia, ib) {
                (Some(x), Some(y)) => Some(Expr::sub(x, y)),
                (Some(x), None) => Some(x),
                (None, Some(y)) => Some(Expr::neg(y)),
                (None, None) => None,
            };
            (r, im)
        }
        Expr::Neg(a) => {
            let (r, im) = split_complex_exp_arg(a);
            (r.map(Expr::neg), im.map(Expr::neg))
        }
        Expr::Mul(a, b) => {
            if is_imag_atom(a) {
                (None, Some((**b).clone()))
            } else if is_imag_atom(b) {
                (None, Some((**a).clone()))
            } else {
                (Some(arg.clone()), None)
            }
        }
        _ if is_imag_atom(arg) => (None, Some(Expr::int(1))),
        _ => (Some(arg.clone()), None),
    }
}

fn demoivre_inner(expr: &Expr, depth: usize) -> Expr {
    if depth > 256 {
        return expr.clone();
    }
    let convert_exp = |arg: &Expr| -> Expr {
        let (real_opt, imag_opt) = split_complex_exp_arg(arg);
        match imag_opt {
            Some(im) => {
                let trig = Expr::add(Expr::cos(im.clone()), Expr::mul(imag_unit(), Expr::sin(im)));
                match real_opt {
                    Some(r) => Expr::mul(Expr::exp(r), trig),
                    None => trig,
                }
            }
            None => Expr::exp(arg.clone()),
        }
    };
    match expr {
        Expr::Exp(arg) => convert_exp(&demoivre_inner(arg, depth + 1)),
        Expr::Pow(base, exp) if matches!(base.as_ref(), Expr::E) => {
            convert_exp(&demoivre_inner(exp, depth + 1))
        }
        _ => map_children(expr, &|c| demoivre_inner(c, depth + 1)),
    }
}

/// Evaluate a constant expression numerically to `f64` (`float(expr)`).
pub fn eval_float(expr: &Expr) -> Option<Expr> {
    let empty = HashMap::new();
    let val = expr.eval(&empty)?;
    Some(Expr::from_f64(val))
}

/// Polynomial division `divide(p, q, [var])` returning `(quotient, remainder)`.
pub fn poly_divide(
    p_expr: &Expr,
    q_expr: &Expr,
    var_hint: Option<&str>,
) -> Result<(Expr, Expr), PolyError> {
    let mut ctx = VarContext::new();
    if let Some(vh) = var_hint {
        ctx.add(vh);
    }
    for v in p_expr.variables() {
        ctx.add(&v);
    }
    for v in q_expr.variables() {
        ctx.add(&v);
    }
    let p_frac = to_fraction(p_expr, &ctx)?;
    let q_frac = to_fraction(q_expr, &ctx)?;
    if !p_frac.den.is_constant() || !q_frac.den.is_constant() {
        let p = Poly::from_expr(p_expr)?;
        let q = Poly::from_expr(q_expr)?;
        if q.is_zero() {
            return Err(PolyError::DivisionByZero);
        }
        let (quot, rem) = poly_divrem(&p, &q).ok_or(PolyError::DivisionByZero)?;
        return Ok((quot.to_expr(), rem.to_expr()));
    }
    if q_frac.num.is_zero() {
        return Err(PolyError::DivisionByZero);
    }
    let p_den = p_frac
        .den
        .terms()
        .first()
        .map(|t| t.coeff.clone())
        .unwrap_or_else(|| num_rational::BigRational::from_integer(1.into()));
    let q_den = q_frac
        .den
        .terms()
        .first()
        .map(|t| t.coeff.clone())
        .unwrap_or_else(|| num_rational::BigRational::from_integer(1.into()));
    let p_scaled = p_frac
        .num
        .scale(&(num_rational::BigRational::from_integer(1.into()) / p_den));
    let q_scaled = q_frac
        .num
        .scale(&(num_rational::BigRational::from_integer(1.into()) / q_den));
    let (quot, rem) = poly_divrem(&p_scaled, &q_scaled).ok_or(PolyError::DivisionByZero)?;
    Ok((quot.to_expr(), rem.to_expr()))
}

/// Extract `(numerator, denominator)` of a rational expression (`num` / `denom`).
pub fn num_denom(expr: &Expr) -> Result<(Expr, Expr), PolyError> {
    let mut ctx = VarContext::new();
    for v in expr.variables() {
        ctx.add(&v);
    }
    let frac = to_fraction(expr, &ctx)?;
    Ok((frac.num.to_expr(), frac.den.to_expr()))
}

const MAX_FINITE_AGG_TERMS: i64 = 10_000;

/// Evaluate a finite `sum(body, var, lo, hi)` or `product(body, var, lo, hi)`.
pub fn eval_finite_aggregate(
    body: &Expr,
    var: &str,
    lo: &Expr,
    hi: &Expr,
    is_product: bool,
) -> Result<Expr, String> {
    let lo_i = expr_to_i64(lo).ok_or_else(|| format!("lower bound '{lo}' is not an integer"))?;
    let hi_i = expr_to_i64(hi).ok_or_else(|| format!("upper bound '{hi}' is not an integer"))?;
    if hi_i < lo_i {
        return Ok(Expr::int(if is_product { 1 } else { 0 }));
    }
    let count = hi_i.saturating_sub(lo_i).saturating_add(1);
    if count > MAX_FINITE_AGG_TERMS {
        return Err(format!(
            "finite aggregate range ({count} terms) exceeds maximum of {MAX_FINITE_AGG_TERMS}"
        ));
    }
    let mut acc = Expr::int(if is_product { 1 } else { 0 });
    for k in lo_i..=hi_i {
        if crate::cancel::is_cancelled() {
            return Err("cancelled".to_string());
        }
        let term = subst_expr(&Expr::int(k), var, body);
        if k == lo_i {
            acc = term;
        } else if is_product {
            acc = Expr::mul(acc, term);
        } else {
            acc = Expr::add(acc, term);
        }
        if (k - lo_i) % 16 == 15 {
            acc = egraph_simplify(&acc, false);
        }
    }
    Ok(egraph_simplify(&acc, false))
}

fn expr_to_i64(e: &Expr) -> Option<i64> {
    match e {
        Expr::Const(r) if Rat::is_integer(r) => r.numer().to_i64(),
        Expr::Neg(inner) => expr_to_i64(inner).and_then(|v| v.checked_neg()),
        _ => None,
    }
}
