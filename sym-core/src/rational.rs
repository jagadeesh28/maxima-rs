//! Rational constant utilities for exact symbolic arithmetic.
//!
//! All numeric constants in the [`Expr`](crate::expr::Expr) AST are stored as
//! [`BigRational`] values. This module provides ergonomic constructors,
//! predicates, and conversion routines.
//!
//! # Why Rationals? (ADR-0004)
//!
//! Using exact rational arithmetic during symbolic simplification prevents
//! rounding-induced bugs. For example, `(3/7) * (7/3)` folds to exactly `1`
//! rather than `0.9999999999999998`. Conversion to `f64`/`f32` happens only
//! once, during final code generation.

use num_bigint::BigInt;
pub use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// Utility namespace for [`BigRational`] operations.
///
/// Not a wrapper type — these are free functions and associated functions
/// to construct, inspect, and convert `BigRational` values.
pub struct Rat;

impl Rat {
    // ── Constructors ───────────────────────────────────────────────

    /// Create a rational from an integer.
    ///
    /// ```
    /// # use sym_core::rational::Rat;
    /// # use num_traits::Zero;
    /// let zero = Rat::from_int(0);
    /// assert!(zero.is_zero());
    /// ```
    pub fn from_int(n: i64) -> BigRational {
        BigRational::from_integer(BigInt::from(n))
    }

    /// Create an exact rational from numerator and denominator.
    ///
    /// # Panics
    ///
    /// Panics if `den` is zero.
    ///
    /// ```
    /// # use sym_core::rational::Rat;
    /// let half = Rat::from_pair(1, 2);
    /// assert_eq!(Rat::to_f64(&half), 0.5);
    /// ```
    pub fn from_pair(num: i64, den: i64) -> BigRational {
        assert!(den != 0, "denominator cannot be zero");
        BigRational::new(BigInt::from(num), BigInt::from(den))
    }

    /// Create a rational that best approximates the given `f64`.
    ///
    /// Uses continued fraction expansion to find a clean rational
    /// representation. Integer values and simple fractions (like 0.5 → 1/2,
    /// 0.25 → 1/4) are represented exactly.
    ///
    /// # Special Cases
    ///
    /// - `NaN`, `Inf`, `-Inf` all map to `0/1`.
    ///
    /// ```
    /// # use sym_core::rational::Rat;
    /// assert_eq!(Rat::to_f64(&Rat::from_f64(1.5)), 1.5);
    /// assert_eq!(Rat::to_f64(&Rat::from_f64(100.0)), 100.0);
    /// assert_eq!(Rat::to_f64(&Rat::from_f64(0.25)), 0.25);
    /// ```
    pub fn from_f64(v: f64) -> BigRational {
        if !v.is_finite() || v == 0.0 {
            return BigRational::zero();
        }
        BigRational::from_float(v).unwrap_or_else(BigRational::zero)
    }

    // ── Predicates ─────────────────────────────────────────────────

    /// Check if the rational is exactly zero.
    pub fn is_zero(r: &BigRational) -> bool {
        r.is_zero()
    }

    /// Check if the rational is exactly one.
    pub fn is_one(r: &BigRational) -> bool {
        r.is_one()
    }

    /// Check if the rational is negative.
    pub fn is_negative(r: &BigRational) -> bool {
        r.is_negative()
    }

    /// Check if the rational represents an integer (denominator == 1).
    pub fn is_integer(r: &BigRational) -> bool {
        r.denom() == &BigInt::from(1)
    }

    // ── Conversion ─────────────────────────────────────────────────

    /// Convert a rational to `f64` for numerical evaluation.
    ///
    /// ```
    /// # use sym_core::rational::Rat;
    /// let third = Rat::from_pair(1, 3);
    /// let v = Rat::to_f64(&third);
    /// assert!((v - 1.0 / 3.0).abs() < 1e-15);
    /// ```
    pub fn to_f64(r: &BigRational) -> f64 {
        let max_bits = r.numer().bits().max(r.denom().bits());
        if max_bits > 1000 {
            let shift = max_bits - 1000;
            let n_scaled = r.numer() >> shift;
            let d_scaled = r.denom() >> shift;
            return n_scaled.to_f64().unwrap_or(f64::NAN) / d_scaled.to_f64().unwrap_or(1.0);
        }
        r.numer().to_f64().unwrap_or(f64::NAN) / r.denom().to_f64().unwrap_or(1.0)
    }

