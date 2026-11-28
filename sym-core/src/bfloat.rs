//! Pure-Rust arbitrary-precision floating-point numbers (`bfloat`) (Phase 39).
//!
//! Mirrors Maxima's `float.lisp` `bfloat` representation and `fpprec` precision control,
//! backed by pure-Rust [`dashu_float::DBig`].

use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use dashu_float::round::mode::HalfAway;
use dashu_float::{Context, DBig};
use num_rational::BigRational;

use crate::expr::Expr;

/// Default decimal precision matching Maxima's `$fpprec = 16`.
pub const DEFAULT_FPPREC: usize = 16;

/// Concrete decimal context type alias.
pub type DecContext = Context<HalfAway>;

/// Precision context for bigfloat operations, tracking decimal precision `fpprec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BfloatContext {
    pub fpprec: usize,
}

impl Default for BfloatContext {
    fn default() -> Self {
        Self {
            fpprec: DEFAULT_FPPREC,
        }
    }
}

impl BfloatContext {
    pub fn new(fpprec: usize) -> Self {
        Self {
            fpprec: fpprec.max(1),
        }
    }

    pub fn dashu_context(&self) -> DecContext {
        DecContext::new(self.fpprec.max(1))
    }

    pub fn pi(&self) -> DBig {
        self.dashu_context().pi::<10>().value()
    }

    pub fn e(&self) -> DBig {
        let ctx = self.dashu_context();
        ctx.exp(DBig::ONE.repr()).value()
    }
}

/// An arbitrary-precision floating-point number, wrapping [`dashu_float::DBig`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BfloatWrapper {
    pub value: DBig,
    pub prec: usize,
}

impl Hash for BfloatWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.prec.hash(state);
        self.to_maxima_string().hash(state);
    }
}

impl BfloatWrapper {
    /// Create a new `BfloatWrapper` rounding to `prec` decimal digits (clamped to `>= 1`).
    pub fn new(value: DBig, prec: usize) -> Self {
        let prec = prec.max(1);
        let ctx = DecContext::new(prec);
        let rounded = ctx.add(value.repr(), DBig::ZERO.repr()).value();
        Self {
            value: rounded,
            prec,
        }
    }

    /// Construct from an `f64`.
    pub fn from_f64(val: f64, prec: usize) -> Result<Self, String> {
        if !val.is_finite() {
            return Err(format!(
                "non-finite f64 cannot be converted to bfloat: {val}"
            ));
        }
        let s = val.to_string();
        Self::parse(&s, prec)
    }

    /// Construct from an exact rational constant `BigRational`.
    pub fn from_rational(r: &BigRational, prec: usize) -> Result<Self, String> {
        let prec = prec.max(1);
        let num_str = r.numer().to_string();
        let den_str = r.denom().to_string();
        let num = DBig::from_str(&num_str)
            .map_err(|e| format!("invalid rational numerator '{num_str}': {e}"))?;
        let den = DBig::from_str(&den_str)
            .map_err(|e| format!("invalid rational denominator '{den_str}': {e}"))?;
        if den == DBig::ZERO {
            return Err("division by zero in rational to bfloat conversion".into());
        }
        let ctx = DecContext::new(prec);
        let val = ctx.div(num.repr(), den.repr()).value();
        Ok(Self { value: val, prec })
    }

    /// Parse a bigfloat from string (supporting Maxima's `b` / `B` exponent notation).
    pub fn parse(s: &str, prec: usize) -> Result<Self, String> {
        let prec = prec.max(1);
        let cleaned = s.trim().replace(['b', 'B'], "e");
        let d =
            DBig::from_str(&cleaned).map_err(|e| format!("invalid bfloat string '{s}': {e}"))?;
        let ctx = DecContext::new(prec);
        let rounded = ctx.add(d.repr(), DBig::ZERO.repr()).value();
        Ok(Self {
            value: rounded,
            prec,
        })
    }

    /// Convert to machine `f64`.
    pub fn to_f64(&self) -> f64 {
        self.value.to_f64().value()
    }

    /// Check if zero.
    pub fn is_zero(&self) -> bool {
        self.value == DBig::ZERO
    }

