//! Asymptotic limits (Phase 27 / Phase 49).
//!
//! `limit(expr, var, point)` and `limit_dir(expr, var, point, dir)` compute
//! exact symbolic limits via:
//! 1. Truncated Laurent/Taylor series expansion ([`crate::series`]) with gensym
//!    substitution for `inf`/`minf` and pole/oscillation detection (`inf`, `minf`, `und`, `ind`).
//! 2. Gruntz MRV asymptotic reduction ([`mrv`]) for `exp`/`ln` towers.
//! 3. L'Hôpital's rule ([`crate::diff::differentiate`]) for `0/0` and `inf/inf` forms.
//!
//! Unsound numeric convergence guessing is removed per ADR-0012.

pub mod mrv;
pub mod zero;

use crate::diff::differentiate;
use crate::expr::Expr;
use crate::series::series_expand;
use std::collections::HashMap;

/// Direction of approach for a limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitDir {
    /// Two-sided limit.
    Both,
    /// Right-hand limit (`x -> point+`).
    Plus,
    /// Left-hand limit (`x -> point-`).
    Minus,
}

fn is_pos_inf(point: &Expr) -> bool {
    matches!(point, Expr::Var(n) if n.as_str() == "inf")
}

fn is_neg_inf(point: &Expr) -> bool {
    matches!(point, Expr::Var(n) if n.as_str() == "minf")
}

fn is_inf_like(e: &Expr) -> bool {
    matches!(e, Expr::Var(n) if n == "inf" || n == "minf")
}

fn is_ind_or_und(e: &Expr) -> bool {
    matches!(e, Expr::Var(n) if n == "und" || n == "ind")
}

/// Generate a fresh variable name not occurring in `expr`.
pub(super) fn fresh_var(expr: &Expr, prefix: &str) -> String {
    let vars = expr.variables();
    for i in 0..10_000 {
        let cand = format!("{prefix}_{i}");
        if !vars.contains(&cand) {
            return cand;
        }
    }
    format!("{prefix}_fallback")
}

