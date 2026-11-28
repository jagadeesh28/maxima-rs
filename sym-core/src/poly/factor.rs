//! Univariate polynomial factorization over Q[x]:
//! - Yun's square-free factorization
//! - Rational-root extraction (with overflow-safe u128 divisors)
//! - Cantor-Zassenhaus over F_p[x] + linear Hensel lifting to Z[x] + factor recombination.

use super::gcd::{monic, poly_derivative, poly_div_exact, poly_gcd};
use super::{Monomial, Poly, Term, VarContext};
use crate::expr::Expr;
use crate::rational::Rat;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// Factor an expression over Q when it is a univariate polynomial.
pub fn factor(expr: &Expr) -> Expr {
    match Poly::from_expr(expr) {
        Ok(p) => factor_poly(&p),
        Err(_) => expr.clone(),
    }
}

fn factor_poly(p: &Poly) -> Expr {
    if p.degree() == 0 || p.nvars() != 1 {
        return p.to_expr();
    }
    let (lead, factors) = factor_poly_list(p);
    let mut parts: Vec<Expr> = Vec::new();
    if !Rat::is_one(&lead) {
        parts.push(Expr::constant(lead));
    }
    for (fac, mult) in factors {
        let fe = fac.to_expr();
        if mult == 1 {
            parts.push(fe);
        } else {
            parts.push(Expr::pow(fe, Expr::int(i64::from(mult))));
        }
    }
    multiply_exprs(parts)
}

/// Factor a univariate polynomial `p` over Q into `(leading_coeff, Vec<(monic_irreducible, mult)>)`.
pub(crate) fn factor_poly_list(p: &Poly) -> (BigRational, Vec<(Poly, u32)>) {
    if p.is_zero() {
        return (BigRational::zero(), Vec::new());
    }
    let lead = p
        .leading_term()
        .map(|t| t.coeff.clone())
        .unwrap_or_else(BigRational::one);
    if p.degree() == 0 || p.nvars() != 1 {
        return (lead, Vec::new());
    }

    let mp = monic(p);
    let sqf = yun_square_free(&mp);
    let mut out: Vec<(Poly, u32)> = Vec::new();

    for (sf, mult) in sqf {
        if sf.degree() == 0 {
            continue;
        }
        let mut rem = sf;
        // 1. Extract all rational roots first
        if let Some(roots) = distinct_rational_roots(&rem) {
            for r in roots {
                let lin = linear_poly(&r, &rem.ctx);
                if let Some(q) = poly_div_exact(&rem, &lin) {
                    out.push((lin, mult));
                    rem = q;
                }
            }
        }
        // 2. Factor remaining square-free factor with no rational roots via CZ + Hensel
        if rem.degree() >= 4 {
            let irr_factors = factor_square_free_cz(&rem);
            for f in irr_factors {
                out.push((monic(&f), mult));
            }
        } else if rem.degree() > 0 {
            out.push((monic(&rem), mult));
        }
    }

    // Merge identical monic factors if any
    let mut merged: Vec<(Poly, u32)> = Vec::new();
    for (f, m) in out {
        if let Some(pos) = merged.iter().position(|(ef, _)| ef == &f) {
            merged[pos].1 += m;
        } else {
            merged.push((f, m));
        }
    }
    (lead, merged)
}

/// Yun's square-free factorization of a monic univariate polynomial over Q[x].
pub(crate) fn yun_square_free(p: &Poly) -> Vec<(Poly, u32)> {
    if p.degree() <= 1 {
        return if p.degree() == 1 {
            vec![(p.clone(), 1)]
        } else {
            Vec::new()
        };
    }
    let dp = poly_derivative(p);
    let mut g = poly_gcd(p, &dp);
    let mut w = poly_div_exact(p, &g).unwrap_or_else(|| p.clone());
    let mut y = poly_div_exact(&dp, &g).unwrap_or(dp);
    let mut z = y.sub(&poly_derivative(&w));

    let mut result = Vec::new();
    let mut i = 1u32;
    while !z.is_zero() {
        g = poly_gcd(&w, &z);
        if g.degree() > 0 {
            result.push((monic(&g), i));
        }
        w = poly_div_exact(&w, &g).unwrap_or(w);
        y = poly_div_exact(&z, &g).unwrap_or(z);
        z = y.sub(&poly_derivative(&w));
        i += 1;
        if i > 256 {
            break;
        }
    }
    if w.degree() > 0 {
        result.push((monic(&w), i));
    }
    result
}

