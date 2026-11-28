//! Most Rapidly Varying (MRV) subexpression enumeration and Gruntz algorithm (Phase 27 / Phase 49).
//!
//! Implements Dominik Gruntz (1996) MRV set computation, comparability classes,
//! and asymptotic limit reduction for `exp`/`ln` towers as `var -> +inf`.

use crate::expr::Expr;

/// Exponential/logarithmic subexpressions of `expr` that depend on `var`, ordered
/// by decreasing size (most rapidly varying first).
pub fn mrv_set(expr: &Expr, var: &str) -> Vec<Expr> {
    let mut out: Vec<Expr> = Vec::new();
    collect(expr, var, &mut out);
    out.sort_by_key(|e| std::cmp::Reverse(e.node_count()));
    out.dedup();
    out
}

/// Comparability relation between two positive expressions `f, g -> +inf` (or `0+`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MrvOrder {
    /// `f` varies strictly slower than `g` (`lim ln|f|/ln|g| = 0`).
    Less,
    /// `f` and `g` are in the same comparability class (`lim ln|f|/ln|g| in R \ {0}`).
    Equal,
    /// `f` varies strictly faster than `g` (`lim ln|f|/ln|g| = ±inf`).
    Greater,
}

/// Compare the asymptotic growth rates of `f` and `g` as `var -> +inf`.
pub fn cmp_mrv(f: &Expr, g: &Expr, var: &str, depth: usize) -> Option<MrvOrder> {
    if f == g {
        return Some(MrvOrder::Equal);
    }
    if depth > 6 {
        return None;
    }
    let lf = log_asymptotic(f);
    let lg = log_asymptotic(g);
    let ratio = Expr::div(lf, lg);
    let l = super::limit_at_depth(
        &ratio,
        var,
        &Expr::var("inf"),
        super::LimitDir::Both,
        depth + 1,
    )?;
    if l.is_zero() {
        Some(MrvOrder::Less)
    } else if matches!(&l, Expr::Var(n) if n == "inf" || n == "minf") {
        Some(MrvOrder::Greater)
    } else if matches!(&l, Expr::Var(n) if n == "und" || n == "ind") {
        None
    } else {
        Some(MrvOrder::Equal)
    }
}

fn log_asymptotic(e: &Expr) -> Expr {
    match e {
        Expr::Exp(u) => (**u).clone(),
        other => Expr::ln(other.clone()),
    }
}

fn mrv_max(a: Vec<Expr>, b: Vec<Expr>, var: &str, depth: usize) -> Vec<Expr> {
    if a.is_empty() {
        return b;
    }
    if b.is_empty() {
        return a;
    }
    match cmp_mrv(&a[0], &b[0], var, depth) {
        Some(MrvOrder::Greater) => a,
        Some(MrvOrder::Less) => b,
        Some(MrvOrder::Equal) | None => {
            let mut merged = a;
            for item in b {
                if !merged.contains(&item) {
                    merged.push(item);
                }
            }
            merged
        }
    }
}

