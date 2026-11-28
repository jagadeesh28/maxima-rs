//! Polynomial root finding via rational-root factoring, linear/quadratic/cubic/biquadratic,
//! parametric equations, and `linsolve` (Phase 29 / Phase 50).

use crate::expr::Expr;
use crate::poly::factor::distinct_rational_roots;
use crate::poly::gcd::poly_div_exact;
use crate::poly::rat::ratsimp;
use crate::poly::{Monomial, Poly, VarContext, expand};
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::{One, Zero};

/// Classification of an algebraic solve result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveStatus {
    /// Finite list of exact roots (complete).
    Finite,
    /// Identity `0 = 0`: every value is a solution.
    All,
    /// Contradiction `c = 0` (`c != 0`): no solutions exist.
    Empty,
    /// Partial factorization: some roots could not be expressed in closed form.
    Partial,
}

/// Solve a univariate or parametric polynomial equation `expr = 0` for `var`.
pub(crate) fn solve_polynomial(expr: &Expr, var: &str) -> Option<(Vec<Expr>, bool)> {
    let (roots, status) = solve_polynomial_classified(expr, var)?;
    Some((
        roots,
        matches!(
            status,
            SolveStatus::Finite | SolveStatus::All | SolveStatus::Empty
        ),
    ))
}

/// Solve `expr = 0` with explicit [`SolveStatus`] (`All` for `0 = 0`, `Empty` for `1 = 0`).
pub(crate) fn solve_polynomial_classified(
    expr: &Expr,
    var: &str,
) -> Option<(Vec<Expr>, SolveStatus)> {
    let p = Poly::from_expr(expr).ok()?;
    if p.is_zero() {
        return Some((Vec::new(), SolveStatus::All));
    }
    if p.vars().iter().all(|v| v != var) {
        // Non-zero constant w.r.t. `var`
        return Some((Vec::new(), SolveStatus::Empty));
    }
    // Parametric polynomial in `var` (contains other parameters)
    if p.vars().iter().any(|v| v != var) {
        return solve_parametric(&p, var);
    }

    let mut cur = p.clone();
    let mut roots: Vec<Expr> = Vec::new();

    if let Some(rationals) = distinct_rational_roots(&p) {
        for r in rationals {
            if cur.degree() == 0 {
                break;
            }
            let lin = linear_poly(&r, &cur.ctx);
            while let Some(q) = poly_div_exact(&cur, &lin) {
                cur = q;
                roots.push(Expr::constant(r.clone()));
            }
        }
    }

    match cur.degree() {
        0 => {}
        1 => {
            let a = coeff(&cur, 1);
            let b = coeff(&cur, 0);
            if !a.is_zero() {
                roots.push(Expr::constant(-&b / &a));
                cur = Poly::constant(BigRational::one(), cur.ctx.clone());
            }
        }
        2 => {
            let q_roots = quadratic_roots(&cur);
            if !q_roots.is_empty() {
                roots.extend(q_roots);
                cur = Poly::constant(BigRational::one(), cur.ctx.clone());
            }
        }
        3 => {
            let c_roots = cubic_cardano_roots(&cur);
            if !c_roots.is_empty() {
                roots.extend(c_roots);
                cur = Poly::constant(BigRational::one(), cur.ctx.clone());
            }
        }
        4 => {
            // Biquadratic: a*x^4 + b*x^2 + c = 0
            if coeff(&cur, 3).is_zero() && coeff(&cur, 1).is_zero() {
                let a = coeff(&cur, 4);
                let b = coeff(&cur, 2);
                let c = coeff(&cur, 0);
                let u_roots = quadratic_expr_roots(
                    &Expr::constant(a),
                    &Expr::constant(b),
                    &Expr::constant(c),
                );
                for u in u_roots {
                    let sq = Expr::sqrt(u);
                    roots.push(sq.clone());
                    roots.push(Expr::neg(sq));
                }
                cur = Poly::constant(BigRational::one(), cur.ctx.clone());
            }
        }
        _ => {}
    }

    let status = if cur.degree() == 0 {
        if roots.is_empty() {
            SolveStatus::Empty
        } else {
            SolveStatus::Finite
        }
    } else {
        SolveStatus::Partial
    };
    Some((roots, status))
}

