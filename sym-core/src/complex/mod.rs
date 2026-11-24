//! Complex analysis, rectangular form decomposition, and residue calculus (Phase 41).
//!
//! # Primary Reference Sources
//! - **Upstream Maxima 5.50.0**: `src/rpart.lisp` (`risplit`, `trisplit`, `$rectform`,
//!   `$polarform`, `cabs`, `absarg`, `genatan`).
//! - **ADR-0010**: Complex Atoms, `Expr::I`, and Canonical Rectangular Decomposition.
//! - **ADR-0012**: Tri-State Outcomes and Verification Honesty.
//!
//! # Architectural Overview
//! All complex expressions are decomposed into real and imaginary components
//! `expr = u + i*v` where `u` and `v` are strictly real-valued expressions
//! (variables are assumed real by default, matching Maxima 5.50's `rpart.lisp`).
//! Procedural evaluation handles:
//! - Exact algebraic arithmetic (sum, difference, product, quotient)
//! - Integer and fractional powers (including cyclic powers $i^k$)
//! - Euler's formula for exponentials $\exp(u + iv) = e^u(\cos v + i\sin v)$
//! - Complex logarithms $\ln(u + iv) = \frac{1}{2}\ln(u^2+v^2) + i\operatorname{atan2}(v, u)$
//! - Complex trigonometric & hyperbolic functions
//! - Laurent series residue extraction via [`crate::series::series_expand`].

use crate::expr::Expr;
use crate::rational::Rat;
use num_rational::BigRational;
use num_traits::{Signed, ToPrimitive, Zero};

/// Smart constructor: $a + b$ with immediate zero and constant folding.
fn smart_add(a: Expr, b: Expr) -> Expr {
    if a.is_zero() {
        return b;
    }
    if b.is_zero() {
        return a;
    }
    if let (Expr::Const(ra), Expr::Const(rb)) = (&a, &b) {
        return Expr::constant(ra + rb);
    }
    if let Expr::Neg(inner) = b {
        return smart_sub(a, *inner);
    }
    if let Expr::Neg(inner) = a {
        return smart_sub(b, *inner);
    }
    Expr::add(a, b)
}

/// Smart constructor: $a - b$ with immediate zero and constant folding.
fn smart_sub(a: Expr, b: Expr) -> Expr {
    if b.is_zero() {
        return a;
    }
    if a.is_zero() {
        return smart_neg(b);
    }
    if a == b {
        return Expr::int(0);
    }
    if let (Expr::Const(ra), Expr::Const(rb)) = (&a, &b) {
        return Expr::constant(ra - rb);
    }
    if let Expr::Neg(inner) = b {
        return smart_add(a, *inner);
    }
    Expr::sub(a, b)
}

/// Smart constructor: $-a$ with immediate zero and constant folding.
fn smart_neg(a: Expr) -> Expr {
    if a.is_zero() {
        return Expr::int(0);
    }
    if let Expr::Neg(inner) = a {
        return *inner;
    }
    if let Expr::Const(r) = a {
        return Expr::constant(-r);
    }
    Expr::neg(a)
}

/// Smart constructor: $a \cdot b$ with immediate zero, one, and constant folding.
fn smart_mul(a: Expr, b: Expr) -> Expr {
    if a.is_zero() || b.is_zero() {
        return Expr::int(0);
    }
    if a.is_one() {
        return b;
    }
    if b.is_one() {
        return a;
    }
    if a == Expr::int(-1) {
        return smart_neg(b);
    }
    if b == Expr::int(-1) {
        return smart_neg(a);
    }
    if let (Expr::Const(ra), Expr::Const(rb)) = (&a, &b) {
        return Expr::constant(ra * rb);
    }
    Expr::mul(a, b)
}

/// Smart constructor: $a / b$ with immediate zero, one, and constant folding.
fn smart_div(a: Expr, b: Expr) -> Expr {
    if a.is_zero() {
        return Expr::int(0);
    }
    if b.is_one() {
        return a;
    }
    if b == Expr::int(-1) {
        return smart_neg(a);
    }
    if a == b {
        return Expr::int(1);
    }
    if let (Expr::Const(ra), Expr::Const(rb)) = (&a, &b)
        && !rb.is_zero()
    {
        return Expr::constant(ra / rb);
    }
    Expr::div(a, b)
}

