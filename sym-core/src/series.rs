//! Truncated Laurent series arithmetic (Phase 26).
//!
//! A [`Series`] represents `Σ c_k (x - center)^(valuation + k)` truncated to
//! exponents below `order`. Negative valuations model poles (Laurent series).
//! Coefficients are exact [`Expr`] values, so elementary-function expansions stay
//! exact (rationals, `π`, etc.).

use crate::expr::Expr;
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

/// A truncated Laurent (or Taylor) series about `center`.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    /// Expansion variable name.
    pub var: String,
    /// Expansion center (as an [`Expr`]).
    pub center: Expr,
    /// Lowest exponent retained (may be negative).
    pub valuation: i64,
    /// Coefficient of exponent `valuation + index`.
    pub coeffs: Vec<Expr>,
    /// Truncation order: exponents kept are `< order`.
    pub order: i64,
}

/// Fold constant-only arithmetic and algebraic zero/one identities so coefficient cancellation works.
fn fold(e: Expr) -> Expr {
    match e {
        Expr::Neg(a) => {
            let a = fold(*a);
            match a {
                Expr::Const(r) => Expr::constant(-r),
                Expr::Neg(inner) => *inner,
                other => Expr::Neg(Box::new(other)),
            }
        }
        Expr::Add(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
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
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x + y),
                _ => Expr::Add(Box::new(a), Box::new(b)),
            }
        }
        Expr::Sub(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
            if b.is_zero() {
                return a;
            }
            if a.is_zero() {
                return fold(Expr::Neg(Box::new(b)));
            }
            if a == b {
                return Expr::int(0);
            }
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x - y),
                _ => Expr::Sub(Box::new(a), Box::new(b)),
            }
        }
        Expr::Mul(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
            if a.is_zero() || b.is_zero() {
                return Expr::int(0);
            }
            if a.is_one() {
                return b;
            }
            if b.is_one() {
                return a;
            }
            if matches!(&a, Expr::Const(r) if *r == Rat::from_int(-1)) {
                return fold(Expr::Neg(Box::new(b)));
            }
            if matches!(&b, Expr::Const(r) if *r == Rat::from_int(-1)) {
                return fold(Expr::Neg(Box::new(a)));
            }
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x * y),
                _ => Expr::Mul(Box::new(a), Box::new(b)),
            }
        }
        Expr::Div(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
            if a.is_zero() && !b.is_zero() {
                return Expr::int(0);
            }
            if b.is_one() {
                return a;
            }
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) if !y.is_zero() => Expr::constant(x / y),
                _ => Expr::Div(Box::new(a), Box::new(b)),
            }
        }
        Expr::Pow(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
            if b.is_zero() && !a.is_zero() {
                return Expr::int(1);
            }
            if b.is_one() {
                return a;
            }
            if a.is_one() {
                return Expr::int(1);
            }
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) if Rat::is_integer(y) => {
                    if let Some(res) = Rat::pow_int(x, y) {
                        return Expr::constant(res);
                    }
                    Expr::Pow(Box::new(a), Box::new(b))
                }
                _ => Expr::Pow(Box::new(a), Box::new(b)),
            }
        }
        other => other,
    }
}

impl Series {
    /// Trim leading/trailing zero coefficients, adjusting the valuation.
    fn normalized(mut self) -> Series {
        self.coeffs = std::mem::take(&mut self.coeffs)
            .into_iter()
            .map(fold)
            .collect();
        while !self.coeffs.is_empty() && self.coeffs[0].is_zero() {
            self.coeffs.remove(0);
            self.valuation += 1;
        }
        while self.coeffs.last().map(|c| c.is_zero()).unwrap_or(false) {
            self.coeffs.pop();
        }
        self
    }

    /// The zero series.
    pub fn zero(var: &str, center: &Expr, order: i64) -> Series {
        Series {
            var: var.to_string(),
            center: center.clone(),
            valuation: 0,
            coeffs: Vec::new(),
            order,
        }
    }

    /// A constant series.
    pub fn constant(value: Expr, var: &str, center: &Expr, order: i64) -> Series {
        Series {
            var: var.to_string(),
            center: center.clone(),
            valuation: 0,
            coeffs: vec![value],
            order,
        }
        .normalized()
    }

    /// The coefficient of `x^exp` (zero if outside the retained range).
    pub fn coeff_at(&self, exp: i64) -> Expr {
        if exp >= self.valuation && exp < self.order {
            let idx = (exp - self.valuation) as usize;
            self.coeffs
                .get(idx)
                .cloned()
                .unwrap_or_else(|| Expr::int(0))
        } else {
            Expr::int(0)
        }
    }