fn solve_parametric(p: &Poly, var: &str) -> Option<(Vec<Expr>, SolveStatus)> {
    let var_idx = p.ctx.index_of(var)?;
    let max_deg = p
        .terms()
        .iter()
        .map(|t| t.mono.exponent(var_idx))
        .max()
        .unwrap_or(0);
    let coeff_expr = |deg: u32| -> Expr {
        let terms: Vec<_> = p
            .terms()
            .iter()
            .filter(|t| t.mono.exponent(var_idx) == deg)
            .map(|t| {
                let mut exps = t.mono.exps().to_vec();
                exps[var_idx] = 0;
                crate::poly::Term {
                    coeff: t.coeff.clone(),
                    mono: Monomial::from_exps(exps),
                }
            })
            .collect();
        expand(&Poly::from_terms(terms, p.ctx.clone()).to_expr())
    };
    match max_deg {
        0 => Some((Vec::new(), SolveStatus::Empty)),
        1 => {
            let a = coeff_expr(1);
            let b = coeff_expr(0);
            if a.is_zero() {
                return Some((Vec::new(), SolveStatus::Empty));
            }
            let root = ratsimp(&Expr::neg(Expr::div(b, a))).unwrap_or_else(|_| Expr::int(0));
            Some((vec![root], SolveStatus::Finite))
        }
        2 => {
            let a = coeff_expr(2);
            let b = coeff_expr(1);
            let c = coeff_expr(0);
            Some((quadratic_expr_roots(&a, &b, &c), SolveStatus::Finite))
        }
        _ => None,
    }
}

fn coeff(p: &Poly, exp: u32) -> BigRational {
    for t in p.terms() {
        if t.mono.exponent(0) == exp {
            return t.coeff.clone();
        }
    }
    BigRational::zero()
}

fn linear_poly(r: &BigRational, ctx: &VarContext) -> Poly {
    let mut exps = vec![0u32; ctx.len()];
    exps[0] = 1;
    let x = Poly::monomial(BigRational::one(), Monomial::from_exps(exps), ctx.clone());
    x.add(&Poly::constant(-r.clone(), ctx.clone()))
}

fn quadratic_expr_roots(a: &Expr, b: &Expr, c: &Expr) -> Vec<Expr> {
    let disc = expand(&Expr::sub(
        Expr::pow(b.clone(), Expr::int(2)),
        Expr::mul(Expr::int(4), Expr::mul(a.clone(), c.clone())),
    ));
    let two_a = expand(&Expr::mul(Expr::int(2), a.clone()));
    let neg_b = Expr::neg(b.clone());
    if let Expr::Const(dr) = &disc
        && *dr < BigRational::zero()
    {
        #[cfg(feature = "complex")]
        {
            let imag = Expr::mul(Expr::i(), Expr::sqrt(Expr::constant(-dr.clone())));
            return vec![
                Expr::div(Expr::add(neg_b.clone(), imag.clone()), two_a.clone()),
                Expr::div(Expr::sub(neg_b, imag), two_a),
            ];
        }
        #[cfg(not(feature = "complex"))]
        {
            return Vec::new();
        }
    }
    let sqrt = Expr::sqrt(disc);
    vec![
        Expr::div(Expr::add(neg_b.clone(), sqrt.clone()), two_a.clone()),
        Expr::div(Expr::sub(neg_b, sqrt), two_a),
    ]
}

fn quadratic_roots(p: &Poly) -> Vec<Expr> {
    let a = Expr::constant(coeff(p, 2));
    let b = Expr::constant(coeff(p, 1));
    let c = Expr::constant(coeff(p, 0));
    quadratic_expr_roots(&a, &b, &c)
}