/// Compute the Gruntz Most Rapidly Varying (MRV) set of `expr` as `var -> +inf`.
pub fn gruntz_mrv(expr: &Expr, var: &str, depth: usize) -> Vec<Expr> {
    if !expr.variables().iter().any(|v| v == var) {
        return Vec::new();
    }
    match expr {
        Expr::Var(v) if v == var => vec![expr.clone()],
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) => mrv_max(
            gruntz_mrv(a, var, depth),
            gruntz_mrv(b, var, depth),
            var,
            depth,
        ),
        Expr::Pow(b, e) => {
            if !e.variables().iter().any(|v| v == var) {
                gruntz_mrv(b, var, depth)
            } else {
                let rewritten = Expr::exp(Expr::mul((**e).clone(), Expr::ln((**b).clone())));
                gruntz_mrv(&rewritten, var, depth)
            }
        }
        Expr::Exp(u) => {
            let inner = gruntz_mrv(u, var, depth);
            let lim_u = if depth < 6 {
                super::limit_at_depth(u, var, &Expr::var("inf"), super::LimitDir::Both, depth + 1)
            } else {
                None
            };
            if matches!(lim_u, Some(Expr::Var(ref n)) if n == "inf" || n == "minf") {
                mrv_max(vec![expr.clone()], inner, var, depth)
            } else {
                inner
            }
        }
        Expr::Neg(a)
        | Expr::Sin(a)
        | Expr::Cos(a)
        | Expr::Tan(a)
        | Expr::Ln(a)
        | Expr::Sqrt(a)
        | Expr::Sinh(a)
        | Expr::Cosh(a)
        | Expr::Tanh(a)
        | Expr::Asin(a)
        | Expr::Acos(a)
        | Expr::Atan(a)
        | Expr::Abs(a) => gruntz_mrv(a, var, depth),
        _ => Vec::new(),
    }
}

/// Gruntz MRV limit for `expr` as `var -> +inf` by rewriting the MRV class in terms of `w -> 0+`.
pub fn gruntz_limit_inf(expr: &Expr, var: &str, depth: usize) -> Option<Expr> {
    if depth > 6 {
        return None;
    }
    let omega_set = gruntz_mrv(expr, var, depth);
    if omega_set.is_empty() {
        return None;
    }
    // If MRV is just {var}, substitute var = 1/w -> w = 0+
    if omega_set.len() == 1 && omega_set[0] == Expr::var(var) {
        return None; // Already handled by 1/t series expansion
    }
    // Choose a canonical omega = exp(u) in the MRV set such that omega -> 0+ (invert if -> +inf)
    let mut omega_opt = None;
    for item in &omega_set {
        if let Expr::Exp(u) = item {
            let lu =
                super::limit_at_depth(u, var, &Expr::var("inf"), super::LimitDir::Both, depth + 1);
            if matches!(lu, Some(Expr::Var(ref n)) if n == "minf") {
                omega_opt = Some((item.clone(), (**u).clone()));
                break;
            } else if matches!(lu, Some(Expr::Var(ref n)) if n == "inf") {
                omega_opt = Some((
                    Expr::exp(Expr::neg((**u).clone())),
                    Expr::neg((**u).clone()),
                ));
                break;
            }
        }
    }
    let (_omega_expr, log_omega) = omega_opt?;
    let w_name = super::fresh_var(expr, "__gruntz_w");
    let w_var = Expr::var(&w_name);

    // Rewrite every exp(v) in omega_set as w^c * exp(v - c * log_omega) where c = lim (v / log_omega)
    let mut rewritten = expr.clone();
    for item in &omega_set {
        if let Expr::Exp(v) = item {
            let ratio = Expr::div((**v).clone(), log_omega.clone());
            let c = super::limit_at_depth(
                &ratio,
                var,
                &Expr::var("inf"),
                super::LimitDir::Both,
                depth + 1,
            )?;
            if matches!(c, Expr::Var(ref n) if n == "inf" || n == "minf" || n == "und" || n == "ind")
            {
                return None;
            }
            let diff = super::simplify_light(Expr::sub(
                (**v).clone(),
                Expr::mul(c.clone(), log_omega.clone()),
            ));
            let replacement = if diff.is_zero() {
                Expr::pow(w_var.clone(), c)
            } else {
                Expr::mul(Expr::pow(w_var.clone(), c), Expr::exp(diff))
            };
            rewritten = replace_subexpr(&rewritten, item, &replacement);
        }
    }
    if rewritten == *expr {
        return None;
    }
    let res = super::limit_at_depth(
        &rewritten,
        &w_name,
        &Expr::int(0),
        super::LimitDir::Plus,
        depth + 1,
    )?;
    if res.variables().iter().any(|v| v == var) {
        None
    } else {
        Some(res)
    }
}