fn multiply_exprs(parts: Vec<Expr>) -> Expr {
    let mut it = parts.into_iter();
    match it.next() {
        None => Expr::int(1),
        Some(first) => it.fold(first, Expr::mul),
    }
}

/// Integer coefficients (descending degree) of a univariate polynomial, if exact.
pub(crate) fn univariate_i64_coeffs(p: &Poly) -> Option<Vec<i64>> {
    if p.nvars() != 1 {
        return None;
    }
    let deg = p.degree() as usize;
    let mut coeffs = vec![0i64; deg + 1];
    for t in p.terms() {
        if !Rat::is_integer(&t.coeff) {
            return None;
        }
        let v = t.coeff.numer().to_i64()?;
        let e = t.mono.exponent(0) as usize;
        coeffs[deg - e] = coeffs[deg - e].checked_add(v)?;
    }
    Some(coeffs)
}

/// Convert a univariate polynomial over Q into a primitive integer polynomial (ascending coeffs `[c0, c1, ..., cd]`).
fn to_primitive_bigint_asc(p: &Poly) -> Vec<BigInt> {
    let deg = p.degree() as usize;
    // Clear denominators
    let mut lcm_den = BigInt::one();
    for t in p.terms() {
        lcm_den = num_integer_lcm(&lcm_den, t.coeff.denom());
    }
    let mut coeffs = vec![BigInt::zero(); deg + 1];
    for t in p.terms() {
        let e = t.mono.exponent(0) as usize;
        let scaled = &t.coeff * BigRational::from_integer(lcm_den.clone());
        coeffs[e] += scaled.to_integer();
    }
    // Divide by GCD of coefficients and ensure positive leading coeff
    let mut g = BigInt::zero();
    for c in &coeffs {
        g = num_integer_gcd(&g, c);
    }
    if !g.is_zero() && g > BigInt::one() {
        for c in &mut coeffs {
            *c /= &g;
        }
    }
    if coeffs.last().is_some_and(|c| c.is_negative()) {
        for c in &mut coeffs {
            *c = -c.clone();
        }
    }
    coeffs
}

fn num_integer_gcd(a: &BigInt, b: &BigInt) -> BigInt {
    let mut x = a.abs();
    let mut y = b.abs();
    while !y.is_zero() {
        let r = &x % &y;
        x = y;
        y = r;
    }
    x
}

fn num_integer_lcm(a: &BigInt, b: &BigInt) -> BigInt {
    if a.is_zero() || b.is_zero() {
        return BigInt::zero();
    }
    (a.abs() / num_integer_gcd(a, b)) * b.abs()
}

fn poly_from_bigint_asc(coeffs: &[BigInt], ctx: &VarContext) -> Poly {
    let mut terms = Vec::new();
    for (e, c) in coeffs.iter().enumerate() {
        if !c.is_zero() {
            let mut exps = vec![0u32; ctx.len()];
            exps[0] = e as u32;
            terms.push(Term {
                coeff: BigRational::from_integer(c.clone()),
                mono: Monomial::from_exps(exps),
            });
        }
    }
    Poly::from_terms(terms, ctx.clone())
}

fn divisors_u128(n: i64) -> Vec<i64> {
    let m = n.unsigned_abs() as u128;
    if m == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut d = 1u128;
    while d <= m / d {
        if m.is_multiple_of(d) {
            if let Ok(di) = i64::try_from(d) {
                out.push(di);
            }
            let q = m / d;
            if q != d
                && let Ok(qi) = i64::try_from(q)
            {
                out.push(qi);
            }
        }
        d += 1;
    }
    out.sort_unstable();
    out
}