/// Cardano's formula for depressed/general cubic `a*x^3 + b*x^2 + c*x + d = 0`.
fn cubic_cardano_roots(p: &Poly) -> Vec<Expr> {
    let a = coeff(p, 3);
    if a.is_zero() {
        return Vec::new();
    }
    let b = coeff(p, 2) / &a;
    let c = coeff(p, 1) / &a;
    let d = coeff(p, 0) / &a;
    let three = Rat::from_int(3);
    let two = Rat::from_int(2);
    let twenty_seven = Rat::from_int(27);
    let four = Rat::from_int(4);

    // Depressed cubic t^3 + pp*t + qq = 0 with x = t - b/3
    let pp = &c - (&b * &b) / &three;
    let qq = (&two * &b * &b * &b) / &twenty_seven - (&b * &c) / &three + &d;
    let shift = Expr::constant(-&b / &three);

    let disc_cardano = (&qq * &qq) / &four + (&pp * &pp * &pp) / &twenty_seven;
    let half_neg_q = Expr::constant(-&qq / &two);
    let sqrt_d = Expr::sqrt(Expr::constant(disc_cardano));
    let u = Expr::pow(
        Expr::add(half_neg_q.clone(), sqrt_d.clone()),
        Expr::frac(1, 3),
    );
    let v = Expr::pow(Expr::sub(half_neg_q, sqrt_d), Expr::frac(1, 3));
    let r1 = Expr::add(Expr::add(u.clone(), v.clone()), shift.clone());

    #[cfg(feature = "complex")]
    {
        let half = Expr::frac(-1, 2);
        let sq3_2_i = Expr::mul(Expr::i(), Expr::div(Expr::sqrt(Expr::int(3)), Expr::int(2)));
        let omega = Expr::add(half.clone(), sq3_2_i.clone());
        let omega2 = Expr::sub(half, sq3_2_i);
        let r2 = Expr::add(
            Expr::add(
                Expr::mul(omega.clone(), u.clone()),
                Expr::mul(omega2.clone(), v.clone()),
            ),
            shift.clone(),
        );
        let r3 = Expr::add(Expr::add(Expr::mul(omega2, u), Expr::mul(omega, v)), shift);
        vec![r1, r2, r3]
    }
    #[cfg(not(feature = "complex"))]
    {
        vec![r1]
    }
}

/// Solve a system of linear equations `eqs[i] = 0` in variables `vars`.
///
/// Uses fraction-free Gaussian elimination over symbolic expressions (`ratsimp`).
pub fn linsolve(eqs: &[Expr], vars: &[&str]) -> Option<Vec<Expr>> {
    let m = eqs.len();
    let n = vars.len();
    if m < n || n == 0 {
        return None;
    }
    // Build augmented matrix [A | b] where eq = sum(A_ij * x_j) - b_i = 0
    let mut mat: Vec<Vec<Expr>> = Vec::with_capacity(m);
    for eq in eqs {
        let p = Poly::from_expr(eq).ok()?;
        let mut row = Vec::with_capacity(n + 1);
        for &v in vars {
            let c_terms: Vec<_> = if let Some(v_idx) = p.ctx.index_of(v) {
                p.terms()
                    .iter()
                    .filter(|t| {
                        if t.mono.exponent(v_idx) != 1 {
                            return false;
                        }
                        // Ensure linear in all vars
                        vars.iter().all(|&ov| {
                            ov == v || p.ctx.index_of(ov).is_none_or(|oi| t.mono.exponent(oi) == 0)
                        })
                    })
                    .map(|t| {
                        let mut exps = t.mono.exps().to_vec();
                        exps[v_idx] = 0;
                        crate::poly::Term {
                            coeff: t.coeff.clone(),
                            mono: Monomial::from_exps(exps),
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };
            row.push(expand(&Poly::from_terms(c_terms, p.ctx.clone()).to_expr()));
        }
        // Constant term (independent of all `vars`), negated for RHS
        let const_terms: Vec<_> = p
            .terms()
            .iter()
            .filter(|t| {
                vars.iter()
                    .all(|&v| p.ctx.index_of(v).is_none_or(|vi| t.mono.exponent(vi) == 0))
            })
            .cloned()
            .collect();
        let rhs = expand(&Expr::neg(
            Poly::from_terms(const_terms, p.ctx.clone()).to_expr(),
        ));
        row.push(rhs);
        mat.push(row);
    }

    // Gaussian elimination with ratsimp
    for col in 0..n {
        let pivot_row = (col..m).find(|&r| !mat[r][col].is_zero())?;
        mat.swap(col, pivot_row);
        let pivot = mat[col][col].clone();
        for r in (col + 1)..m {
            if mat[r][col].is_zero() {
                continue;
            }
            let factor = ratsimp(&Expr::div(mat[r][col].clone(), pivot.clone())).ok()?;
            for c in col..=n {
                let updated = Expr::sub(
                    mat[r][c].clone(),
                    Expr::mul(factor.clone(), mat[col][c].clone()),
                );
                mat[r][c] = ratsimp(&updated).ok()?;
            }
        }
    }
    // Check overdetermined consistency
    for row in mat.iter().take(m).skip(n) {
        if !row[n].is_zero() {
            return None;
        }
    }
    // Back-substitution
    let mut sol = vec![Expr::int(0); n];
    for i in (0..n).rev() {
        let mut acc = mat[i][n].clone();
        for j in (i + 1)..n {
            acc = Expr::sub(acc, Expr::mul(mat[i][j].clone(), sol[j].clone()));
        }
        sol[i] = ratsimp(&Expr::div(acc, mat[i][i].clone())).ok()?;
    }
    Some(sol)
}