fn replace_subexpr(e: &Expr, target: &Expr, replacement: &Expr) -> Expr {
    if e == target {
        return replacement.clone();
    }
    match e {
        Expr::Add(a, b) => Expr::add(
            replace_subexpr(a, target, replacement),
            replace_subexpr(b, target, replacement),
        ),
        Expr::Sub(a, b) => Expr::sub(
            replace_subexpr(a, target, replacement),
            replace_subexpr(b, target, replacement),
        ),
        Expr::Mul(a, b) => Expr::mul(
            replace_subexpr(a, target, replacement),
            replace_subexpr(b, target, replacement),
        ),
        Expr::Div(a, b) => Expr::div(
            replace_subexpr(a, target, replacement),
            replace_subexpr(b, target, replacement),
        ),
        Expr::Pow(a, b) => Expr::pow(
            replace_subexpr(a, target, replacement),
            replace_subexpr(b, target, replacement),
        ),
        Expr::Neg(a) => Expr::neg(replace_subexpr(a, target, replacement)),
        Expr::Sin(a) => Expr::sin(replace_subexpr(a, target, replacement)),
        Expr::Cos(a) => Expr::cos(replace_subexpr(a, target, replacement)),
        Expr::Tan(a) => Expr::tan(replace_subexpr(a, target, replacement)),
        Expr::Exp(a) => Expr::exp(replace_subexpr(a, target, replacement)),
        Expr::Ln(a) => Expr::ln(replace_subexpr(a, target, replacement)),
        Expr::Sqrt(a) => Expr::sqrt(replace_subexpr(a, target, replacement)),
        Expr::Sinh(a) => Expr::sinh(replace_subexpr(a, target, replacement)),
        Expr::Cosh(a) => Expr::cosh(replace_subexpr(a, target, replacement)),
        Expr::Tanh(a) => Expr::tanh(replace_subexpr(a, target, replacement)),
        Expr::Asin(a) => Expr::asin(replace_subexpr(a, target, replacement)),
        Expr::Acos(a) => Expr::acos(replace_subexpr(a, target, replacement)),
        Expr::Atan(a) => Expr::atan(replace_subexpr(a, target, replacement)),
        Expr::Abs(a) => Expr::abs(replace_subexpr(a, target, replacement)),
        _ => e.clone(),
    }
}

fn collect(e: &Expr, var: &str, out: &mut Vec<Expr>) {
    if !e.variables().iter().any(|v| v == var) {
        return;
    }
    if matches!(e, Expr::Exp(_) | Expr::Ln(_)) && !out.contains(e) {
        out.push(e.clone());
    }
    match e {
        Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
            collect(a, var, out);
            collect(b, var, out);
        }
        Expr::Neg(a)
        | Expr::Sin(a)
        | Expr::Cos(a)
        | Expr::Tan(a)
        | Expr::Exp(a)
        | Expr::Ln(a)
        | Expr::Sqrt(a)
        | Expr::Sinh(a)
        | Expr::Cosh(a)
        | Expr::Tanh(a)
        | Expr::Asin(a)
        | Expr::Acos(a)
        | Expr::Atan(a)
        | Expr::Abs(a)
        | Expr::Factorial(a) => collect(a, var, out),
        Expr::Atan2(y, x) => {
            collect(y, var, out);
            collect(x, var, out);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_exp_and_ln() {
        let x = Expr::var("x");
        let e = Expr::add(Expr::exp(x.clone()), Expr::ln(x));
        let set = mrv_set(&e, "x");
        assert!(set.iter().any(|s| matches!(s, Expr::Exp(_))));
        assert!(set.iter().any(|s| matches!(s, Expr::Ln(_))));
    }

    #[test]
    fn empty_when_independent() {
        let e = Expr::add(Expr::var("y"), Expr::exp(Expr::var("y")));
        assert!(mrv_set(&e, "x").is_empty());
    }
}