/// Candidate rational roots `p/q` of an integer polynomial (descending coeffs).
fn rational_root_candidates(coeffs: &[i64]) -> Vec<BigRational> {
    let deg = coeffs.len() - 1;
    let constant = coeffs[deg];
    if constant == 0 {
        return vec![BigRational::zero()];
    }
    let leading = coeffs[0];
    if leading == 0 {
        return Vec::new();
    }
    let mut out: Vec<BigRational> = Vec::new();
    for pn in divisors_u128(constant) {
        for qn in divisors_u128(leading) {
            if qn == 0 {
                continue;
            }
            let r = BigRational::new(BigInt::from(pn), BigInt::from(qn));
            out.push(r.clone());
            out.push(-r);
        }
    }
    out.sort();
    out.dedup();
    out
}

fn find_root(p: &Poly, coeffs: &[i64]) -> Option<BigRational> {
    rational_root_candidates(coeffs)
        .into_iter()
        .find(|r| eval_poly(p, r).is_zero())
}

/// Evaluate a univariate polynomial at a rational point.
pub(crate) fn eval_poly(p: &Poly, r: &BigRational) -> BigRational {
    let mut acc = BigRational::zero();
    for t in p.terms() {
        let e = t.mono.exponent(0);
        let mut pw = BigRational::one();
        for _ in 0..e {
            pw = &pw * r;
        }
        acc = &acc + &(&t.coeff * &pw);
    }
    acc
}

fn linear_poly(r: &BigRational, ctx: &VarContext) -> Poly {
    let mut exps = vec![0u32; ctx.len()];
    exps[0] = 1;
    let x = Poly::monomial(BigRational::one(), Monomial::from_exps(exps), ctx.clone());
    let c = Poly::constant(-r.clone(), ctx.clone());
    x.add(&c)
}

/// All rational roots of a univariate polynomial (with multiplicity).
pub(crate) fn distinct_rational_roots(p: &Poly) -> Option<Vec<BigRational>> {
    let mut cur = p.clone();
    let mut roots = Vec::new();
    loop {
        if cur.degree() == 0 {
            break;
        }
        let prim = to_primitive_bigint_asc(&cur);
        let prim_poly = poly_from_bigint_asc(&prim, &cur.ctx);
        let coeffs = univariate_i64_coeffs(&prim_poly)?;
        let Some(root) = find_root(&prim_poly, &coeffs) else {
            break;
        };
        let lin = linear_poly(&root, &cur.ctx);
        let Some(q) = poly_div_exact(&cur, &lin) else {
            break;
        };
        roots.push(root);
        cur = q;
    }
    Some(roots)
}

// ── Cantor-Zassenhaus over F_p[x] + Linear Hensel Lifting ───────────────

type FpPoly = Vec<u64>; // ascending coefficients mod p

fn fp_trim(a: &mut FpPoly) {
    while a.last().is_some_and(|&c| c == 0) {
        a.pop();
    }
}

fn fp_add(a: &[u64], b: &[u64], p: u64) -> FpPoly {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        out.push((av + bv) % p);
    }
    fp_trim(&mut out);
    out
}

fn fp_sub(a: &[u64], b: &[u64], p: u64) -> FpPoly {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let av = a.get(i).copied().unwrap_or(0);
        let bv = b.get(i).copied().unwrap_or(0);
        out.push((av + p - (bv % p)) % p);
    }
    fp_trim(&mut out);
    out
}

fn fp_mul(a: &[u64], b: &[u64], p: u64) -> FpPoly {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0u64; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        for (j, &bj) in b.iter().enumerate() {
            out[i + j] = ((out[i + j] as u128 + (ai as u128) * (bj as u128)) % (p as u128)) as u64;
        }
    }
    fp_trim(&mut out);
    out
}

fn fp_pow_mod_num(mut base: u64, mut exp: u64, p: u64) -> u64 {
    let mut res = 1u64;
    base %= p;
    while exp > 0 {
        if exp & 1 == 1 {
            res = ((res as u128 * base as u128) % (p as u128)) as u64;
        }
        base = ((base as u128 * base as u128) % (p as u128)) as u64;
        exp >>= 1;
    }
    res
}

fn fp_inv(a: u64, p: u64) -> u64 {
    fp_pow_mod_num(a, p - 2, p)
}