    /// The constant term, or `None` if the series has a pole at the center.
    pub fn constant_term(&self) -> Option<Expr> {
        if self.valuation > 0 {
            Some(Expr::int(0))
        } else if self.valuation == 0 {
            Some(self.coeff_at(0))
        } else {
            None
        }
    }

    /// Multiply every coefficient by `k`.
    pub fn scale(&self, k: &Expr) -> Series {
        Series {
            var: self.var.clone(),
            center: self.center.clone(),
            valuation: self.valuation,
            coeffs: self
                .coeffs
                .iter()
                .map(|c| Expr::mul(c.clone(), k.clone()))
                .collect(),
            order: self.order,
        }
        .normalized()
    }

    /// Convert back into an [`Expr`].
    pub fn to_expr(&self) -> Expr {
        let base = if self.center.is_zero() {
            Expr::var(&self.var)
        } else {
            Expr::sub(Expr::var(&self.var), self.center.clone())
        };
        let mut acc = Expr::int(0);
        for (i, c) in self.coeffs.iter().enumerate() {
            if c.is_zero() {
                continue;
            }
            let e = self.valuation + i as i64;
            let xpow = if e == 0 {
                Expr::int(1)
            } else {
                Expr::pow(base.clone(), Expr::int(e))
            };
            acc = Expr::add(acc, Expr::mul(c.clone(), xpow));
        }
        acc
    }

    /// Series addition.
    #[allow(clippy::should_implement_trait)]
    pub fn add(&self, other: &Series) -> Series {
        let order = self.order.min(other.order);
        let v = self.valuation.min(other.valuation);
        let n = (order - v).max(0) as usize;
        let mut coeffs = Vec::with_capacity(n);
        for k in 0..n {
            let e = v + k as i64;
            coeffs.push(Expr::add(self.coeff_at(e), other.coeff_at(e)));
        }
        Series {
            var: self.var.clone(),
            center: self.center.clone(),
            valuation: v,
            coeffs,
            order,
        }
        .normalized()
    }

    /// Series subtraction.
    #[allow(clippy::should_implement_trait)]
    pub fn sub(&self, other: &Series) -> Series {
        self.add(&other.neg())
    }

    /// Series negation.
    #[allow(clippy::should_implement_trait)]
    pub fn neg(&self) -> Series {
        Series {
            var: self.var.clone(),
            center: self.center.clone(),
            valuation: self.valuation,
            coeffs: self.coeffs.iter().map(|c| Expr::neg(c.clone())).collect(),
            order: self.order,
        }
        .normalized()
    }

    /// Series multiplication (truncated convolution).
    ///
    /// Valid truncation order is `min(order_a + val_b, order_b + val_a)`.
    #[allow(clippy::should_implement_trait)]
    pub fn mul(&self, other: &Series) -> Series {
        if self.coeffs.is_empty() || other.coeffs.is_empty() {
            return Series::zero(&self.var, &self.center, self.order.min(other.order));
        }
        let order = (self.order + other.valuation).min(other.order + self.valuation);
        let v = self.valuation + other.valuation;
        let n = (order - v).max(0) as usize;
        let mut coeffs = vec![Expr::int(0); n];
        for i in 0..self.coeffs.len() {
            let ea = self.valuation + i as i64;
            for j in 0..other.coeffs.len() {
                let eb = other.valuation + j as i64;
                let e = ea + eb;
                if e >= v && e < order {
                    let idx = (e - v) as usize;
                    coeffs[idx] = Expr::add(
                        coeffs[idx].clone(),
                        Expr::mul(self.coeffs[i].clone(), other.coeffs[j].clone()),
                    );
                }
            }
        }
        Series {
            var: self.var.clone(),
            center: self.center.clone(),
            valuation: v,
            coeffs,
            order,
        }
        .normalized()
    }

    /// Multiplicative inverse, if the leading coefficient is non-zero.
    ///
    /// For `f = c_0 x^v + ... + O(x^order)`, `1/f = x^{-v}/c_0 + ... + O(x^{order - 2*v})`.
    pub fn inv(&self) -> Option<Series> {
        if self.coeffs.is_empty() {
            return None;
        }
        let c0 = self.coeff_at(self.valuation);
        if c0.is_zero() {
            return None;
        }
        let v = self.valuation;
        let new_v = -v;
        let new_order = self.order - 2 * v;
        let n = (new_order - new_v).max(0) as usize;
        if n == 0 {
            return Some(Series {
                var: self.var.clone(),
                center: self.center.clone(),
                valuation: new_v,
                coeffs: Vec::new(),
                order: new_order,
            });
        }
        let mut d = vec![Expr::int(0); n];
        d[0] = Expr::div(Expr::int(1), c0.clone());
        for k in 1..n {
            let mut sum = Expr::int(0);
            for j in 1..=k {
                sum = Expr::add(
                    sum,
                    Expr::mul(self.coeff_at(v + j as i64), d[k - j].clone()),
                );
            }
            d[k] = Expr::neg(Expr::mul(Expr::div(Expr::int(1), c0.clone()), sum));
        }
        Some(
            Series {
                var: self.var.clone(),
                center: self.center.clone(),
                valuation: new_v,
                coeffs: d,
                order: new_order,
            }
            .normalized(),
        )
    }