/// Smart constructor: $a^b$ with immediate zero, one, and constant folding.
fn smart_pow(base: Expr, exp: Expr) -> Expr {
    if exp.is_zero() {
        return Expr::int(1);
    }
    if exp.is_one() {
        return base;
    }
    if base.is_zero() {
        return Expr::int(0);
    }
    if base.is_one() {
        return Expr::int(1);
    }
    if let (Expr::Const(rb), Expr::Const(re)) = (&base, &exp)
        && let Some(res) = Rat::pow_int(rb, re)
    {
        return Expr::constant(res);
    }
    Expr::pow(base, exp)
}

/// Smart square root $\sqrt{a}$.
fn smart_sqrt(a: Expr) -> Expr {
    if a.is_zero() {
        return Expr::int(0);
    }
    if a.is_one() {
        return Expr::int(1);
    }
    if let Expr::Const(r) = &a
        && r.is_positive()
    {
        // Check perfect squares
        let num = r.numer();
        let den = r.denom();
        let s_num = num.sqrt();
        let s_den = den.sqrt();
        if &s_num * &s_num == *num && &s_den * &s_den == *den {
            return Expr::constant(BigRational::new(s_num, s_den));
        }
    }
    Expr::sqrt(a)
}

/// Checks if an expression is a known rational multiple of $\pi$, e.g. $k \cdot \pi$.
/// Returns `Some(k)` if $v = k \cdot \pi$.
fn pi_multiple(v: &Expr) -> Option<BigRational> {
    if *v == Expr::Pi {
        return Some(BigRational::from_integer(1.into()));
    }
    if let Expr::Neg(inner) = v {
        return pi_multiple(inner).map(|k| -k);
    }
    if let Expr::Mul(a, b) = v {
        if **b == Expr::Pi
            && let Expr::Const(r) = &**a
        {
            return Some(r.clone());
        }
        if **a == Expr::Pi
            && let Expr::Const(r) = &**b
        {
            return Some(r.clone());
        }
    }
    if let Expr::Div(a, b) = v
        && **a == Expr::Pi
        && let Expr::Const(r) = &**b
        && !r.is_zero()
    {
        return Some(BigRational::from_integer(1.into()) / r);
    }
    None
}

/// Evaluate exact trigonometric values for known multiples of $\pi$.
/// Returns `(cos(v), sin(v))`.
fn trig_angle(v: &Expr) -> (Option<Expr>, Option<Expr>) {
    if v.is_zero() {
        return (Some(Expr::int(1)), Some(Expr::int(0)));
    }
    if let Some(k) = pi_multiple(v) {
        // Evaluate k mod 2
        let two = BigRational::from_integer(2.into());
        let mut rem = &k % &two;
        if rem.is_negative() {
            rem += &two;
        }
        // Common exact angles
        let half = BigRational::new(1.into(), 2.into());
        let one = BigRational::from_integer(1.into());
        let three_halves = BigRational::new(3.into(), 2.into());

        if rem.is_zero() {
            return (Some(Expr::int(1)), Some(Expr::int(0)));
        } else if rem == half {
            // pi / 2
            return (Some(Expr::int(0)), Some(Expr::int(1)));
        } else if rem == one {
            // pi
            return (Some(Expr::int(-1)), Some(Expr::int(0)));
        } else if rem == three_halves {
            // 3*pi / 2
            return (Some(Expr::int(0)), Some(Expr::int(-1)));
        }
    }
    (None, None)
}

fn smart_sin(a: Expr) -> Expr {
    if a.is_zero() {
        return Expr::int(0);
    }
    if let (_, Some(s)) = trig_angle(&a) {
        return s;
    }
    Expr::sin(a)
}

fn smart_cos(a: Expr) -> Expr {
    if a.is_zero() {
        return Expr::int(1);
    }
    if let (Some(c), _) = trig_angle(&a) {
        return c;
    }
    Expr::cos(a)
}

fn smart_sinh(a: Expr) -> Expr {
    if a.is_zero() {
        return Expr::int(0);
    }
    Expr::sinh(a)
}

fn smart_cosh(a: Expr) -> Expr {
    if a.is_zero() {
        return Expr::int(1);
    }
    Expr::cosh(a)
}