fn fp_monic(a: &[u64], p: u64) -> FpPoly {
    let mut out = a.to_vec();
    fp_trim(&mut out);
    if let Some(&lc) = out.last()
        && lc != 1
    {
        let inv = fp_inv(lc, p);
        for c in &mut out {
            *c = ((*c as u128 * inv as u128) % p as u128) as u64;
        }
    }
    out
}

fn fp_divrem(a: &[u64], b: &[u64], p: u64) -> (FpPoly, FpPoly) {
    let mut r = a.to_vec();
    fp_trim(&mut r);
    let mut b_norm = b.to_vec();
    fp_trim(&mut b_norm);
    if b_norm.is_empty() {
        return (Vec::new(), r);
    }
    let deg_b = b_norm.len() - 1;
    let inv_lc = fp_inv(b_norm[deg_b], p);
    let mut q = vec![0u64; r.len().saturating_sub(deg_b)];

    while r.len() > deg_b {
        let deg_r = r.len() - 1;
        let shift = deg_r - deg_b;
        let coeff = ((r[deg_r] as u128 * inv_lc as u128) % p as u128) as u64;
        if shift < q.len() {
            q[shift] = coeff;
        }
        for (i, &bi) in b_norm.iter().enumerate() {
            let sub = ((coeff as u128 * bi as u128) % p as u128) as u64;
            r[i + shift] = (r[i + shift] + p - sub) % p;
        }
        fp_trim(&mut r);
    }
    fp_trim(&mut q);
    (q, r)
}

fn fp_gcd(a: &[u64], b: &[u64], p: u64) -> FpPoly {
    let mut x = a.to_vec();
    let mut y = b.to_vec();
    fp_trim(&mut x);
    fp_trim(&mut y);
    while !y.is_empty() {
        let (_, r) = fp_divrem(&x, &y, p);
        x = y;
        y = r;
    }
    fp_monic(&x, p)
}

fn fp_deriv(a: &[u64], p: u64) -> FpPoly {
    if a.len() <= 1 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(a.len() - 1);
    for (i, &c) in a.iter().enumerate().skip(1) {
        out.push(((c as u128 * i as u128) % p as u128) as u64);
    }
    fp_trim(&mut out);
    out
}

fn fp_powmod(base: &[u64], mut exp: u128, modulus: &[u64], p: u64) -> FpPoly {
    let mut res = vec![1u64];
    let mut cur = fp_divrem(base, modulus, p).1;
    while exp > 0 {
        if exp & 1 == 1 {
            let prod = fp_mul(&res, &cur, p);
            res = fp_divrem(&prod, modulus, p).1;
        }
        cur = fp_divrem(&fp_mul(&cur, &cur, p), modulus, p).1;
        exp >>= 1;
    }
    res
}

fn fp_distinct_degree_factor(f: &[u64], p: u64) -> Vec<(FpPoly, usize)> {
    let mut out = Vec::new();
    let mut h = vec![0u64, 1u64]; // x
    let mut cur = fp_monic(f, p);
    let mut d = 1usize;
    while cur.len() > 1 && 2 * d < cur.len() {
        h = fp_powmod(&h, u128::from(p), &cur, p);
        let h_minus_x = fp_sub(&h, &[0, 1], p);
        let g = fp_gcd(&cur, &h_minus_x, p);
        if g.len() > 1 {
            out.push((g.clone(), d));
            cur = fp_divrem(&cur, &g, p).0;
            h = fp_divrem(&h, &cur, p).1;
        }
        d += 1;
    }
    if cur.len() > 1 {
        let deg = cur.len() - 1;
        out.push((cur, deg));
    }
    out
}