    /// Series division.
    #[allow(clippy::should_implement_trait)]
    pub fn div(&self, other: &Series) -> Option<Series> {
        Some(self.mul(&other.inv()?))
    }

    /// Integer power of a series.
    pub fn pow_int(&self, n: i64) -> Option<Series> {
        if n == 0 {
            return Some(Series::constant(
                Expr::int(1),
                &self.var,
                &self.center,
                self.order,
            ));
        }
        if n < 0 {
            return self.inv()?.pow_int(-n);
        }
        let mut result = Series::constant(Expr::int(1), &self.var, &self.center, self.order);
        let mut base = self.clone();
        let mut e = n as u64;
        while e > 0 {
            if e & 1 == 1 {
                result = result.mul(&base);
            }
            e >>= 1;
            if e > 0 {
                base = base.mul(&base);
            }
        }
        Some(result)
    }

    /// Term-by-term derivative with respect to `var`.
    ///
    /// Differentiating a series truncated at `O(x^order)` yields truncation `O(x^(order - 1))`.
    pub fn diff(&self) -> Series {
        let new_v = self.valuation - 1;
        let order = self.order - 1;
        let n = (order - new_v).max(0) as usize;
        let mut coeffs = vec![Expr::int(0); n];
        for i in 0..self.coeffs.len() {
            let e = self.valuation + i as i64;
            let de = e - 1;
            if de >= new_v && de < order {
                let idx = (de - new_v) as usize;
                coeffs[idx] = Expr::add(
                    coeffs[idx].clone(),
                    Expr::mul(self.coeffs[i].clone(), Expr::int(e)),
                );
            }
        }
        Series {
            var: self.var.clone(),
            center: self.center.clone(),
            valuation: new_v,
            coeffs,
            order,
        }
        .normalized()
    }

    /// Compose `self(g(x))`, requiring `g` to vanish at the center (`valuation ≥ 1`).
    pub fn compose(&self, g: &Series) -> Option<Series> {
        if self.valuation < 0 || g.valuation < 1 {
            return None;
        }
        let order = self.order.min(g.order);
        let mut acc = Series::zero(&self.var, &self.center, order);
        for (i, c) in self.coeffs.iter().enumerate() {
            if c.is_zero() {
                continue;
            }
            let e = self.valuation + i as i64;
            if e < 0 {
                continue;
            }
            let gp = g.pow_int(e)?;
            acc = acc.add(&gp.scale(c));
        }
        Some(acc.normalized())
    }
}

/// Expand `expr` as a truncated Laurent/Taylor series about `center`, retaining powers
/// up to and including degree `order` (truncation cutoff `order + 1`).
pub fn series_expand(expr: &Expr, var: &str, center: &Expr, order: i64) -> Option<Series> {
    expand(expr, var, center, order + 1)
}

/// Convenience wrapper: expand a series and convert it back into an [`Expr`].
///
/// Supports Puiseux (`sqrt(x)`) and logarithmic (`ln(x)`) terms at `center = 0`
/// when `series_expand` cannot represent non-integer or logarithmic valuations directly.
pub fn taylor(expr: &Expr, var: &str, center: &Expr, order: i64) -> Option<Expr> {
    if let Some(s) = series_expand(expr, var, center, order) {
        return Some(s.to_expr());
    }
    if center.is_zero() {
        match expr {
            Expr::Ln(inner) => {
                let g = series_expand(inner, var, center, order)?;
                if g.valuation > 0 {
                    let c0 = g.coeff_at(g.valuation);
                    let shifted = Series {
                        var: g.var.clone(),
                        center: g.center.clone(),
                        valuation: 0,
                        coeffs: g.coeffs.clone(),
                        order: g.order - g.valuation,
                    };
                    let ln_shifted = ln_series(shifted)?;
                    let log_term = Expr::mul(Expr::int(g.valuation), Expr::ln(Expr::var(var)));
                    let mut out = Expr::add(log_term, ln_shifted.to_expr());
                    if !c0.is_one() {
                        // ln_shifted already includes const_ln(c0)
                    }
                    out = fold(out);
                    return Some(out);
                }
            }
            Expr::Sqrt(inner) => {
                let g = series_expand(inner, var, center, order)?;
                if g.valuation % 2 != 0 {
                    let shifted = Series {
                        var: g.var.clone(),
                        center: g.center.clone(),
                        valuation: 0,
                        coeffs: g.coeffs.clone(),
                        order: g.order - g.valuation,
                    };
                    let root_shifted = pow_series_rational(shifted, 1, 2)?;
                    let k = (g.valuation - 1).div_euclid(2);
                    let puiseux = if k == 0 {
                        Expr::sqrt(Expr::var(var))
                    } else {
                        Expr::mul(
                            Expr::pow(Expr::var(var), Expr::int(k)),
                            Expr::sqrt(Expr::var(var)),
                        )
                    };
                    return Some(fold(Expr::mul(puiseux, root_shifted.to_expr())));
                }
            }
            _ => {}
        }
    }
    None
}

