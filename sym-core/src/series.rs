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

/// Fold constant-only arithmetic so zero/coefficient detection works.
fn fold(e: Expr) -> Expr {
    match e {
        Expr::Neg(a) => {
            let a = fold(*a);
            match a {
                Expr::Const(r) => Expr::constant(-r),
                other => Expr::Neg(Box::new(other)),
            }
        }
        Expr::Add(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x + y),
                _ => Expr::Add(Box::new(a), Box::new(b)),
            }
        }
        Expr::Sub(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x - y),
                _ => Expr::Sub(Box::new(a), Box::new(b)),
            }
        }
        Expr::Mul(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) => Expr::constant(x * y),
                _ => Expr::Mul(Box::new(a), Box::new(b)),
            }
        }
        Expr::Div(a, b) => {
            let a = fold(*a);
            let b = fold(*b);
            match (&a, &b) {
                (Expr::Const(x), Expr::Const(y)) if !y.is_zero() => Expr::constant(x / y),
                _ => Expr::Div(Box::new(a), Box::new(b)),
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
    #[allow(clippy::should_implement_trait)]
    pub fn mul(&self, other: &Series) -> Series {
        let order = self.order.min(other.order);
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
    pub fn inv(&self) -> Option<Series> {
        if self.coeffs.is_empty() {
            return None;
        }
        let c0 = self.coeff_at(self.valuation);
        if c0.is_zero() {
            return None;
        }
        let v = self.valuation;
        let order = self.order;
        let new_v = -v;
        let n = (order - new_v).max(0) as usize;
        if n == 0 {
            return Some(Series {
                var: self.var.clone(),
                center: self.center.clone(),
                valuation: new_v,
                coeffs: Vec::new(),
                order,
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
                order,
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
    pub fn diff(&self) -> Series {
        let new_v = self.valuation - 1;
        let order = self.order;
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

/// Expand `expr` as a truncated Laurent/Taylor series about `center`, order `order`.
pub fn series_expand(expr: &Expr, var: &str, center: &Expr, order: i64) -> Option<Series> {
    expand(expr, var, center, order)
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

fn const_sin(c: &Expr) -> Expr {
    if c.is_zero() {
        Expr::int(0)
    } else {
        Expr::sin(c.clone())
    }
}

fn const_cos(c: &Expr) -> Expr {
    if c.is_zero() {
        Expr::int(1)
    } else {
        Expr::cos(c.clone())
    }
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
    let ec = if c.is_zero() {
        Expr::int(1)
    } else {
        Expr::exp(c)
    };
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
    let lnc = if c.is_one() {
        Expr::int(0)
    } else {
        Expr::ln(c)
    };
    Some(ln1ph.add(&Series::constant(lnc, &var, &center, order)))
}

fn expand(expr: &Expr, var: &str, center: &Expr, order: i64) -> Option<Series> {
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
            Some(expand(a, var, center, order)?.mul(&expand(b, var, center, order)?))
        }
        Expr::Div(a, b) => {
            let sa = expand(a, var, center, order)?;
            let sb = expand(b, var, center, order)?;
            sa.div(&sb)
        }
        Expr::Neg(a) => Some(expand(a, var, center, order)?.neg()),
        Expr::Pow(base, exp) => {
            let n = const_rat(exp)?;
            if !Rat::is_integer(&n) {
                return None;
            }
            let ni = n.numer().to_i64()?;
            expand(base, var, center, order)?.pow_int(ni)
        }
        Expr::Sin(arg) => sin_series(expand(arg, var, center, order)?),
        Expr::Cos(arg) => cos_series(expand(arg, var, center, order)?),
        Expr::Exp(arg) => exp_series(expand(arg, var, center, order)?),
        Expr::Ln(arg) => ln_series(expand(arg, var, center, order)?),
        _ => None,
    }
}