fn fp_equal_degree_split(f: &[u64], d: usize, p: u64, out: &mut Vec<FpPoly>) {
    let deg = f.len() - 1;
    if deg == d {
        out.push(fp_monic(f, p));
        return;
    }
    let exp = (u128::from(p).pow(d as u32) - 1) / 2;
    let mut seed = 1u64;
    for _ in 0..64 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let a: FpPoly = (0..deg)
            .map(|k| (seed.wrapping_add(k as u64 * 17)) % p)
            .collect();
        let g1 = fp_gcd(&a, f, p);
        if g1.len() > 1 && g1.len() - 1 < deg {
            let q = fp_divrem(f, &g1, p).0;
            fp_equal_degree_split(&g1, d, p, out);
            fp_equal_degree_split(&q, d, p, out);
            return;
        }
        let pw = fp_powmod(&a, exp, f, p);
        let pw_minus_1 = fp_sub(&pw, &[1], p);
        let g = fp_gcd(&pw_minus_1, f, p);
        if g.len() > 1 && g.len() - 1 < deg {
            let q = fp_divrem(f, &g, p).0;
            fp_equal_degree_split(&g, d, p, out);
            fp_equal_degree_split(&q, d, p, out);
            return;
        }
    }
    out.push(fp_monic(f, p));
}

fn fp_factor(f: &[u64], p: u64) -> Vec<FpPoly> {
    let mut out = Vec::new();
    for (g, d) in fp_distinct_degree_factor(f, p) {
        fp_equal_degree_split(&g, d, p, &mut out);
    }
    out
}

fn bigint_mod(a: &BigInt, m: &BigInt) -> BigInt {
    let r = a % m;
    if r.is_negative() { r + m } else { r }
}

fn bigint_sym_mod(a: &BigInt, m: &BigInt) -> BigInt {
    let r = bigint_mod(a, m);
    let half = m / 2;
    if r > half { r - m } else { r }
}

fn bigint_poly_mul(a: &[BigInt], b: &[BigInt]) -> Vec<BigInt> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    let mut out = vec![BigInt::zero(); a.len() + b.len() - 1];
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            out[i + j] += ai * bj;
        }
    }
    out
}

fn bigint_poly_mod(a: &[BigInt], m: &BigInt) -> Vec<BigInt> {
    let mut out: Vec<BigInt> = a.iter().map(|c| bigint_mod(c, m)).collect();
    while out.last().is_some_and(|c| c.is_zero()) {
        out.pop();
    }
    out
}

fn fp_ext_gcd(a: &[u64], b: &[u64], p: u64) -> (FpPoly, FpPoly, FpPoly) {
    let mut old_r = a.to_vec();
    let mut r = b.to_vec();
    let mut old_s = vec![1u64];
    let mut s = Vec::new();
    let mut old_t = Vec::new();
    let mut t = vec![1u64];
    fp_trim(&mut old_r);
    fp_trim(&mut r);
    while !r.is_empty() {
        let (q, rem) = fp_divrem(&old_r, &r, p);
        old_r = r;
        r = rem;
        let ns = fp_sub(&old_s, &fp_mul(&q, &s, p), p);
        old_s = s;
        s = ns;
        let nt = fp_sub(&old_t, &fp_mul(&q, &t, p), p);
        old_t = t;
        t = nt;
    }
    if let Some(&lc) = old_r.last()
        && lc != 1
    {
        let inv = fp_inv(lc, p);
        for c in &mut old_r {
            *c = ((*c as u128 * inv as u128) % p as u128) as u64;
        }
        for c in &mut old_s {
            *c = ((*c as u128 * inv as u128) % p as u128) as u64;
        }
        for c in &mut old_t {
            *c = ((*c as u128 * inv as u128) % p as u128) as u64;
        }
    }
    (old_r, old_s, old_t)
}