/// Decompose an arbitrary symbolic expression into real and imaginary parts $(u, v)$
/// such that $expr = u + i \cdot v$.
///
/// Implements Maxima 5.50 `src/rpart.lisp` (`risplit`).
/// All symbols and constants are assumed real by default (ADR-0010).
pub fn rectform(expr: &Expr) -> (Expr, Expr) {
    match expr {
        // Maxima rpart.lisp:387 (cond ((eq l '$%i) (cons 0 1)))
        Expr::I => (Expr::int(0), Expr::int(1)),

        // Real atoms
        Expr::Const(_) | Expr::Var(_) | Expr::Pi | Expr::E => (expr.clone(), Expr::int(0)),
        #[cfg(feature = "bfloat")]
        Expr::Bfloat(_) => (expr.clone(), Expr::int(0)),
        Expr::Tensor(_, _) => (expr.clone(), Expr::int(0)),

        // Negation: -(u + iv) = -u - iv
        Expr::Neg(a) => {
            let (u, v) = rectform(a);
            (smart_neg(u), smart_neg(v))
        }

        // Addition: (u1 + iv1) + (u2 + iv2) = (u1 + u2) + i(v1 + v2)
        Expr::Add(a, b) => {
            let (u1, v1) = rectform(a);
            let (u2, v2) = rectform(b);
            (smart_add(u1, u2), smart_add(v1, v2))
        }

        // Subtraction: (u1 + iv1) - (u2 + iv2) = (u1 - u2) + i(v1 - v2)
        Expr::Sub(a, b) => {
            let (u1, v1) = rectform(a);
            let (u2, v2) = rectform(b);
            (smart_sub(u1, u2), smart_sub(v1, v2))
        }

        // Multiplication: (u1 + iv1)(u2 + iv2) = (u1*u2 - v1*v2) + i(u1*v2 + v1*u2)
        Expr::Mul(a, b) => {
            let (u1, v1) = rectform(a);
            let (u2, v2) = rectform(b);
            mul_pairs((u1, v1), (u2, v2))
        }

        // Division: (u1 + iv1)/(u2 + iv2) = ((u1*u2 + v1*v2) + i(v1*u2 - u1*v2))/(u2^2 + v2^2)
        Expr::Div(a, b) => {
            let (u1, v1) = rectform(a);
            let (u2, v2) = rectform(b);
            div_pairs((u1, v1), (u2, v2))
        }

        // Power: (u + iv)^p
        Expr::Pow(base, exp) => pow_pairs(base, exp),

        // Exponential: exp(u + iv) = exp(u) * (cos(v) + i*sin(v))
        Expr::Exp(a) => {
            let (u, v) = rectform(a);
            if v.is_zero() {
                return (smart_pow(Expr::e(), u), Expr::int(0));
            }
            let eu = if u.is_zero() {
                Expr::int(1)
            } else {
                Expr::exp(u)
            };
            let (cos_v, sin_v) = trig_angle(&v);
            let cos_term = cos_v.unwrap_or_else(|| Expr::cos(v.clone()));
            let sin_term = sin_v.unwrap_or_else(|| Expr::sin(v));
            (smart_mul(eu.clone(), cos_term), smart_mul(eu, sin_term))
        }

        // Logarithm: ln(u + iv) = 1/2 * ln(u^2 + v^2) + i * atan2(v, u)
        Expr::Ln(a) => {
            let (u, v) = rectform(a);
            if v.is_zero() {
                if let Expr::Const(r) = &u
                    && r.is_negative()
                {
                    // ln(-|r|) = ln(|r|) + i*pi
                    let abs_r = Expr::constant(-r);
                    let real_part = if abs_r.is_one() {
                        Expr::int(0)
                    } else {
                        Expr::ln(abs_r)
                    };
                    return (real_part, Expr::pi());
                }
                return (Expr::ln(u), Expr::int(0));
            }
            // Real part: 1/2 * ln(u^2 + v^2)
            let mag_sq = smart_add(
                smart_mul(u.clone(), u.clone()),
                smart_mul(v.clone(), v.clone()),
            );
            let real_part = if mag_sq.is_one() {
                Expr::int(0)
            } else {
                smart_mul(Expr::frac(1, 2), Expr::ln(mag_sq))
            };
            let imag_part = carg_pair(&u, &v);
            (real_part, imag_part)
        }

        // Square root: sqrt(a) = a^(1/2)
        Expr::Sqrt(a) => pow_pairs(a, &Expr::frac(1, 2)),

        // Trigonometric functions (rpart.lisp:407-423)
        Expr::Sin(a) => {
            let (u, v) = rectform(a);
            if v.is_zero() {
                return (smart_sin(u), Expr::int(0));
            }
            // sin(u + iv) = sin(u)*cosh(v) + i*cos(u)*sinh(v)
            (
                smart_mul(smart_sin(u.clone()), smart_cosh(v.clone())),
                smart_mul(smart_cos(u), smart_sinh(v)),
            )
        }

        Expr::Cos(a) => {
            let (u, v) = rectform(a);
            if v.is_zero() {
                return (smart_cos(u), Expr::int(0));
            }
            // cos(u + iv) = cos(u)*cosh(v) - i*sin(u)*sinh(v)
            (
                smart_mul(smart_cos(u.clone()), smart_cosh(v.clone())),
                smart_neg(smart_mul(smart_sin(u), smart_sinh(v))),
            )
        }

        Expr::Tan(a) => {
            let (u, v) = rectform(a);
            if v.is_zero() {
                return (Expr::tan(u), Expr::int(0));
            }
            // tan(u + iv) = (sin(2u) + i*sinh(2v)) / (cos(2u) + cosh(2v))
            let two_u = smart_mul(Expr::int(2), u);
            let two_v = smart_mul(Expr::int(2), v);
            let denom = smart_add(smart_cos(two_u.clone()), smart_cosh(two_v.clone()));
            (
                smart_div(smart_sin(two_u), denom.clone()),
                smart_div(smart_sinh(two_v), denom),
            )
        }

        // Hyperbolic functions (rpart.lisp:407-423)
        Expr::Sinh(a) => {
            let (u, v) = rectform(a);
            if v.is_zero() {
                return (smart_sinh(u), Expr::int(0));
            }
            // sinh(u + iv) = sinh(u)*cos(v) + i*cosh(u)*sin(v)
            (
                smart_mul(smart_sinh(u.clone()), smart_cos(v.clone())),
                smart_mul(smart_cosh(u), smart_sin(v)),
            )
        }

        Expr::Cosh(a) => {
            let (u, v) = rectform(a);
            if v.is_zero() {
                return (smart_cosh(u), Expr::int(0));
            }
            // cosh(u + iv) = cosh(u)*cos(v) + i*sinh(u)*sin(v)
            (
                smart_mul(smart_cosh(u.clone()), smart_cos(v.clone())),
                smart_mul(smart_sinh(u), smart_sin(v)),
            )
        }

        Expr::Tanh(a) => {
            let (u, v) = rectform(a);
            if v.is_zero() {
                return (Expr::tanh(u), Expr::int(0));
            }
            // tanh(u + iv) = (sinh(2u) + i*sin(2v)) / (cosh(2u) + cos(2v))
            let two_u = smart_mul(Expr::int(2), u);
            let two_v = smart_mul(Expr::int(2), v);
            let denom = smart_add(Expr::cosh(two_u.clone()), Expr::cos(two_v.clone()));
            (
                smart_div(Expr::sinh(two_u), denom.clone()),
                smart_div(Expr::sin(two_v), denom),
            )
        }

        // Fallthrough for higher calculus constructs or unexpanded inverse trigs
        Expr::Asin(_)
        | Expr::Acos(_)
        | Expr::Atan(_)
        | Expr::Factorial(_)
        | Expr::Derivative(_, _)
        | Expr::Integral(_, _)
        | Expr::Sum(_, _, _, _)
        | Expr::Matrix(_) => (expr.clone(), Expr::int(0)),
        #[cfg(feature = "specfn")]
        Expr::SpecialFn(_, _) => (expr.clone(), Expr::int(0)),
    }
}