    /// Format the rational as an `f64` literal string for codegen.
    ///
    /// Always produces a representation with a decimal point to ensure
    /// the Rust compiler interprets it as a float.
    ///
    /// ```
    /// # use sym_core::rational::Rat;
    /// assert_eq!(Rat::to_f64_literal(&Rat::from_int(2)), "2.0_f64");
    /// assert_eq!(Rat::to_f64_literal(&Rat::from_pair(1, 3)), "0.3333333333333333_f64");
    /// ```
    pub fn to_f64_literal(r: &BigRational) -> String {
        let v = Self::to_f64(r);
        if v == v.floor() && v.is_finite() {
            // Integer value: ensure decimal point
            format!("{v:.1}_f64")
        } else {
            format!("{v}_f64")
        }
    }

    /// Format the rational as an `f32` literal string for codegen.
    pub fn to_f32_literal(r: &BigRational) -> String {
        let v = Self::to_f64(r) as f32;
        if v == v.floor() && v.is_finite() {
            format!("{v:.1}_f32")
        } else {
            format!("{v}_f32")
        }
    }

    // ── Arithmetic helpers for constant folding ────────────────────

    /// Attempt to compute `a + b` as rationals.
    pub fn add(a: &BigRational, b: &BigRational) -> BigRational {
        a + b
    }

    /// Attempt to compute `a - b` as rationals.
    pub fn sub(a: &BigRational, b: &BigRational) -> BigRational {
        a - b
    }

    /// Attempt to compute `a * b` as rationals.
    pub fn mul(a: &BigRational, b: &BigRational) -> BigRational {
        a * b
    }

    /// Attempt to compute `a / b` as rationals. Returns `None` if `b` is zero.
    pub fn div(a: &BigRational, b: &BigRational) -> Option<BigRational> {
        if b.is_zero() { None } else { Some(a / b) }
    }

    /// Attempt to compute `base ^ exp` when `exp` is an integer.
    /// Returns `None` if `exp` is not an integer or the result overflows.
    pub fn pow_int(base: &BigRational, exp: &BigRational) -> Option<BigRational> {
        if !Self::is_integer(exp) {
            return None;
        }
        let n = exp.numer().to_i64()?;
        let abs_n = n.unsigned_abs();
        if abs_n > 10_000 {
            return None;
        }
        if n == 0 {
            if base.is_zero() {
                return None; // 0^0 is undefined
            }
            return Some(BigRational::one());
        }
        if n < 0 {
            if base.is_zero() {
                return None; // 0^(-n) is undefined
            }
            // base^(-n) = (1/base)^n
            let inv = BigRational::new(base.denom().clone(), base.numer().clone());
            Some(num_traits::pow(inv, abs_n as usize))
        } else {
            Some(num_traits::pow(base.clone(), abs_n as usize))
        }
    }

    /// Attempt to compute `n!` for a non-negative integer rational.
    ///
    /// Returns `None` for non-integers, negative values, or arguments above
    /// `100` (guarding against pathological constant folding).
    ///
    /// ```ignore
    /// assert_eq!(Rat::factorial(&Rat::from_int(5)), Some(Rat::from_int(120)));
    /// ```
    pub fn factorial(r: &BigRational) -> Option<BigRational> {
        if !Self::is_integer(r) {
            return None;
        }
        let n = r.numer().to_u32()?;
        if n > 100 {
            return None;
        }
        let mut acc = BigInt::one();
        for k in 2..=n {
            acc *= BigInt::from(k);
        }
        Some(BigRational::from_integer(acc))
    }

    /// Negate a rational.
    pub fn negate(r: &BigRational) -> BigRational {
        -r
    }