/// Linear Hensel lift of `f ≡ g * h (mod p)` up to `mod p^k`.
fn hensel_lift_pair(
    f: &[BigInt],
    g0: &[u64],
    h0: &[u64],
    p: u64,
    bound: &BigInt,
) -> (Vec<BigInt>, Vec<BigInt>) {
    let (_, s0, t0) = fp_ext_gcd(g0, h0, p);
    let mut g: Vec<BigInt> = g0.iter().map(|&c| BigInt::from(c)).collect();
    let mut h: Vec<BigInt> = h0.iter().map(|&c| BigInt::from(c)).collect();
    let p_big = BigInt::from(p);
    let mut modulus = p_big.clone();

    while modulus <= *bound {
        let gh = bigint_poly_mul(&g, &h);
        let mut e_coeffs = Vec::with_capacity(f.len().max(gh.len()));
        for i in 0..f.len().max(gh.len()) {
            let fv = f.get(i).cloned().unwrap_or_default();
            let ghv = gh.get(i).cloned().unwrap_or_default();
            e_coeffs.push((fv - ghv) / &modulus);
        }
        let e_mod_p: FpPoly = e_coeffs
            .iter()
            .map(|c| bigint_mod(c, &p_big).to_u64().unwrap_or(0))
            .collect();
        let g_mod_p: FpPoly = g0.to_vec();
        let h_mod_p: FpPoly = h0.to_vec();

        // s*g + t*h = 1 => e = (s*e)*g + (t*e)*h => dg = t*e mod g, dh = s*e mod h
        let te = fp_mul(&t0, &e_mod_p, p);
        let se = fp_mul(&s0, &e_mod_p, p);
        let (q_te, dg) = fp_divrem(&te, &g_mod_p, p);
        let dh_raw = fp_add(&se, &fp_mul(&q_te, &h_mod_p, p), p);
        let (_, dh) = fp_divrem(&dh_raw, &h_mod_p, p);

        let next_mod = &modulus * &p_big;
        for (i, &c) in dg.iter().enumerate() {
            if i >= g.len() {
                g.resize(i + 1, BigInt::zero());
            }
            g[i] += BigInt::from(c) * &modulus;
        }
        for (i, &c) in dh.iter().enumerate() {
            if i >= h.len() {
                h.resize(i + 1, BigInt::zero());
            }
            h[i] += BigInt::from(c) * &modulus;
        }
        g = bigint_poly_mod(&g, &next_mod);
        h = bigint_poly_mod(&h, &next_mod);
        modulus = next_mod;
    }
    (g, h)
}