/// Helper: multiply two complex pairs $(u_1 + iv_1)(u_2 + iv_2)$.
fn mul_pairs(p1: (Expr, Expr), p2: (Expr, Expr)) -> (Expr, Expr) {
    let (u1, v1) = p1;
    let (u2, v2) = p2;
    if v1.is_zero() && v2.is_zero() {
        return (smart_mul(u1, u2), Expr::int(0));
    }
    if v1.is_zero() {
        return (smart_mul(u1.clone(), u2), smart_mul(u1, v2));
    }
    if v2.is_zero() {
        return (smart_mul(u1, u2.clone()), smart_mul(v1, u2));
    }
    if u1.is_zero() && u2.is_zero() {
        // (i*v1)(i*v2) = -v1*v2
        return (smart_neg(smart_mul(v1, v2)), Expr::int(0));
    }
    let real = smart_sub(
        smart_mul(u1.clone(), u2.clone()),
        smart_mul(v1.clone(), v2.clone()),
    );
    let imag = smart_add(smart_mul(u1, v2), smart_mul(v1, u2));
    (real, imag)
}

/// Helper: divide two complex pairs $(u_1 + iv_1)/(u_2 + iv_2)$.
fn div_pairs(p1: (Expr, Expr), p2: (Expr, Expr)) -> (Expr, Expr) {
    let (u1, v1) = p1;
    let (u2, v2) = p2;
    if v2.is_zero() {
        return (smart_div(u1, u2.clone()), smart_div(v1, u2));
    }
    let denom = smart_add(
        smart_mul(u2.clone(), u2.clone()),
        smart_mul(v2.clone(), v2.clone()),
    );
    let num_real = smart_add(
        smart_mul(u1.clone(), u2.clone()),
        smart_mul(v1.clone(), v2.clone()),
    );
    let num_imag = smart_sub(smart_mul(v1, u2), smart_mul(u1, v2));
    (
        smart_div(num_real, denom.clone()),
        smart_div(num_imag, denom),
    )
}