fn const_rat(e: &Expr) -> Option<BigRational> {
    match e {
        Expr::Const(r) => Some(r.clone()),
        _ => None,
    }
}

fn factorial_expr(k: i64) -> Expr {
    let mut acc = BigRational::one();
    for i in 2..=k {
        acc *= Rat::from_int(i);
    }
    Expr::constant(acc)
}

fn sin_kernel(var: &str, center: &Expr, order: i64) -> Series {
    let n = order.max(0) as usize;
    let mut coeffs = vec![Expr::int(0); n];
    let mut k = 0i64;
    while 2 * k + 1 < order {
        let e = (2 * k + 1) as usize;
        let sign = if k % 2 == 0 { 1 } else { -1 };
        coeffs[e] = Expr::div(Expr::int(sign), factorial_expr(2 * k + 1));
        k += 1;
    }
    Series {
        var: var.to_string(),
        center: center.clone(),
        valuation: 0,
        coeffs,
        order,
    }
    .normalized()
}

fn cos_kernel(var: &str, center: &Expr, order: i64) -> Series {
    let n = order.max(0) as usize;
    let mut coeffs = vec![Expr::int(0); n];
    let mut k = 0i64;
    while 2 * k < order {
        let e = (2 * k) as usize;
        let sign = if k % 2 == 0 { 1 } else { -1 };
        coeffs[e] = Expr::div(Expr::int(sign), factorial_expr(2 * k));
        k += 1;
    }
    Series {
        var: var.to_string(),
        center: center.clone(),
        valuation: 0,
        coeffs,
        order,
    }
    .normalized()
}

fn exp_kernel(var: &str, center: &Expr, order: i64) -> Series {
    let n = order.max(0) as usize;
    let mut coeffs = vec![Expr::int(0); n];
    for (idx, slot) in coeffs.iter_mut().enumerate() {
        *slot = Expr::div(Expr::int(1), factorial_expr(idx as i64));
    }
    Series {
        var: var.to_string(),
        center: center.clone(),
        valuation: 0,
        coeffs,
        order,
    }
    .normalized()
}

fn ln1p_kernel(var: &str, center: &Expr, order: i64) -> Series {
    let n = order.max(0) as usize;
    let mut coeffs = vec![Expr::int(0); n];
    for (idx, slot) in coeffs.iter_mut().enumerate() {
        if idx == 0 {
            continue;
        }
        let e = idx as i64;
        let sign = if e % 2 == 1 { 1 } else { -1 };
        *slot = Expr::frac(sign, e);
    }
    Series {
        var: var.to_string(),
        center: center.clone(),
        valuation: 0,
        coeffs,
        order,
    }
    .normalized()
}

/// Check if `e` is a rational multiple of `pi`, returning `Some(k)` where `e = k * pi`.
fn pi_multiple(e: &Expr) -> Option<BigRational> {
    match e {
        Expr::Pi => Some(BigRational::one()),
        Expr::Neg(a) => pi_multiple(a).map(|k| -k),
        Expr::Mul(a, b) => {
            if let Some(k) = pi_multiple(a)
                && let Some(r) = const_rat(b)
            {
                return Some(k * r);
            }
            if let Some(k) = pi_multiple(b)
                && let Some(r) = const_rat(a)
            {
                return Some(k * r);
            }
            None
        }
        Expr::Div(a, b) => {
            let k = pi_multiple(a)?;
            let r = const_rat(b)?;
            if r.is_zero() { None } else { Some(k / r) }
        }
        _ => None,
    }
}