fn factor_square_free_cz(p: &Poly) -> Vec<Poly> {
    let f_asc = to_primitive_bigint_asc(p);
    let deg = f_asc.len().saturating_sub(1);
    if deg < 2 {
        return vec![p.clone()];
    }
    let lc = f_asc[deg].clone();
    let max_coeff = f_asc
        .iter()
        .map(|c| c.abs())
        .max()
        .unwrap_or_else(BigInt::one);
    let bound = BigInt::from(2) * &lc * &max_coeff * BigInt::from(2).pow(deg as u32);

    // Choose a small odd prime p_prime where lc != 0 mod p and f is square-free mod p
    let primes = [5u64, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47];
    let mut chosen = None;
    for &pr in &primes {
        let pr_big = BigInt::from(pr);
        if (&lc % &pr_big).is_zero() {
            continue;
        }
        let inv_lc = fp_inv(bigint_mod(&lc, &pr_big).to_u64().unwrap_or(1), pr);
        let f_mod: FpPoly = f_asc
            .iter()
            .map(|c| {
                let cm = bigint_mod(c, &pr_big).to_u64().unwrap_or(0);
                ((cm as u128 * inv_lc as u128) % pr as u128) as u64
            })
            .collect();
        let df = fp_deriv(&f_mod, pr);
        if fp_gcd(&f_mod, &df, pr).len() == 1 {
            chosen = Some((pr, f_mod));
            break;
        }
    }

    let Some((pr, f_mod)) = chosen else {
        return vec![p.clone()];
    };

    let fp_factors = fp_factor(&f_mod, pr);
    if fp_factors.len() <= 1 {
        return vec![p.clone()];
    }

    // Lift and test subsets of modular factors against f_asc
    let mut rem_poly = p.clone();
    let mut rem_asc = f_asc;
    let mut out = Vec::new();
    let mut remaining_fp = fp_factors;

    // Normalize monic f_monic_asc mod pr^k
    let mut k_size = 1usize;
    while 2 * k_size <= remaining_fp.len() {
        let mut found_split = false;
        let n = remaining_fp.len();
        let mut idxs: Vec<usize> = (0..k_size).collect();
        'subsets: loop {
            let mut g_fp = vec![1u64];
            for &idx in &idxs {
                g_fp = fp_mul(&g_fp, &remaining_fp[idx], pr);
            }
            let mut h_fp = vec![1u64];
            for (idx, fac) in remaining_fp.iter().enumerate() {
                if !idxs.contains(&idx) {
                    h_fp = fp_mul(&h_fp, fac, pr);
                }
            }
            let cur_lc = rem_asc.last().cloned().unwrap_or_else(BigInt::one);
            let f_monic_test = if cur_lc.is_one() {
                rem_asc.clone()
            } else {
                // Scale by lc^{deg-1}: f(x) -> lc * f(x)
                rem_asc.iter().map(|c| c * &cur_lc).collect()
            };
            let (g_lift, _) = hensel_lift_pair(&f_monic_test, &g_fp, &h_fp, pr, &bound);
            let pr_big = BigInt::from(pr);
            let mut full_mod = pr_big.clone();
            while full_mod <= bound {
                full_mod *= &pr_big;
            }
            let g_cand_coeffs: Vec<BigInt> = g_lift
                .iter()
                .map(|c| bigint_sym_mod(&(c * &cur_lc), &full_mod))
                .collect();
            let g_poly = poly_from_bigint_asc(&g_cand_coeffs, &rem_poly.ctx);
            let g_prim = poly_from_bigint_asc(&to_primitive_bigint_asc(&g_poly), &rem_poly.ctx);

            if g_prim.degree() > 0
                && g_prim.degree() < rem_poly.degree()
                && let Some(q) = poly_div_exact(&rem_poly, &g_prim)
            {
                out.push(monic(&g_prim));
                rem_poly = q;
                rem_asc = to_primitive_bigint_asc(&rem_poly);
                let mut next_fp = Vec::new();
                for (idx, fac) in remaining_fp.into_iter().enumerate() {
                    if !idxs.contains(&idx) {
                        next_fp.push(fac);
                    }
                }
                remaining_fp = next_fp;
                found_split = true;
                break 'subsets;
            }

            // Advance combination
            let mut pos = k_size;
            let mut moved = false;
            while pos > 0 {
                pos -= 1;
                if idxs[pos] < n - k_size + pos {
                    idxs[pos] += 1;
                    for j in (pos + 1)..k_size {
                        idxs[j] = idxs[j - 1] + 1;
                    }
                    moved = true;
                    break;
                }
            }
            if !moved {
                break;
            }
        }
        if !found_split {
            k_size += 1;
        }
    }

    if rem_poly.degree() > 0 {
        out.push(monic(&rem_poly));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poly::expand::expand;
    use std::collections::HashMap;

    fn eval_at(e: &Expr, x: f64) -> f64 {
        let mut env = HashMap::new();
        env.insert("x".to_string(), x);
        e.eval(&env).expect("eval")
    }

    #[test]
    fn factor_difference_of_squares() {
        let x = Expr::var("x");
        let e = Expr::sub(Expr::pow(x, Expr::int(2)), Expr::int(1));
        let f = factor(&e);
        let back = expand(&f);
        for xv in [-3.0_f64, -0.5, 0.0, 2.0] {
            assert!((eval_at(&back, xv) - eval_at(&e, xv)).abs() < 1e-9);
        }
    }

    #[test]
    fn factor_cubic_with_zero_root() {
        let x = Expr::var("x");
        let e = Expr::sub(Expr::pow(x.clone(), Expr::int(3)), x);
        let f = factor(&e);
        let back = expand(&f);
        for xv in [-2.0_f64, 0.0, 1.5, 3.0] {
            assert!((eval_at(&back, xv) - eval_at(&e, xv)).abs() < 1e-9);
        }
    }

    #[test]
    fn factor_quartic_irreducible_quadratics() {
        // x^4 + 4 = (x^2 - 2x + 2)(x^2 + 2x + 2)
        let x = Expr::var("x");
        let e = Expr::add(Expr::pow(x, Expr::int(4)), Expr::int(4));
        let f = factor(&e);
        assert!(
            matches!(f, Expr::Mul(_, _)),
            "expected product of quadratics, got {f}"
        );
        let back = expand(&f);
        for xv in [-2.0_f64, -0.5, 0.0, 1.5, 3.0] {
            assert!((eval_at(&back, xv) - eval_at(&e, xv)).abs() < 1e-9);
        }
    }
}