/// Replace every occurrence of variable `name` with `r`.
pub(super) fn subst_var(e: &Expr, name: &str, r: &Expr) -> Expr {
    if let Expr::Var(n) = e
        && n.as_str() == name
    {
        return r.clone();
    }
    match e {
        Expr::Var(_) | Expr::Const(_) | Expr::Pi | Expr::E => e.clone(),
        #[cfg(feature = "bfloat")]
        Expr::Bfloat(_) => e.clone(),
        #[cfg(feature = "complex")]
        Expr::I => e.clone(),
        Expr::Add(a, b) => Expr::add(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Sub(a, b) => Expr::sub(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Mul(a, b) => Expr::mul(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Div(a, b) => Expr::div(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Pow(a, b) => Expr::pow(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Neg(a) => Expr::neg(subst_var(a, name, r)),
        Expr::Sin(a) => Expr::sin(subst_var(a, name, r)),
        Expr::Cos(a) => Expr::cos(subst_var(a, name, r)),
        Expr::Tan(a) => Expr::tan(subst_var(a, name, r)),
        Expr::Exp(a) => Expr::exp(subst_var(a, name, r)),
        Expr::Ln(a) => Expr::ln(subst_var(a, name, r)),
        Expr::Sqrt(a) => Expr::sqrt(subst_var(a, name, r)),
        Expr::Sinh(a) => Expr::sinh(subst_var(a, name, r)),
        Expr::Cosh(a) => Expr::cosh(subst_var(a, name, r)),
        Expr::Tanh(a) => Expr::tanh(subst_var(a, name, r)),
        Expr::Asin(a) => Expr::asin(subst_var(a, name, r)),
        Expr::Acos(a) => Expr::acos(subst_var(a, name, r)),
        Expr::Atan(a) => Expr::atan(subst_var(a, name, r)),
        Expr::Abs(a) => Expr::abs(subst_var(a, name, r)),
        Expr::Atan2(y, x) => Expr::atan2(subst_var(y, name, r), subst_var(x, name, r)),
        Expr::Factorial(a) => Expr::factorial(subst_var(a, name, r)),
        Expr::Tensor(n, idx) => Expr::Tensor(n.clone(), idx.clone()),
        Expr::Derivative(a, b) => Expr::derivative(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Integral(a, b) => Expr::integral(subst_var(a, name, r), subst_var(b, name, r)),
        Expr::Sum(a, b, c, d) => Expr::sum(
            subst_var(a, name, r),
            subst_var(b, name, r),
            subst_var(c, name, r),
            subst_var(d, name, r),
        ),
        Expr::Matrix(m) => Expr::matrix(
            m.iter()
                .map(|row| row.iter().map(|x| subst_var(x, name, r)).collect())
                .collect(),
        ),
        #[cfg(feature = "specfn")]
        Expr::SpecialFn(kind, args) => {
            Expr::SpecialFn(*kind, args.iter().map(|x| subst_var(x, name, r)).collect())
        }
    }
}

#[cfg(feature = "assume")]
use crate::assume::{Assumptions, Sign};

/// Compute the two-sided limit of `expr` as `var` approaches `point`.
///
/// `point` is an [`Expr`] constant for finite limits, or `Expr::var("inf")` /
/// `Expr::var("minf")` for infinite limits. Returns `None` when the limit cannot
/// be determined, or `inf`/`minf`/`und`/`ind` when known.
pub fn limit(expr: &Expr, var: &str, point: &Expr) -> Option<Expr> {
    limit_dir(expr, var, point, LimitDir::Both)
}

/// Compute a directional (`Both`, `Plus`, `Minus`) limit of `expr` as `var` approaches `point`.
pub fn limit_dir(expr: &Expr, var: &str, point: &Expr, dir: LimitDir) -> Option<Expr> {
    #[cfg(feature = "assume")]
    {
        limit_dir_with_assumptions(expr, var, point, dir, None)
    }
    #[cfg(not(feature = "assume"))]
    {
        let _ = dir;
        limit_at_depth(expr, var, point, dir, 0)
    }
}

/// Compute limit with optional [`Assumptions`] context.
#[cfg(feature = "assume")]
pub fn limit_with_assumptions(
    expr: &Expr,
    var: &str,
    point: &Expr,
    assumptions: Option<&Assumptions>,
) -> Option<Expr> {
    limit_dir_with_assumptions(expr, var, point, LimitDir::Both, assumptions)
}

/// Compute directional limit with optional [`Assumptions`] context.
#[cfg(feature = "assume")]
pub fn limit_dir_with_assumptions(
    expr: &Expr,
    var: &str,
    point: &Expr,
    dir: LimitDir,
    assumptions: Option<&Assumptions>,
) -> Option<Expr> {
    if !expr.variables().iter().any(|v| v == var) {
        return Some(expr.clone());
    }
    // Check monomial power with assumptions as point -> inf
    if let Expr::Pow(base, exp) = expr
        && matches!(base.as_ref(), Expr::Var(v) if v == var)
        && is_pos_inf(point)
        && !exp.variables().contains(&var.to_string())
        && let Some(ctx) = assumptions
    {
        match ctx.csign(exp) {
            Sign::Pos => return Some(Expr::var("inf")),
            Sign::Neg => return Some(Expr::int(0)),
            Sign::Zero => return Some(Expr::int(1)),
            _ => {}
        }
    }
    limit_at_depth(expr, var, point, dir, 0)
}

pub(super) fn limit_at_depth(
    expr: &Expr,
    var: &str,
    point: &Expr,
    dir: LimitDir,
    depth: usize,
) -> Option<Expr> {
    if depth > 12 {
        return None;
    }
    if !expr.variables().iter().any(|v| v == var) {
        return Some(expr.clone());
    }
    // Check oscillatory trig at infinity
    if (is_pos_inf(point) || is_neg_inf(point))
        && let Some(osc) = check_oscillatory(expr, var, point, depth)
    {
        return Some(osc);
    }
    if let Some(r) = symbolic_limit(expr, var, point, dir) {
        return Some(r);
    }
    if let Expr::Exp(arg) = expr
        && let Some(l) = limit_at_depth(arg, var, point, dir, depth + 1)
    {
        return exp_limit_of(&l);
    }
    if let Expr::Pow(base, exp) = expr {
        let e_arg = Expr::mul((**exp).clone(), Expr::ln((**base).clone()));
        if let Some(l) = limit_at_depth(&e_arg, var, point, dir, depth + 1) {
            return exp_limit_of(&l);
        }
    }
    if let Some(r) = limit_exp_difference(expr, var, point, dir, depth) {
        return Some(r);
    }
    if is_pos_inf(point)
        && let Some(r) = mrv::gruntz_limit_inf(expr, var, depth)
    {
        return Some(r);
    }
    if is_neg_inf(point) {
        let neg_expr = subst_var(expr, var, &Expr::neg(Expr::var(var)));
        if let Some(r) = mrv::gruntz_limit_inf(&neg_expr, var, depth) {
            return Some(r);
        }
    }
    if let Expr::Ln(inner) = expr
        && let Some(li) = limit_at_depth(inner, var, point, dir, depth + 1)
    {
        if matches!(&li, Expr::Var(n) if n == "inf") {
            return Some(Expr::var("inf"));
        }
        if li.is_zero() && (dir == LimitDir::Plus || is_pos_inf(point) || is_neg_inf(point)) {
            return Some(Expr::var("minf"));
        }
    }
    if let Some(r) = lhopital_limit(expr, var, point, dir, depth) {
        return Some(r);
    }
    None
}

fn check_oscillatory(expr: &Expr, var: &str, point: &Expr, depth: usize) -> Option<Expr> {
    match expr {
        Expr::Sin(inner) | Expr::Cos(inner) => {
            if let Some(l) = limit_at_depth(inner, var, point, LimitDir::Both, depth + 1)
                && is_inf_like(&l)
            {
                return Some(Expr::var("ind"));
            }
            None
        }
        _ => None,
    }
}

fn exp_limit_of(l: &Expr) -> Option<Expr> {
    if l.is_zero() {
        Some(Expr::int(1))
    } else if l.is_one() {
        Some(Expr::e())
    } else if matches!(l, Expr::Var(n) if n == "inf") {
        Some(Expr::var("inf"))
    } else if matches!(l, Expr::Var(n) if n == "minf") {
        Some(Expr::int(0))
    } else if is_ind_or_und(l) {
        None
    } else {
        Some(Expr::exp(l.clone()))
    }
}

/// Light algebraic simplification focused on cancellation of common subterms.
pub(super) fn simplify_light(e: Expr) -> Expr {
    match e {
        Expr::Neg(a) => {
            let a = simplify_light(*a);
            if a.is_zero() {
                return Expr::int(0);
            }
            if let Expr::Neg(inner) = a {
                return *inner;
            }
            Expr::Neg(Box::new(a))
        }
        Expr::Sub(a, b) => {
            let a = simplify_light(*a);
            let b = simplify_light(*b);
            if a == b {
                return Expr::int(0);
            }
            if b.is_zero() {
                return a;
            }
            if a.is_zero() {
                return simplify_light(Expr::Neg(Box::new(b)));
            }
            if let Expr::Add(p, q) = &a {
                if **p == b {
                    return (**q).clone();
                }
                if **q == b {
                    return (**p).clone();
                }
            }
            Expr::Sub(Box::new(a), Box::new(b))
        }
        Expr::Add(a, b) => {
            let a = simplify_light(*a);
            let b = simplify_light(*b);
            if a.is_zero() {
                return b;
            }
            if b.is_zero() {
                return a;
            }
            if let Expr::Neg(inner) = &b
                && **inner == a
            {
                return Expr::int(0);
            }
            if let Expr::Neg(inner) = &a
                && **inner == b
            {
                return Expr::int(0);
            }
            Expr::Add(Box::new(a), Box::new(b))
        }
        Expr::Mul(a, b) => {
            let a = simplify_light(*a);
            let b = simplify_light(*b);
            if a.is_zero() || b.is_zero() {
                return Expr::int(0);
            }
            if a.is_one() {
                return b;
            }
            if b.is_one() {
                return a;
            }
            if matches!(&a, Expr::Const(r) if *r == crate::rational::Rat::from_int(-1)) {
                return simplify_light(Expr::Neg(Box::new(b)));
            }
            if matches!(&b, Expr::Const(r) if *r == crate::rational::Rat::from_int(-1)) {
                return simplify_light(Expr::Neg(Box::new(a)));
            }
            if let (Expr::Exp(ea), Expr::Exp(eb)) = (&a, &b) {
                let sum = simplify_light(Expr::add((**ea).clone(), (**eb).clone()));
                if sum.is_zero() {
                    return Expr::int(1);
                }
                return Expr::exp(sum);
            }
            Expr::Mul(Box::new(a), Box::new(b))
        }
        other => other,
    }
}

/// `e^A - e^B ≈ e^B·(A-B)` when `A - B → 0` (first-order leading term).
fn limit_exp_difference(
    expr: &Expr,
    var: &str,
    point: &Expr,
    dir: LimitDir,
    depth: usize,
) -> Option<Expr> {
    let Expr::Sub(a, b) = expr else {
        return None;
    };
    let (Expr::Exp(aa), Expr::Exp(bb)) = (a.as_ref(), b.as_ref()) else {
        return None;
    };
    let diff = simplify_light(Expr::sub((**aa).clone(), (**bb).clone()));
    let ld = limit_at_depth(&diff, var, point, dir, depth + 1)?;
    if !ld.is_zero() {
        return None;
    }
    let approx = simplify_light(Expr::mul(Expr::exp((**bb).clone()), diff));
    limit_at_depth(&approx, var, point, dir, depth + 1)
}

fn coeff_sign(c: &Expr) -> Option<i8> {
    let empty = HashMap::new();
    if let Some(v) = c.eval(&empty)
        && v.is_finite()
        && v != 0.0
    {
        return Some(if v > 0.0 { 1 } else { -1 });
    }
    #[cfg(feature = "assume")]
    {
        match Assumptions::new().csign(c) {
            Sign::Pos => return Some(1),
            Sign::Neg => return Some(-1),
            _ => {}
        }
    }
    None
}

fn pole_limit(c: &Expr, valuation: i64, dir: LimitDir) -> Option<Expr> {
    let sgn = coeff_sign(c)?;
    let even_pole = valuation % 2 == 0;
    match dir {
        LimitDir::Plus => {
            if sgn > 0 {
                Some(Expr::var("inf"))
            } else {
                Some(Expr::var("minf"))
            }
        }
        LimitDir::Minus => {
            let eff = if even_pole { sgn } else { -sgn };
            if eff > 0 {
                Some(Expr::var("inf"))
            } else {
                Some(Expr::var("minf"))
            }
        }
        LimitDir::Both => {
            if even_pole {
                if sgn > 0 {
                    Some(Expr::var("inf"))
                } else {
                    Some(Expr::var("minf"))
                }
            } else {
                Some(Expr::var("und"))
            }
        }
    }
}

fn symbolic_limit(expr: &Expr, var: &str, point: &Expr, dir: LimitDir) -> Option<Expr> {
    if is_pos_inf(point) {
        let t_name = fresh_var(expr, "__lim_t");
        let sub = subst_var(expr, var, &Expr::div(Expr::int(1), Expr::var(&t_name)));
        let s = series_expand(&sub, &t_name, &Expr::int(0), 8)?;
        return if s.valuation >= 0 {
            s.constant_term()
        } else {
            pole_limit(&s.coeff_at(s.valuation), s.valuation, LimitDir::Plus)
        };
    }
    if is_neg_inf(point) {
        let t_name = fresh_var(expr, "__lim_t");
        let sub = subst_var(
            expr,
            var,
            &Expr::neg(Expr::div(Expr::int(1), Expr::var(&t_name))),
        );
        let s = series_expand(&sub, &t_name, &Expr::int(0), 8)?;
        return if s.valuation >= 0 {
            s.constant_term()
        } else {
            pole_limit(&s.coeff_at(s.valuation), s.valuation, LimitDir::Plus)
        };
    }
    let s = series_expand(expr, var, point, 8)?;
    if s.valuation >= 0 {
        s.constant_term()
    } else {
        pole_limit(&s.coeff_at(s.valuation), s.valuation, dir)
    }
}

fn lhopital_limit(
    expr: &Expr,
    var: &str,
    point: &Expr,
    dir: LimitDir,
    depth: usize,
) -> Option<Expr> {
    if depth >= 4 {
        return None;
    }
    let (num, den) = match expr {
        Expr::Div(a, b) => ((**a).clone(), (**b).clone()),
        Expr::Mul(a, b) => {
            let la = limit_at_depth(a, var, point, dir, depth + 1);
            let lb = limit_at_depth(b, var, point, dir, depth + 1);
            match (la, lb) {
                (Some(va), Some(vb)) if va.is_zero() && is_inf_like(&vb) => {
                    ((**a).clone(), Expr::div(Expr::int(1), (**b).clone()))
                }
                (Some(va), Some(vb)) if is_inf_like(&va) && vb.is_zero() => {
                    ((**b).clone(), Expr::div(Expr::int(1), (**a).clone()))
                }
                _ => return None,
            }
        }
        _ => return None,
    };

    let l_num = limit_at_depth(&num, var, point, dir, depth + 1)?;
    let l_den = limit_at_depth(&den, var, point, dir, depth + 1)?;
    let is_indet =
        (l_num.is_zero() && l_den.is_zero()) || (is_inf_like(&l_num) && is_inf_like(&l_den));
    if !is_indet {
        return None;
    }
    let dnum = differentiate(&num, var);
    let dden = differentiate(&den, var);
    if dden.is_zero() {
        return None;
    }
    limit_at_depth(&Expr::div(dnum, dden), var, point, dir, depth + 1)
}