    /// Check if one.
    pub fn is_one(&self) -> bool {
        self.value == DBig::ONE
    }

    /// Format using canonical Maxima notation with `b` exponent indicator (e.g. `1.2345b0`, `2.5b-3`).
    pub fn to_maxima_string(&self) -> String {
        let s = format!("{:e}", self.value);
        s.replace('e', "b")
    }

    /// Format in LaTeX.
    pub fn to_latex(&self) -> String {
        let s = format!("{:e}", self.value);
        if let Some((mant, exp)) = s.split_once('e') {
            if exp == "0" {
                mant.to_string()
            } else {
                format!("{mant} \\times 10^{{{exp}}}")
            }
        } else {
            s
        }
    }

    // ── Arithmetic Operations ──────────────────────────────────────────

    pub fn add(&self, other: &Self) -> Self {
        let prec = self.prec.max(other.prec);
        let ctx = DecContext::new(prec);
        let res = ctx.add(self.value.repr(), other.value.repr()).value();
        Self { value: res, prec }
    }

    pub fn sub(&self, other: &Self) -> Self {
        let prec = self.prec.max(other.prec);
        let ctx = DecContext::new(prec);
        let res = ctx.sub(self.value.repr(), other.value.repr()).value();
        Self { value: res, prec }
    }

    pub fn mul(&self, other: &Self) -> Self {
        let prec = self.prec.max(other.prec);
        let ctx = DecContext::new(prec);
        let res = ctx.mul(self.value.repr(), other.value.repr()).value();
        Self { value: res, prec }
    }

    pub fn div(&self, other: &Self) -> Result<Self, String> {
        if other.is_zero() {
            return Err("division by zero in bfloat".into());
        }
        let prec = self.prec.max(other.prec);
        let ctx = DecContext::new(prec);
        let res = ctx.div(self.value.repr(), other.value.repr()).value();
        Ok(Self { value: res, prec })
    }

    pub fn neg(&self) -> Self {
        let ctx = DecContext::new(self.prec);
        let res = ctx.sub(DBig::ZERO.repr(), self.value.repr()).value();
        Self {
            value: res,
            prec: self.prec,
        }
    }

    pub fn pow(&self, other: &Self) -> Result<Self, String> {
        if self.value == DBig::ZERO && other.value <= DBig::ZERO {
            return Err("0^p is undefined for non-positive exponent p".into());
        }
        if self.value < DBig::ZERO && other.value.fract() != DBig::ZERO {
            return Err("negative base with non-integer exponent in bfloat".into());
        }
        let prec = self.prec.max(other.prec).max(1);
        let ctx = DecContext::new(prec);
        let res = ctx.powf(self.value.repr(), other.value.repr()).value();
        Ok(Self { value: res, prec })
    }

    pub fn sqrt(&self) -> Result<Self, String> {
        if self.value < DBig::ZERO {
            return Err("square root of negative bfloat".into());
        }
        let ctx = DecContext::new(self.prec);
        let res = ctx.sqrt(self.value.repr()).value();
        Ok(Self {
            value: res,
            prec: self.prec,
        })
    }

    pub fn exp(&self) -> Self {
        let ctx = DecContext::new(self.prec);
        let res = ctx.exp(self.value.repr()).value();
        Self {
            value: res,
            prec: self.prec,
        }
    }

    pub fn ln(&self) -> Result<Self, String> {
        if self.value <= DBig::ZERO {
            return Err("logarithm of non-positive bfloat".into());
        }
        let ctx = DecContext::new(self.prec);
        let res = ctx.ln(self.value.repr()).value();
        Ok(Self {
            value: res,
            prec: self.prec,
        })
    }

    pub fn sin(&self) -> Result<Self, String> {
        let ctx = DecContext::new(self.prec);
        match ctx.sin(self.value.repr()).ok(&ctx) {
            Some(r) => Ok(Self {
                value: r.value(),
                prec: self.prec,
            }),
            None => Err(format!(
                "sin({}) precision loss or domain error",
                self.to_maxima_string()
            )),
        }
    }