fn const_sin(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_zero() {
        return Expr::int(0);
    }
    if let Some(k) = pi_multiple(&c) {
        // Reduce k modulo 2
        let two = Rat::from_int(2);
        let q = (&k / &two).floor();
        let rem = k - q * &two;
        if rem.is_zero() || rem == Rat::from_int(1) {
            return Expr::int(0);
        }
        if rem == Rat::from_pair(1, 2) {
            return Expr::int(1);
        }
        if rem == Rat::from_pair(3, 2) {
            return Expr::int(-1);
        }
        if rem == Rat::from_pair(1, 6) || rem == Rat::from_pair(5, 6) {
            return Expr::frac(1, 2);
        }
        if rem == Rat::from_pair(7, 6) || rem == Rat::from_pair(11, 6) {
            return Expr::frac(-1, 2);
        }
    }
    Expr::sin(c)
}

fn const_cos(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_zero() {
        return Expr::int(1);
    }
    if let Some(k) = pi_multiple(&c) {
        let two = Rat::from_int(2);
        let q = (&k / &two).floor();
        let rem = k - q * &two;
        if rem.is_zero() {
            return Expr::int(1);
        }
        if rem == Rat::from_int(1) {
            return Expr::int(-1);
        }
        if rem == Rat::from_pair(1, 2) || rem == Rat::from_pair(3, 2) {
            return Expr::int(0);
        }
        if rem == Rat::from_pair(1, 3) || rem == Rat::from_pair(5, 3) {
            return Expr::frac(1, 2);
        }
        if rem == Rat::from_pair(2, 3) || rem == Rat::from_pair(4, 3) {
            return Expr::frac(-1, 2);
        }
    }
    Expr::cos(c)
}

fn const_exp(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_zero() {
        Expr::int(1)
    } else if c.is_one() {
        Expr::e()
    } else if let Expr::Ln(inner) = c {
        *inner
    } else {
        Expr::exp(c)
    }
}

fn const_ln(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_one() {
        Expr::int(0)
    } else if matches!(c, Expr::E) {
        Expr::int(1)
    } else {
        Expr::ln(c)
    }
}

fn const_sinh(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_zero() {
        Expr::int(0)
    } else {
        Expr::sinh(c)
    }
}

fn const_cosh(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_zero() {
        Expr::int(1)
    } else {
        Expr::cosh(c)
    }
}

fn const_asin(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_zero() {
        Expr::int(0)
    } else if c.is_one() {
        Expr::div(Expr::pi(), Expr::int(2))
    } else if matches!(&c, Expr::Const(r) if *r == Rat::from_int(-1)) {
        Expr::neg(Expr::div(Expr::pi(), Expr::int(2)))
    } else {
        Expr::asin(c)
    }
}

fn const_acos(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_one() {
        Expr::int(0)
    } else if c.is_zero() {
        Expr::div(Expr::pi(), Expr::int(2))
    } else if matches!(&c, Expr::Const(r) if *r == Rat::from_int(-1)) {
        Expr::pi()
    } else {
        Expr::acos(c)
    }
}

fn const_atan(c: &Expr) -> Expr {
    let c = fold(c.clone());
    if c.is_zero() {
        Expr::int(0)
    } else if c.is_one() {
        Expr::div(Expr::pi(), Expr::int(4))
    } else if matches!(&c, Expr::Const(r) if *r == Rat::from_int(-1)) {
        Expr::neg(Expr::div(Expr::pi(), Expr::int(4)))
    } else {
        Expr::atan(c)
    }
}

fn const_pow(base: &Expr, exp: &Expr) -> Expr {
    let base = fold(base.clone());
    let exp = fold(exp.clone());
    if exp.is_zero() && !base.is_zero() {
        return Expr::int(1);
    }
    if exp.is_one() {
        return base;
    }
    if base.is_one() {
        return Expr::int(1);
    }
    if let (Expr::Const(b), Expr::Const(e)) = (&base, &exp)
        && *e == Rat::from_pair(1, 2)
        && *b >= BigRational::zero()
    {
        let num_sqrt = b.numer().sqrt();
        let den_sqrt = b.denom().sqrt();
        if &num_sqrt * &num_sqrt == *b.numer() && &den_sqrt * &den_sqrt == *b.denom() {
            return Expr::constant(BigRational::new(num_sqrt, den_sqrt));
        }
    }
    Expr::pow(base, exp)
}