/// Helper: compute powers $(u + iv)^p$.
fn pow_pairs(base: &Expr, exp: &Expr) -> (Expr, Expr) {
    // 1. Cyclic powers of the imaginary unit i^k
    if *base == Expr::I
        && let Expr::Const(r) = exp
        && Rat::is_integer(r)
        && let Some(k) = r.numer().to_i64()
    {
        let m = k.rem_euclid(4);
        return match m {
            0 => (Expr::int(1), Expr::int(0)),
            1 => (Expr::int(0), Expr::int(1)),
            2 => (Expr::int(-1), Expr::int(0)),
            3 => (Expr::int(0), Expr::int(-1)),
            _ => (Expr::int(1), Expr::int(0)),
        };
    }

    let (u, v) = rectform(base);

    // 2. Base is real
    if v.is_zero() {
        if let Expr::Const(r) = &u
            && r.is_negative()
        {
            // Negative real base: (-|r|)^p = |r|^p * exp(i*pi*p)
            if let Expr::Const(re) = exp {
                let abs_r = Expr::constant(-r);
                let mag = smart_pow(abs_r, exp.clone());
                let angle = smart_mul(Expr::pi(), Expr::constant(re.clone()));
                let (cos_a, sin_a) = trig_angle(&angle);
                let c = cos_a.unwrap_or_else(|| Expr::cos(angle.clone()));
                let s = sin_a.unwrap_or_else(|| Expr::sin(angle));
                return (smart_mul(mag.clone(), c), smart_mul(mag, s));
            }
        }
        // If exp is real
        let (eu, ev) = rectform(exp);
        if ev.is_zero() {
            return (smart_pow(u, eu), Expr::int(0));
        }
    }

    // 3. Integer powers via repeated multiplication / binary exponentiation
    if let Expr::Const(re) = exp
        && Rat::is_integer(re)
        && let Some(n) = re.numer().to_i64()
    {
        if (0..=64).contains(&n) {
            let mut cur = (Expr::int(1), Expr::int(0));
            let mut b = (u, v);
            let mut p = n;
            while p > 0 {
                if p % 2 == 1 {
                    cur = mul_pairs(cur, b.clone());
                }
                b = mul_pairs(b.clone(), b);
                p /= 2;
            }
            return cur;
        } else if (-64..0).contains(&n) {
            let pos_pow = pow_pairs(base, &Expr::int(-n));
            return div_pairs((Expr::int(1), Expr::int(0)), pos_pow);
        }
    }

    // 4. General power: z^w = exp(w * ln(z))
    let ln_base = Expr::ln(base.clone());
    let w_ln_z = smart_mul(exp.clone(), ln_base);
    rectform(&Expr::exp(w_ln_z))
}

/// Helper: compute $\operatorname{carg}(u + iv) = \operatorname{atan2}(v, u)$.
fn carg_pair(u: &Expr, v: &Expr) -> Expr {
    if v.is_zero() {
        if let Expr::Const(r) = u
            && r.is_negative()
        {
            return Expr::pi();
        }
        return Expr::int(0);
    }
    if u.is_zero() {
        if let Expr::Const(r) = v
            && r.is_negative()
        {
            return smart_neg(Expr::frac(1, 2) * Expr::pi());
        }
        return Expr::frac(1, 2) * Expr::pi();
    }
    Expr::atan(smart_div(v.clone(), u.clone()))
}