    pub fn cos(&self) -> Result<Self, String> {
        let ctx = DecContext::new(self.prec);
        match ctx.cos(self.value.repr()).ok(&ctx) {
            Some(r) => Ok(Self {
                value: r.value(),
                prec: self.prec,
            }),
            None => Err(format!(
                "cos({}) precision loss or domain error",
                self.to_maxima_string()
            )),
        }
    }

    pub fn tan(&self) -> Result<Self, String> {
        let ctx = DecContext::new(self.prec);
        match ctx.tan(self.value.repr()).ok(&ctx) {
            Some(r) => Ok(Self {
                value: r.value(),
                prec: self.prec,
            }),
            None => Err(format!(
                "tan({}) precision loss or domain error",
                self.to_maxima_string()
            )),
        }
    }

    pub fn sinh(&self) -> Self {
        if self.is_zero() {
            return self.clone();
        }
        // Use extended working precision to avoid catastrophic cancellation near 0 (e.g. 1b-10)
        let work_prec = self.prec.saturating_mul(2).saturating_add(32);
        let work_ctx = DecContext::new(work_prec);
        let ep = work_ctx.exp(self.value.repr()).value();
        let neg_val = work_ctx.sub(DBig::ZERO.repr(), self.value.repr()).value();
        let em = work_ctx.exp(neg_val.repr()).value();
        let diff = work_ctx.sub(ep.repr(), em.repr()).value();
        let two = DBig::from(2u8);
        let raw = work_ctx.div(diff.repr(), two.repr()).value();
        Self::new(raw, self.prec)
    }

    pub fn cosh(&self) -> Self {
        let work_prec = self.prec.saturating_mul(2).saturating_add(16);
        let work_ctx = DecContext::new(work_prec);
        let ep = work_ctx.exp(self.value.repr()).value();
        let neg_val = work_ctx.sub(DBig::ZERO.repr(), self.value.repr()).value();
        let em = work_ctx.exp(neg_val.repr()).value();
        let sum = work_ctx.add(ep.repr(), em.repr()).value();
        let two = DBig::from(2u8);
        let raw = work_ctx.div(sum.repr(), two.repr()).value();
        Self::new(raw, self.prec)
    }

    pub fn tanh(&self) -> Self {
        if self.is_zero() {
            return self.clone();
        }
        let work_prec = self.prec.saturating_mul(2).saturating_add(32);
        let work_ctx = DecContext::new(work_prec);
        let ep = work_ctx.exp(self.value.repr()).value();
        let neg_val = work_ctx.sub(DBig::ZERO.repr(), self.value.repr()).value();
        let em = work_ctx.exp(neg_val.repr()).value();
        let diff = work_ctx.sub(ep.repr(), em.repr()).value();
        let sum = work_ctx.add(ep.repr(), em.repr()).value();
        let raw = work_ctx.div(diff.repr(), sum.repr()).value();
        Self::new(raw, self.prec)
    }

    pub fn asin(&self) -> Result<Self, String> {
        let ctx = DecContext::new(self.prec);
        match ctx.asin(self.value.repr()).ok(&ctx) {
            Some(r) => Ok(Self {
                value: r.value(),
                prec: self.prec,
            }),
            None => Err(format!("domain error in asin({})", self.to_maxima_string())),
        }
    }

    pub fn acos(&self) -> Result<Self, String> {
        let ctx = DecContext::new(self.prec);
        match ctx.acos(self.value.repr()).ok(&ctx) {
            Some(r) => Ok(Self {
                value: r.value(),
                prec: self.prec,
            }),
            None => Err(format!("domain error in acos({})", self.to_maxima_string())),
        }
    }

    pub fn atan(&self) -> Result<Self, String> {
        let ctx = DecContext::new(self.prec);
        match ctx.atan(self.value.repr()).ok(&ctx) {
            Some(r) => Ok(Self {
                value: r.value(),
                prec: self.prec,
            }),
            None => Err(format!(
                "atan({}) precision loss or domain error",
                self.to_maxima_string()
            )),
        }
    }
}

impl fmt::Display for BfloatWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_maxima_string())
    }
}

impl fmt::Debug for BfloatWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bfloat({}, prec={})", self.to_maxima_string(), self.prec)
    }
}