fn sin_series(g: Series) -> Option<Series> {
    if g.valuation < 0 {
        return None;
    }
    let var = g.var.clone();
    let center = g.center.clone();
    let order = g.order;
    let c = g.constant_term()?;
    let h = g.sub(&Series::constant(c.clone(), &var, &center, order));
    let (sinh, cosh) = if h.coeffs.is_empty() {
        (
            Series::zero(&var, &center, order),
            Series::constant(Expr::int(1), &var, &center, order),
        )
    } else {
        (
            sin_kernel(&var, &center, order).compose(&h)?,
            cos_kernel(&var, &center, order).compose(&h)?,
        )
    };
    Some(cosh.scale(&const_sin(&c)).add(&sinh.scale(&const_cos(&c))))
}

fn cos_series(g: Series) -> Option<Series> {
    if g.valuation < 0 {
        return None;
    }
    let var = g.var.clone();
    let center = g.center.clone();
    let order = g.order;
    let c = g.constant_term()?;
    let h = g.sub(&Series::constant(c.clone(), &var, &center, order));
    let (sinh, cosh) = if h.coeffs.is_empty() {
        (
            Series::zero(&var, &center, order),
            Series::constant(Expr::int(1), &var, &center, order),
        )
    } else {
        (
            sin_kernel(&var, &center, order).compose(&h)?,
            cos_kernel(&var, &center, order).compose(&h)?,
        )
    };
    Some(cosh.scale(&const_cos(&c)).sub(&sinh.scale(&const_sin(&c))))
}

fn exp_series(g: Series) -> Option<Series> {
    if g.valuation < 0 {
        return None;
    }
    let var = g.var.clone();
    let center = g.center.clone();
    let order = g.order;
    let c = g.constant_term()?;
    let h = g.sub(&Series::constant(c.clone(), &var, &center, order));
    let exph = if h.coeffs.is_empty() {
        Series::constant(Expr::int(1), &var, &center, order)
    } else {
        exp_kernel(&var, &center, order).compose(&h)?
    };
    let ec = const_exp(&c);
    Some(exph.scale(&ec))
}

fn ln_series(g: Series) -> Option<Series> {
    if g.valuation < 0 {
        return None;
    }
    let var = g.var.clone();
    let center = g.center.clone();
    let order = g.order;
    let c = g.constant_term()?;
    if c.is_zero() {
        return None;
    }
    let h = g.sub(&Series::constant(c.clone(), &var, &center, order));
    let ln1ph = if h.coeffs.is_empty() {
        Series::zero(&var, &center, order)
    } else {
        let hc = h.scale(&Expr::div(Expr::int(1), c.clone()));
        ln1p_kernel(&var, &center, order).compose(&hc)?
    };
    let lnc = const_ln(&c);
    Some(ln1ph.add(&Series::constant(lnc, &var, &center, order)))
}

fn sinh_series(g: Series) -> Option<Series> {
    if g.valuation < 0 {
        return None;
    }
    let var = g.var.clone();
    let center = g.center.clone();
    let order = g.order;
    let c = g.constant_term()?;
    let h = g.sub(&Series::constant(c.clone(), &var, &center, order));
    let (sh, ch) = if h.coeffs.is_empty() {
        (
            Series::zero(&var, &center, order),
            Series::constant(Expr::int(1), &var, &center, order),
        )
    } else {
        let ep = exp_kernel(&var, &center, order).compose(&h)?;
        let em = exp_kernel(&var, &center, order).compose(&h.neg())?;
        let half = Expr::frac(1, 2);
        (ep.sub(&em).scale(&half), ep.add(&em).scale(&half))
    };
    Some(ch.scale(&const_sinh(&c)).add(&sh.scale(&const_cosh(&c))))
}

fn cosh_series(g: Series) -> Option<Series> {
    if g.valuation < 0 {
        return None;
    }
    let var = g.var.clone();
    let center = g.center.clone();
    let order = g.order;
    let c = g.constant_term()?;
    let h = g.sub(&Series::constant(c.clone(), &var, &center, order));
    let (sh, ch) = if h.coeffs.is_empty() {
        (
            Series::zero(&var, &center, order),
            Series::constant(Expr::int(1), &var, &center, order),
        )
    } else {
        let ep = exp_kernel(&var, &center, order).compose(&h)?;
        let em = exp_kernel(&var, &center, order).compose(&h.neg())?;
        let half = Expr::frac(1, 2);
        (ep.sub(&em).scale(&half), ep.add(&em).scale(&half))
    };
    Some(ch.scale(&const_cosh(&c)).add(&sh.scale(&const_sinh(&c))))
}