    /// Parse an integer, decimal, or scientific notation string into an exact [`BigRational`].
    pub fn parse_exact_rational(s: &str) -> Option<BigRational> {
        use num_traits::Pow;

        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let (mantissa, exp_val): (&str, i32) = if let Some(pos) = s.find(['e', 'E']) {
            let (m, e) = s.split_at(pos);
            let e_str = &e[1..];
            let exp = e_str.parse::<i32>().ok()?;
            (m, exp)
        } else {
            (s, 0)
        };

        let (digits_big, frac_len): (BigInt, i32) =
            if let Some((int_part, frac_part)) = mantissa.split_once('.') {
                let combined = format!("{}{}", int_part, frac_part);
                let digits = combined.parse::<BigInt>().ok()?;
                (digits, frac_part.len() as i32)
            } else {
                let digits = mantissa.parse::<BigInt>().ok()?;
                (digits, 0)
            };

        let net_exp = exp_val.checked_sub(frac_len)?;
        if net_exp.unsigned_abs() > 10_000 {
            return None;
        }
        let ten = BigInt::from(10);
        if net_exp >= 0 {
            let factor = ten.pow(net_exp as u32);
            Some(BigRational::from_integer(digits_big * factor))
        } else {
            let divisor = ten.pow((-net_exp) as u32);
            Some(BigRational::new(digits_big, divisor))
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_int() {
        let r = Rat::from_int(42);
        assert_eq!(Rat::to_f64(&r), 42.0);
        assert!(Rat::is_integer(&r));
    }

    #[test]
    fn test_from_int_zero() {
        let r = Rat::from_int(0);
        assert!(Rat::is_zero(&r));
    }

    #[test]
    fn test_from_pair() {
        let r = Rat::from_pair(1, 2);
        assert_eq!(Rat::to_f64(&r), 0.5);
        assert!(!Rat::is_integer(&r));
    }

    #[test]
    fn test_from_pair_reduces() {
        // 4/6 should reduce to 2/3
        let r = Rat::from_pair(4, 6);
        assert_eq!(r.numer(), &BigInt::from(2));
        assert_eq!(r.denom(), &BigInt::from(3));
    }

    #[test]
    #[should_panic(expected = "denominator cannot be zero")]
    fn test_from_pair_zero_denom() {
        Rat::from_pair(1, 0);
    }

    #[test]
    fn test_from_f64_integer() {
        let r = Rat::from_f64(100.0);
        assert!(Rat::is_integer(&r));
        assert_eq!(Rat::to_f64(&r), 100.0);
    }

    #[test]
    fn test_from_f64_half() {
        let r = Rat::from_f64(0.5);
        assert_eq!(r.numer(), &BigInt::from(1));
        assert_eq!(r.denom(), &BigInt::from(2));
    }

    #[test]
    fn test_from_f64_one_point_five() {
        let r = Rat::from_f64(1.5);
        assert_eq!(r.numer(), &BigInt::from(3));
        assert_eq!(r.denom(), &BigInt::from(2));
    }

    #[test]
    fn test_from_f64_quarter() {
        let r = Rat::from_f64(0.25);
        assert_eq!(r.numer(), &BigInt::from(1));
        assert_eq!(r.denom(), &BigInt::from(4));
    }

    #[test]
    fn test_from_f64_negative() {
        let r = Rat::from_f64(-2.5);
        assert!(Rat::is_negative(&r));
        assert_eq!(Rat::to_f64(&r), -2.5);
    }

    #[test]
    fn test_from_f64_nan() {
        let r = Rat::from_f64(f64::NAN);
        assert!(Rat::is_zero(&r));
    }

    #[test]
    fn test_from_f64_inf() {
        let r = Rat::from_f64(f64::INFINITY);
        assert!(Rat::is_zero(&r));
    }

    #[test]
    fn test_from_f64_third() {
        let r = Rat::from_f64(1.0 / 3.0);
        // Should approximate closely
        let v = Rat::to_f64(&r);
        assert!((v - 1.0 / 3.0).abs() < 1e-15);
    }

    #[test]
    fn test_is_one() {
        assert!(Rat::is_one(&Rat::from_int(1)));
        assert!(!Rat::is_one(&Rat::from_int(2)));
    }

    #[test]
    fn test_to_f64_literal_integer() {
        let r = Rat::from_int(2);
        assert_eq!(Rat::to_f64_literal(&r), "2.0_f64");
    }

    #[test]
    fn test_to_f64_literal_fraction() {
        let r = Rat::from_pair(1, 3);
        let lit = Rat::to_f64_literal(&r);
        assert!(lit.ends_with("_f64"));
        assert!(lit.contains("0.333333333333333"));
    }

    #[test]
    fn test_to_f32_literal() {
        let r = Rat::from_int(2);
        assert_eq!(Rat::to_f32_literal(&r), "2.0_f32");
    }

    #[test]
    fn test_arithmetic_add() {
        let a = Rat::from_pair(1, 3);
        let b = Rat::from_pair(1, 6);
        let c = Rat::add(&a, &b);
        // 1/3 + 1/6 = 1/2
        assert_eq!(c.numer(), &BigInt::from(1));
        assert_eq!(c.denom(), &BigInt::from(2));
    }

    #[test]
    fn test_arithmetic_mul() {
        let a = Rat::from_pair(3, 7);
        let b = Rat::from_pair(7, 3);
        let c = Rat::mul(&a, &b);
        // (3/7) * (7/3) = 1 exactly
        assert!(Rat::is_one(&c));
    }

    #[test]
    fn test_arithmetic_div_by_zero() {
        let a = Rat::from_int(1);
        let b = Rat::from_int(0);
        assert!(Rat::div(&a, &b).is_none());
    }

    #[test]
    fn test_pow_int_positive() {
        let base = Rat::from_pair(2, 3);
        let exp = Rat::from_int(3);
        let result = Rat::pow_int(&base, &exp).unwrap();
        // (2/3)^3 = 8/27
        assert_eq!(result.numer(), &BigInt::from(8));
        assert_eq!(result.denom(), &BigInt::from(27));
    }

    #[test]
    fn test_pow_int_negative() {
        let base = Rat::from_pair(2, 3);
        let exp = Rat::from_int(-2);
        let result = Rat::pow_int(&base, &exp).unwrap();
        // (2/3)^(-2) = (3/2)^2 = 9/4
        assert_eq!(result.numer(), &BigInt::from(9));
        assert_eq!(result.denom(), &BigInt::from(4));
    }

    #[test]
    fn test_pow_int_zero() {
        let base = Rat::from_pair(5, 7);
        let exp = Rat::from_int(0);
        let result = Rat::pow_int(&base, &exp).unwrap();
        // anything^0 = 1
        assert!(Rat::is_one(&result));
    }

    #[test]
    fn test_pow_non_integer_exp() {
        let base = Rat::from_int(4);
        let exp = Rat::from_pair(1, 2);
        // 4^(1/2) can't be computed exactly as rational
        assert!(Rat::pow_int(&base, &exp).is_none());
    }

    #[test]
    fn test_phase47_rat_edge_cases() {
        // 0^0 -> None
        assert!(Rat::pow_int(&Rat::from_int(0), &Rat::from_int(0)).is_none());
        // i64::MIN -> None (no panic/overflow)
        assert!(Rat::pow_int(&Rat::from_int(2), &Rat::from_int(i64::MIN)).is_none());
        // 10^400 / (10^400 + 1) -> 1.0 (not NaN from inf/inf)
        let ten_400 = Rat::pow_int(&Rat::from_int(10), &Rat::from_int(400)).unwrap();
        let ratio = &ten_400 / (&ten_400 + Rat::from_int(1));
        let f = Rat::to_f64(&ratio);
        assert!((f - 1.0).abs() < 1e-12, "got {f}");
        // 1e1000000000 rejected
        assert!(Rat::parse_exact_rational("1e1000000000").is_none());
    }
}