// ── High-Level Conversion Function (bfloat) ─────────────────────────

/// Convert an expression to a Bigfloat with precision `prec` (defaulting to [`DEFAULT_FPPREC`]).
///
/// Mirrors Maxima's `$bfloat` in `float.lisp`. Numeric constants, rationals, and algebraic
/// operations between constants are evaluated numerically into bigfloats.
pub fn bfloat(expr: &Expr, prec: Option<usize>) -> Expr {
    let p = prec.unwrap_or(DEFAULT_FPPREC);
    bfloat_eval(expr, p)
}

/// Parse a string directly into an `Expr::Bfloat` with given precision.
pub fn bfloat_from_str(s: &str, prec: Option<usize>) -> Result<Expr, String> {
    let p = prec.unwrap_or(DEFAULT_FPPREC);
    let bw = BfloatWrapper::parse(s, p)?;
    Ok(Expr::Bfloat(bw))
}

fn bfloat_eval(expr: &Expr, p: usize) -> Expr {
    match expr {
        Expr::Const(r) => match BfloatWrapper::from_rational(r, p) {
            Ok(bw) => Expr::Bfloat(bw),
            Err(_) => expr.clone(),
        },
        Expr::Bfloat(bw) => {
            if bw.prec == p {
                Expr::Bfloat(bw.clone())
            } else {
                Expr::Bfloat(BfloatWrapper::new(bw.value.clone(), p))
            }
        }
        Expr::Pi => {
            let ctx = DecContext::new(p);
            Expr::Bfloat(BfloatWrapper {
                value: ctx.pi::<10>().value(),
                prec: p,
            })
        }
        Expr::E => {
            let ctx = DecContext::new(p);
            Expr::Bfloat(BfloatWrapper {
                value: ctx.exp(DBig::ONE.repr()).value(),
                prec: p,
            })
        }
        Expr::Var(_) | Expr::Tensor(_, _) => expr.clone(),
        #[cfg(feature = "complex")]
        Expr::I => expr.clone(),
        Expr::Neg(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                Expr::Bfloat(ba.neg())
            } else {
                Expr::Neg(Box::new(ea))
            }
        }
        Expr::Add(a, b) => {
            let ea = bfloat_eval(a, p);
            let eb = bfloat_eval(b, p);
            if let (Expr::Bfloat(ba), Expr::Bfloat(bb)) = (&ea, &eb) {
                Expr::Bfloat(ba.add(bb))
            } else {
                Expr::Add(Box::new(ea), Box::new(eb))
            }
        }
        Expr::Sub(a, b) => {
            let ea = bfloat_eval(a, p);
            let eb = bfloat_eval(b, p);
            if let (Expr::Bfloat(ba), Expr::Bfloat(bb)) = (&ea, &eb) {
                Expr::Bfloat(ba.sub(bb))
            } else {
                Expr::Sub(Box::new(ea), Box::new(eb))
            }
        }
        Expr::Mul(a, b) => {
            let ea = bfloat_eval(a, p);
            let eb = bfloat_eval(b, p);
            if let (Expr::Bfloat(ba), Expr::Bfloat(bb)) = (&ea, &eb) {
                Expr::Bfloat(ba.mul(bb))
            } else {
                Expr::Mul(Box::new(ea), Box::new(eb))
            }
        }
        Expr::Div(a, b) => {
            let ea = bfloat_eval(a, p);
            let eb = bfloat_eval(b, p);
            if let (Expr::Bfloat(ba), Expr::Bfloat(bb)) = (&ea, &eb) {
                if let Ok(res) = ba.div(bb) {
                    Expr::Bfloat(res)
                } else {
                    Expr::Div(Box::new(ea), Box::new(eb))
                }
            } else {
                Expr::Div(Box::new(ea), Box::new(eb))
            }
        }
        Expr::Pow(a, b) => {
            let ea = bfloat_eval(a, p);
            let eb = bfloat_eval(b, p);
            if let (Expr::Bfloat(ba), Expr::Bfloat(bb)) = (&ea, &eb) {
                if let Ok(res) = ba.pow(bb) {
                    Expr::Bfloat(res)
                } else {
                    Expr::Pow(Box::new(ea), Box::new(eb))
                }
            } else {
                Expr::Pow(Box::new(ea), Box::new(eb))
            }
        }
        Expr::Sin(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                if let Ok(res) = ba.sin() {
                    Expr::Bfloat(res)
                } else {
                    Expr::Sin(Box::new(Expr::Bfloat(ba)))
                }
            } else {
                Expr::Sin(Box::new(ea))
            }
        }
        Expr::Cos(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                if let Ok(res) = ba.cos() {
                    Expr::Bfloat(res)
                } else {
                    Expr::Cos(Box::new(Expr::Bfloat(ba)))
                }
            } else {
                Expr::Cos(Box::new(ea))
            }
        }
        Expr::Tan(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                if let Ok(res) = ba.tan() {
                    Expr::Bfloat(res)
                } else {
                    Expr::Tan(Box::new(Expr::Bfloat(ba)))
                }
            } else {
                Expr::Tan(Box::new(ea))
            }
        }
        Expr::Exp(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                Expr::Bfloat(ba.exp())
            } else {
                Expr::Exp(Box::new(ea))
            }
        }
        Expr::Ln(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                if let Ok(res) = ba.ln() {
                    Expr::Bfloat(res)
                } else {
                    Expr::Ln(Box::new(Expr::Bfloat(ba)))
                }
            } else {
                Expr::Ln(Box::new(ea))
            }
        }
        Expr::Sqrt(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                if let Ok(res) = ba.sqrt() {
                    Expr::Bfloat(res)
                } else {
                    Expr::Sqrt(Box::new(Expr::Bfloat(ba)))
                }
            } else {
                Expr::Sqrt(Box::new(ea))
            }
        }
        Expr::Sinh(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                Expr::Bfloat(ba.sinh())
            } else {
                Expr::Sinh(Box::new(ea))
            }
        }
        Expr::Cosh(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                Expr::Bfloat(ba.cosh())
            } else {
                Expr::Cosh(Box::new(ea))
            }
        }
        Expr::Tanh(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                Expr::Bfloat(ba.tanh())
            } else {
                Expr::Tanh(Box::new(ea))
            }
        }
        Expr::Asin(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                if let Ok(res) = ba.asin() {
                    Expr::Bfloat(res)
                } else {
                    Expr::Asin(Box::new(Expr::Bfloat(ba)))
                }
            } else {
                Expr::Asin(Box::new(ea))
            }
        }
        Expr::Acos(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                if let Ok(res) = ba.acos() {
                    Expr::Bfloat(res)
                } else {
                    Expr::Acos(Box::new(Expr::Bfloat(ba)))
                }
            } else {
                Expr::Acos(Box::new(ea))
            }
        }
        Expr::Atan(a) => {
            let ea = bfloat_eval(a, p);
            if let Expr::Bfloat(ba) = ea {
                if let Ok(res) = ba.atan() {
                    Expr::Bfloat(res)
                } else {
                    Expr::Atan(Box::new(Expr::Bfloat(ba)))
                }
            } else {
                Expr::Atan(Box::new(ea))
            }
        }
        Expr::Factorial(a) => Expr::Factorial(Box::new(bfloat_eval(a, p))),
        Expr::Matrix(rows) => {
            let new_rows = rows
                .iter()
                .map(|r| r.iter().map(|c| bfloat_eval(c, p)).collect())
                .collect();
            Expr::Matrix(new_rows)
        }
        Expr::Derivative(body, var) => {
            Expr::Derivative(Box::new(bfloat_eval(body, p)), var.clone())
        }
        Expr::Integral(body, var) => Expr::Integral(Box::new(bfloat_eval(body, p)), var.clone()),
        Expr::Sum(body, var, low, high) => Expr::Sum(
            Box::new(bfloat_eval(body, p)),
            var.clone(),
            Box::new(bfloat_eval(low, p)),
            Box::new(bfloat_eval(high, p)),
        ),
        #[cfg(feature = "specfn")]
        Expr::SpecialFn(kind, args) => {
            let new_args = args.iter().map(|a| bfloat_eval(a, p)).collect();
            Expr::SpecialFn(*kind, new_args)
        }
    }
}