/// Term-by-term formal integral of a non-singular series (valuation >= 0) with constant term `c0`.
fn integrate_series(s: &Series, c0: Expr) -> Option<Series> {
    if s.valuation < 0 {
        return None;
    }
    let new_order = s.order + 1;
    let mut coeffs = vec![Expr::int(0); new_order.max(0) as usize];
    if new_order > 0 {
        coeffs[0] = c0;
    }
    for (i, c) in s.coeffs.iter().enumerate() {
        let e = s.valuation + i as i64;
        let ne = e + 1;
        if ne >= 0 && ne < new_order {
            coeffs[ne as usize] = Expr::div(c.clone(), Expr::int(ne));
        }
    }
    Some(
        Series {
            var: s.var.clone(),
            center: s.center.clone(),
            valuation: 0,
            coeffs,
            order: new_order,
        }
        .normalized(),
    )
}

fn pow_series_symbolic(g: Series, alpha: &Expr) -> Option<Series> {
    if g.valuation != 0 {
        return None;
    }
    let var = g.var.clone();
    let center = g.center.clone();
    let order = g.order;
    let c0 = g.constant_term()?;
    if c0.is_zero() {
        return None;
    }
    let h = g
        .scale(&Expr::div(Expr::int(1), c0.clone()))
        .sub(&Series::constant(Expr::int(1), &var, &center, order));
    let mut acc = Series::constant(Expr::int(1), &var, &center, order);
    if !h.coeffs.is_empty() && order > 1 {
        let mut h_pow = Series::constant(Expr::int(1), &var, &center, order);
        let mut binom = Expr::int(1);
        for k in 1..order {
            let factor = fold(Expr::sub(alpha.clone(), Expr::int(k - 1)));
            binom = fold(Expr::div(Expr::mul(binom, factor), Expr::int(k)));
            h_pow = h_pow.mul(&h);
            if h_pow.coeffs.is_empty() {
                break;
            }
            acc = acc.add(&h_pow.scale(&binom));
        }
    }
    let c0_pow = const_pow(&c0, alpha);
    Some(acc.scale(&c0_pow))
}

fn pow_series_rational(g: Series, p: i64, q: i64) -> Option<Series> {
    if q <= 0 || g.coeffs.is_empty() || g.valuation % q != 0 {
        return None;
    }
    let shift = (g.valuation / q) * p;
    let shifted = Series {
        var: g.var.clone(),
        center: g.center.clone(),
        valuation: 0,
        coeffs: g.coeffs.clone(),
        order: g.order - g.valuation,
    };
    let alpha = Expr::frac(p, q);
    let mut base_res = pow_series_symbolic(shifted, &alpha)?;
    base_res.valuation += shift;
    base_res.order += shift;
    Some(base_res.normalized())
}