/// Construct a single [`Expr`] in rectangular form $u + i \cdot v$.
///
/// Implements Maxima 5.50 `$rectform` (`rpart.lisp:85`).
pub fn rectform_expr(expr: &Expr) -> Expr {
    let (u, v) = rectform(expr);
    if v.is_zero() {
        return u;
    }
    if u.is_zero() {
        if v.is_one() {
            return Expr::i();
        }
        if v == Expr::int(-1) {
            return smart_neg(Expr::i());
        }
        if let Expr::Neg(inner) = v {
            return smart_neg(smart_mul(Expr::i(), *inner));
        }
        return smart_mul(v, Expr::i());
    }
    if v.is_one() {
        return smart_add(u, Expr::i());
    }
    if v == Expr::int(-1) {
        return smart_sub(u, Expr::i());
    }
    if let Expr::Neg(inner) = v {
        return smart_sub(u, smart_mul(*inner, Expr::i()));
    }
    smart_add(u, smart_mul(v, Expr::i()))
}

/// Extract the real part $\operatorname{Re}(expr)$.
///
/// Implements Maxima 5.50 `$realpart` (`rpart.lisp:39`).
pub fn realpart(expr: &Expr) -> Expr {
    rectform(expr).0
}

/// Extract the imaginary part $\operatorname{Im}(expr)$.
///
/// Implements Maxima 5.50 `$imagpart` (`rpart.lisp:62`).
pub fn imagpart(expr: &Expr) -> Expr {
    rectform(expr).1
}

/// Compute the complex conjugate $expr^* = u - i \cdot v$.
pub fn conjugate(expr: &Expr) -> Expr {
    let (u, v) = rectform(expr);
    if v.is_zero() {
        return u;
    }
    rectform_expr(&smart_sub(u, smart_mul(Expr::i(), v)))
}

/// Compute the complex argument (phase angle) $\operatorname{carg}(expr) \in (-\pi, \pi]$.
///
/// Implements Maxima 5.50 `absarg` (`rpart.lisp:669`).
pub fn carg(expr: &Expr) -> Expr {
    let (u, v) = rectform(expr);
    carg_pair(&u, &v)
}

/// Compute the complex absolute value $|expr| = \sqrt{u^2 + v^2}$.
///
/// Implements Maxima 5.50 `cabs` (`rpart.lisp:103`).
pub fn cabs(expr: &Expr) -> Expr {
    let (u, v) = rectform(expr);
    if v.is_zero() {
        if let Expr::Const(r) = &u
            && r.is_negative()
        {
            return Expr::constant(-r);
        }
        return u;
    }
    if u.is_zero() {
        if let Expr::Const(r) = &v
            && r.is_negative()
        {
            return Expr::constant(-r);
        }
        return v;
    }
    smart_sqrt(smart_add(smart_mul(u.clone(), u), smart_mul(v.clone(), v)))
}

/// Decompose an expression into polar coordinates $(r, \theta)$ where $r = |expr|$ and $\theta = \operatorname{carg}(expr)$.
pub fn polarform(expr: &Expr) -> (Expr, Expr) {
    (cabs(expr), carg(expr))
}

/// Construct a single [`Expr`] in polar form $r \cdot e^{i \theta}$.
///
/// Implements Maxima 5.50 `$polarform` (`rpart.lisp:91`).
pub fn polarform_expr(expr: &Expr) -> Expr {
    let (r, theta) = polarform(expr);
    if theta.is_zero() {
        return r;
    }
    let exp_term = Expr::exp(smart_mul(Expr::i(), theta));
    if r.is_one() {
        exp_term
    } else {
        smart_mul(r, exp_term)
    }
}