fn expand(expr: &Expr, var: &str, center: &Expr, order: i64) -> Option<Series> {
    if !expr.variables().contains(&var.to_string()) {
        return Some(Series::constant(expr.clone(), var, center, order));
    }
    match expr {
        Expr::Const(r) => Some(Series::constant(
            Expr::constant(r.clone()),
            var,
            center,
            order,
        )),
        Expr::Var(name) if name == var => {
            let n = order.max(0) as usize;
            let mut coeffs = vec![Expr::int(0); n];
            if order > 0 {
                coeffs[0] = center.clone();
            }
            if order > 1 {
                coeffs[1] = Expr::int(1);
            }
            Some(
                Series {
                    var: var.to_string(),
                    center: center.clone(),
                    valuation: 0,
                    coeffs,
                    order,
                }
                .normalized(),
            )
        }
        Expr::Add(a, b) => {
            Some(expand(a, var, center, order)?.add(&expand(b, var, center, order)?))
        }
        Expr::Sub(a, b) => {
            Some(expand(a, var, center, order)?.sub(&expand(b, var, center, order)?))
        }
        Expr::Mul(a, b) => {
            let sa0 = expand(a, var, center, order)?;
            let sb0 = expand(b, var, center, order)?;
            let need_a = if sb0.valuation < 0 {
                order - sb0.valuation
            } else {
                order
            };
            let need_b = if sa0.valuation < 0 {
                order - sa0.valuation
            } else {
                order
            };
            let sa = if need_a > order {
                expand(a, var, center, need_a)?
            } else {
                sa0
            };
            let sb = if need_b > order {
                expand(b, var, center, need_b)?
            } else {
                sb0
            };
            Some(sa.mul(&sb))
        }
        Expr::Div(a, b) => {
            let sb0 = expand(b, var, center, order)?;
            let sa0 = expand(a, var, center, order)?;
            let vb = sb0.valuation;
            let va = sa0.valuation;
            let extra_b = 2 * vb.max(0) + (-va).max(0);
            let extra_a = vb.max(0);
            let sb = if extra_b > 0 {
                expand(b, var, center, order + extra_b)?
            } else {
                sb0
            };
            let sa = if extra_a > 0 {
                expand(a, var, center, order + extra_a)?
            } else {
                sa0
            };
            sa.div(&sb)
        }
        Expr::Neg(a) => Some(expand(a, var, center, order)?.neg()),
        Expr::Pow(base, exp) => {
            if !exp.variables().contains(&var.to_string()) {
                if let Some(n) = const_rat(exp) {
                    if Rat::is_integer(&n) {
                        let ni = n.numer().to_i64()?;
                        if ni < 0 {
                            let sb0 = expand(base, var, center, order)?;
                            let extra = if sb0.valuation > 0 {
                                ((-ni) + 1) * sb0.valuation
                            } else {
                                0
                            };
                            let sb = if extra > 0 {
                                expand(base, var, center, order + extra)?
                            } else {
                                sb0
                            };
                            return sb.pow_int(ni);
                        }
                        return expand(base, var, center, order)?.pow_int(ni);
                    }
                    let p = n.numer().to_i64()?;
                    let q = n.denom().to_i64()?;
                    let sb = expand(base, var, center, order)?;
                    return pow_series_rational(sb, p, q);
                }
                if !base.variables().contains(&var.to_string()) {
                    let g = expand(exp, var, center, order)?;
                    return exp_series(g.scale(&const_ln(base)));
                }
                let sb = expand(base, var, center, order)?;
                return pow_series_symbolic(sb, exp);
            }
            // General u^v = exp(v * ln(u))
            let g_ln = ln_series(expand(base, var, center, order)?)?;
            let g_exp = expand(exp, var, center, order)?;
            exp_series(g_exp.mul(&g_ln))
        }
        Expr::Sqrt(arg) => {
            let sb = expand(arg, var, center, order)?;
            pow_series_rational(sb, 1, 2)
        }
        Expr::Sin(arg) => sin_series(expand(arg, var, center, order)?),
        Expr::Cos(arg) => cos_series(expand(arg, var, center, order)?),
        Expr::Tan(arg) => {
            let c0 = cos_series(expand(arg, var, center, order)?)?;
            let extra = 2 * c0.valuation.max(0);
            let g = expand(arg, var, center, order + extra)?;
            let s = sin_series(g.clone())?;
            let c = cos_series(g)?;
            s.div(&c)
        }
        Expr::Exp(arg) => exp_series(expand(arg, var, center, order)?),
        Expr::Ln(arg) => ln_series(expand(arg, var, center, order)?),
        Expr::Sinh(arg) => sinh_series(expand(arg, var, center, order)?),
        Expr::Cosh(arg) => cosh_series(expand(arg, var, center, order)?),
        Expr::Tanh(arg) => {
            let c0 = cosh_series(expand(arg, var, center, order)?)?;
            let extra = 2 * c0.valuation.max(0);
            let g = expand(arg, var, center, order + extra)?;
            let sh = sinh_series(g.clone())?;
            let ch = cosh_series(g)?;
            sh.div(&ch)
        }
        Expr::Atan(arg) => {
            let g = expand(arg, var, center, order + 1)?;
            let c = g.constant_term()?;
            let dg = g.diff();
            let g_trunc = expand(arg, var, center, order)?;
            let denom =
                Series::constant(Expr::int(1), var, center, order).add(&g_trunc.mul(&g_trunc));
            let integrand = dg.div(&denom)?;
            integrate_series(&integrand, const_atan(&c))
        }
        Expr::Asin(arg) => {
            let g = expand(arg, var, center, order + 1)?;
            let c = g.constant_term()?;
            let dg = g.diff();
            let g_trunc = expand(arg, var, center, order)?;
            let one_minus_g2 =
                Series::constant(Expr::int(1), var, center, order).sub(&g_trunc.mul(&g_trunc));
            let denom = pow_series_rational(one_minus_g2, 1, 2)?;
            let integrand = dg.div(&denom)?;
            integrate_series(&integrand, const_asin(&c))
        }
        Expr::Acos(arg) => {
            let g = expand(arg, var, center, order + 1)?;
            let c = g.constant_term()?;
            let dg = g.diff();
            let g_trunc = expand(arg, var, center, order)?;
            let one_minus_g2 =
                Series::constant(Expr::int(1), var, center, order).sub(&g_trunc.mul(&g_trunc));
            let denom = pow_series_rational(one_minus_g2, 1, 2)?;
            let integrand = dg.div(&denom)?.neg();
            integrate_series(&integrand, const_acos(&c))
        }
        _ => None,
    }
}