/// Compute the Laurent residue $\operatorname{Res}(expr, var = point)$.
///
/// The residue of a meromorphic function at a pole $z_0$ is the coefficient of
/// $(z - z_0)^{-1}$ in its Laurent series expansion.
/// Uses [`crate::series::series_expand`] to compute the Laurent series up to order 6.
pub fn residue(expr: &Expr, var: &str, point: &Expr) -> Result<Expr, String> {
    if let Some(s) = crate::series::series_expand(expr, var, point, 6) {
        Ok(s.coeff_at(-1))
    } else {
        Err(format!(
            "Could not compute Laurent series for residue of {expr} at {var} = {point}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_i_powers() {
        assert_eq!(
            rectform_expr(&Expr::pow(Expr::i(), Expr::int(0))),
            Expr::int(1)
        );
        assert_eq!(
            rectform_expr(&Expr::pow(Expr::i(), Expr::int(1))),
            Expr::i()
        );
        assert_eq!(
            rectform_expr(&Expr::pow(Expr::i(), Expr::int(2))),
            Expr::int(-1)
        );
        assert_eq!(
            rectform_expr(&Expr::pow(Expr::i(), Expr::int(3))),
            Expr::neg(Expr::i())
        );
        assert_eq!(
            rectform_expr(&Expr::pow(Expr::i(), Expr::int(4))),
            Expr::int(1)
        );
        assert_eq!(
            rectform_expr(&Expr::pow(Expr::i(), Expr::int(-1))),
            Expr::neg(Expr::i())
        );
        assert_eq!(
            rectform_expr(&Expr::pow(Expr::i(), Expr::int(-2))),
            Expr::int(-1)
        );
    }

    #[test]
    fn test_euler_identity() {
        // exp(i * pi) = -1
        let e_ipi = Expr::exp(Expr::mul(Expr::i(), Expr::pi()));
        assert_eq!(rectform_expr(&e_ipi), Expr::int(-1));

        // exp(i * pi / 2) = i
        let e_ipi2 = Expr::exp(Expr::mul(Expr::i(), Expr::div(Expr::pi(), Expr::int(2))));
        assert_eq!(rectform_expr(&e_ipi2), Expr::i());
    }

    #[test]
    fn test_complex_log() {
        // ln(-1) = i * pi
        let ln_neg1 = Expr::ln(Expr::int(-1));
        let (u, v) = rectform(&ln_neg1);
        assert_eq!(u, Expr::int(0));
        assert_eq!(v, Expr::pi());

        // ln(i) = i * pi / 2
        let ln_i = Expr::ln(Expr::i());
        let (u, v) = rectform(&ln_i);
        assert_eq!(u, Expr::int(0));
        assert_eq!(v, Expr::frac(1, 2) * Expr::pi());
    }

    #[test]
    fn test_complex_arithmetic() {
        // (2 + 3i) + (4 - i) = 6 + 2i
        let z1 = Expr::add(Expr::int(2), Expr::mul(Expr::int(3), Expr::i()));
        let z2 = Expr::sub(Expr::int(4), Expr::i());
        let sum = Expr::add(z1.clone(), z2.clone());
        let (u, v) = rectform(&sum);
        assert_eq!(u, Expr::int(6));
        assert_eq!(v, Expr::int(2));

        // (2 + 3i)*(2 - 3i) = 4 + 9 = 13
        let z_conj = Expr::sub(Expr::int(2), Expr::mul(Expr::int(3), Expr::i()));
        let prod = Expr::mul(z1, z_conj);
        let (u, v) = rectform(&prod);
        assert_eq!(u, Expr::int(13));
        assert_eq!(v, Expr::int(0));
    }

    #[test]
    fn test_cabs_and_carg() {
        // |3 + 4i| = 5
        let z = Expr::add(Expr::int(3), Expr::mul(Expr::int(4), Expr::i()));
        assert_eq!(cabs(&z), Expr::int(5));

        // carg(-5) = pi
        assert_eq!(carg(&Expr::int(-5)), Expr::pi());

        // carg(5i) = pi / 2
        assert_eq!(
            carg(&Expr::mul(Expr::int(5), Expr::i())),
            Expr::frac(1, 2) * Expr::pi()
        );
    }

    #[test]
    fn test_residue() {
        let z = Expr::var("z");

        // Res(1/z, z=0) = 1
        let f1 = Expr::div(Expr::int(1), z.clone());
        let res1 = residue(&f1, "z", &Expr::int(0)).unwrap();
        assert_eq!(res1, Expr::int(1));

        // Res(1/z^2, z=0) = 0
        let f2 = Expr::div(Expr::int(1), Expr::pow(z.clone(), Expr::int(2)));
        let res2 = residue(&f2, "z", &Expr::int(0)).unwrap();
        assert_eq!(res2, Expr::int(0));

        // Res(exp(z)/z^3, z=0) = 1/2
        let f3 = Expr::div(Expr::exp(z.clone()), Expr::pow(z.clone(), Expr::int(3)));
        let res3 = residue(&f3, "z", &Expr::int(0)).unwrap();
        assert_eq!(res3, Expr::frac(1, 2));
    }
}
