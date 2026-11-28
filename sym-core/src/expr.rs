//! Symbolic Expression AST.
//!
//! The [`Expr`] enum is the foundational type of the entire maxima-rs pipeline.
//! Every module — differentiation, e-graph saturation, CSE, and codegen —
//! operates on or produces `Expr` trees.
//!
//! # Design Decisions
//!
//! - **Exact rational constants** ([`num_rational::BigRational`]): No `f64` values
//!   appear in the AST. Float literals are converted to rationals at parse time
//!   and only converted back to float literals during final codegen (ADR-0004).
//!
//! - **`Clone + Hash + Eq`**: Required for e-graph interning via `egg`.
//!
//! - **Box-based recursion**: Each recursive variant boxes its children.
//!   This keeps the enum size small and allows arbitrarily deep trees.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::fmt;

use crate::rational::Rat;
use crate::tensor::Index;

/// Canonical symbolic expression AST.
///
/// All numeric constants are exact rationals until final codegen.
/// The AST is immutable — transformations produce new trees.
///
/// # Examples
///
/// ```
/// use sym_core::expr::Expr;
/// use sym_core::rational::Rat;
///
/// // Build: x^2 + 2*x + 1
/// let x = Expr::var("x");
/// let expr = Expr::add(
///     Expr::add(
///         Expr::pow(x.clone(), Expr::int(2)),
///         Expr::mul(Expr::int(2), x.clone()),
///     ),
///     Expr::int(1),
/// );
/// assert_eq!(format!("{expr}"), "(((x ^ 2) + (2 * x)) + 1)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr {
    /// Named variable: `x`, `y`, `theta`, ...
    Var(String),

    /// Exact rational constant (converted to f64/f32 at codegen).
    Const(BigRational),

    /// Binary addition: `a + b`
    Add(Box<Expr>, Box<Expr>),

    /// Binary subtraction: `a - b`
    Sub(Box<Expr>, Box<Expr>),

    /// Binary multiplication: `a * b`
    Mul(Box<Expr>, Box<Expr>),

    /// Binary division: `a / b`
    Div(Box<Expr>, Box<Expr>),

    /// Exponentiation: `base ^ exponent`
    Pow(Box<Expr>, Box<Expr>),

    /// Unary negation: `-a`
    Neg(Box<Expr>),

    /// `sin(a)`
    Sin(Box<Expr>),

    /// `cos(a)`
    Cos(Box<Expr>),

    /// `tan(a)`
    Tan(Box<Expr>),

    /// `exp(a)` — Euler's number raised to `a`
    Exp(Box<Expr>),

    /// `ln(a)` — natural logarithm
    Ln(Box<Expr>),

    /// `sqrt(a)` — square root (sugar for `Pow(a, 1/2)`)
    Sqrt(Box<Expr>),

    /// `sinh(a)` — hyperbolic sine
    Sinh(Box<Expr>),

    /// `cosh(a)` — hyperbolic cosine
    Cosh(Box<Expr>),

    /// `tanh(a)` — hyperbolic tangent
    Tanh(Box<Expr>),

    /// `asin(a)` — inverse sine
    Asin(Box<Expr>),

    /// `acos(a)` — inverse cosine
    Acos(Box<Expr>),

    /// `atan(a)` — inverse tangent
    Atan(Box<Expr>),

    /// `abs(a)` — absolute value
    Abs(Box<Expr>),

    /// `atan2(y, x)` — two-argument arctangent
    Atan2(Box<Expr>, Box<Expr>),

    /// π — the circle constant (irrational, kept symbolic until evaluation)
    Pi,

    /// e — Euler's number (irrational, kept symbolic until evaluation)
    E,

    /// Imaginary unit: `i = \sqrt{-1}`
    #[cfg(feature = "complex")]
    I,

    /// Factorial: `n!`
    Factorial(Box<Expr>),

    /// A tensor with indices (Ricci calculus). E.g. $T^i_j$
    Tensor(String, Vec<Index>),

    /// Symbolic derivative: `d(expr)/d(var)`
    Derivative(Box<Expr>, Box<Expr>),

    /// Symbolic integral: `\int expr d(var)`
    Integral(Box<Expr>, Box<Expr>),

    /// Symbolic summation: `\sum_{var=lower}^{upper} expr`
    Sum(Box<Expr>, Box<Expr>, Box<Expr>, Box<Expr>),

    /// Matrix or Vector literal
    Matrix(Vec<Vec<Expr>>),

    /// Arbitrary-precision floating-point constant (`bfloat`).
    #[cfg(feature = "bfloat")]
    Bfloat(crate::bfloat::BfloatWrapper),

    /// Non-elementary special function: $\Gamma, \beta, \text{erf}$, Bessel, Airy, orthogonal polynomials.
    #[cfg(feature = "specfn")]
    SpecialFn(crate::specfn::SpecialFnKind, Vec<Expr>),
}

// ── Ergonomic Constructors ─────────────────────────────────────────

impl Expr {
    // ── Leaf constructors ──────────────────────────────────────────

    /// Create a named variable.
    ///
    /// ```
    /// # use sym_core::expr::Expr;
    /// let x = Expr::var("x");
    /// assert_eq!(format!("{x}"), "x");
    /// ```
    pub fn var(name: &str) -> Self {
        Expr::Var(name.to_string())
    }

    /// Create a tensor with indices.
    pub fn tensor(name: &str, indices: Vec<Index>) -> Self {
        Expr::Tensor(name.to_string(), indices)
    }

    /// Create an exact rational constant from a `BigRational`.
    pub fn constant(r: BigRational) -> Self {
        Expr::Const(r)
    }

    /// Create an integer constant.
    ///
    /// ```
    /// # use sym_core::expr::Expr;
    /// let two = Expr::int(2);
    /// assert_eq!(format!("{two}"), "2");
    /// ```
    pub fn int(n: i64) -> Self {
        Expr::Const(Rat::from_int(n))
    }

    /// Create a rational constant from numerator and denominator.
    ///
    /// ```
    /// # use sym_core::expr::Expr;
    /// let half = Expr::frac(1, 2);
    /// assert_eq!(format!("{half}"), "1/2");
    /// ```
    pub fn frac(num: i64, den: i64) -> Self {
        Expr::Const(Rat::from_pair(num, den))
    }

    /// Create a rational constant that best approximates the given `f64`.
    ///
    /// Uses continued fraction expansion for a clean rational approximation.
    ///
    /// ```
    /// # use sym_core::expr::Expr;
    /// let c = Expr::from_f64(1.5);
    /// assert_eq!(format!("{c}"), "3/2");
    /// ```
    pub fn from_f64(v: f64) -> Self {
        Expr::Const(Rat::from_f64(v))
    }

    /// Create an arbitrary-precision floating-point constant (`bfloat`).
    #[cfg(feature = "bfloat")]
    pub fn bfloat(bw: crate::bfloat::BfloatWrapper) -> Self {
        Expr::Bfloat(bw)
    }

    /// Create a bfloat from string with default precision.
    #[cfg(feature = "bfloat")]
    pub fn bfloat_str(s: &str) -> Result<Self, String> {
        let bw = crate::bfloat::BfloatWrapper::parse(s, crate::bfloat::DEFAULT_FPPREC)?;
        Ok(Expr::Bfloat(bw))
    }

    // ── Binary operator constructors ───────────────────────────────

    /// `a + b`
    #[allow(clippy::should_implement_trait)]
    pub fn add(a: Expr, b: Expr) -> Self {
        Expr::Add(Box::new(a), Box::new(b))
    }

    /// `a - b`
    #[allow(clippy::should_implement_trait)]
    pub fn sub(a: Expr, b: Expr) -> Self {
        Expr::Sub(Box::new(a), Box::new(b))
    }

    /// `a * b`
    #[allow(clippy::should_implement_trait)]
    pub fn mul(a: Expr, b: Expr) -> Self {
        Expr::Mul(Box::new(a), Box::new(b))
    }

    /// `a / b`
    #[allow(clippy::should_implement_trait)]
    pub fn div(a: Expr, b: Expr) -> Self {
        Expr::Div(Box::new(a), Box::new(b))
    }

    /// `base ^ exponent`
    pub fn pow(base: Expr, exp: Expr) -> Self {
        Expr::Pow(Box::new(base), Box::new(exp))
    }

    // ── Unary operator constructors ────────────────────────────────

    /// `-a`
    #[allow(clippy::should_implement_trait)]
    pub fn neg(a: Expr) -> Self {
        Expr::Neg(Box::new(a))
    }

    /// `sin(a)`
    pub fn sin(a: Expr) -> Self {
        Expr::Sin(Box::new(a))
    }

    /// `cos(a)`
    pub fn cos(a: Expr) -> Self {
        Expr::Cos(Box::new(a))
    }

    /// `tan(a)`
    pub fn tan(a: Expr) -> Self {
        Expr::Tan(Box::new(a))
    }

    /// `exp(a)` — e^a
    pub fn exp(a: Expr) -> Self {
        Expr::Exp(Box::new(a))
    }

    /// `ln(a)` — natural logarithm
    pub fn ln(a: Expr) -> Self {
        Expr::Ln(Box::new(a))
    }

    /// `sqrt(a)` — square root
    pub fn sqrt(a: Expr) -> Self {
        Expr::Sqrt(Box::new(a))
    }

    /// `sinh(a)` — hyperbolic sine
    pub fn sinh(a: Expr) -> Self {
        Expr::Sinh(Box::new(a))
    }

    /// `cosh(a)` — hyperbolic cosine
    pub fn cosh(a: Expr) -> Self {
        Expr::Cosh(Box::new(a))
    }

    /// `tanh(a)` — hyperbolic tangent
    pub fn tanh(a: Expr) -> Self {
        Expr::Tanh(Box::new(a))
    }

    /// `asin(a)` — inverse sine
    pub fn asin(a: Expr) -> Self {
        Expr::Asin(Box::new(a))
    }

    /// `acos(a)` — inverse cosine
    pub fn acos(a: Expr) -> Self {
        Expr::Acos(Box::new(a))
    }

    /// `atan(a)` — inverse tangent
    pub fn atan(a: Expr) -> Self {
        Expr::Atan(Box::new(a))
    }

    /// `abs(a)` — absolute value
    pub fn abs(a: Expr) -> Self {
        Expr::Abs(Box::new(a))
    }

    /// `atan2(y, x)` — two-argument arctangent
    pub fn atan2(y: Expr, x: Expr) -> Self {
        Expr::Atan2(Box::new(y), Box::new(x))
    }

    /// π constant
    pub fn pi() -> Self {
        Expr::Pi
    }

    /// Euler's number `e`
    pub fn e() -> Self {
        Expr::E
    }

    /// Imaginary unit `i`
    #[cfg(feature = "complex")]
    pub fn i() -> Self {
        Expr::I
    }

    /// Factorial `a!`
    pub fn factorial(a: Expr) -> Self {
        Expr::Factorial(Box::new(a))
    }

    /// `Derivative`
    pub fn derivative(expr: Expr, var: Expr) -> Self {
        Expr::Derivative(Box::new(expr), Box::new(var))
    }

    /// `Integral`
    pub fn integral(expr: Expr, var: Expr) -> Self {
        Expr::Integral(Box::new(expr), Box::new(var))
    }

    /// `Sum`
    pub fn sum(expr: Expr, var: Expr, lower: Expr, upper: Expr) -> Self {
        Expr::Sum(
            Box::new(expr),
            Box::new(var),
            Box::new(lower),
            Box::new(upper),
        )
    }

    /// `Matrix`
    pub fn matrix(mat: Vec<Vec<Expr>>) -> Self {
        Expr::Matrix(mat)
    }

    /// Non-elementary special function constructor.
    #[cfg(feature = "specfn")]
    pub fn special_fn(kind: crate::specfn::SpecialFnKind, args: Vec<Expr>) -> Self {
        Expr::SpecialFn(kind, args)
    }

    /// `gamma(z)` — Euler Gamma function
    #[cfg(feature = "specfn")]
    pub fn gamma(z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::Gamma, vec![z])
    }

    /// `beta(a, b)` — Euler Beta function
    #[cfg(feature = "specfn")]
    pub fn beta(a: Expr, b: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::Beta, vec![a, b])
    }

    /// `erf(z)` — Gauss Error function
    #[cfg(feature = "specfn")]
    pub fn erf(z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::Erf, vec![z])
    }

    /// `erfc(z)` — Complementary Error function
    #[cfg(feature = "specfn")]
    pub fn erfc(z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::Erfc, vec![z])
    }

    /// `bessel_j(n, z)` — Bessel function of the first kind
    #[cfg(feature = "specfn")]
    pub fn bessel_j(n: Expr, z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::BesselJ, vec![n, z])
    }

    /// `bessel_y(n, z)` — Bessel function of the second kind
    #[cfg(feature = "specfn")]
    pub fn bessel_y(n: Expr, z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::BesselY, vec![n, z])
    }

    /// `bessel_i(n, z)` — Modified Bessel function of the first kind
    #[cfg(feature = "specfn")]
    pub fn bessel_i(n: Expr, z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::BesselI, vec![n, z])
    }

    /// `bessel_k(n, z)` — Modified Bessel function of the second kind
    #[cfg(feature = "specfn")]
    pub fn bessel_k(n: Expr, z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::BesselK, vec![n, z])
    }

    /// `airy_ai(z)` — Airy function of the first kind
    #[cfg(feature = "specfn")]
    pub fn airy_ai(z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::AiryAi, vec![z])
    }

    /// `airy_bi(z)` — Airy function of the second kind
    #[cfg(feature = "specfn")]
    pub fn airy_bi(z: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::AiryBi, vec![z])
    }

    /// `legendre_p(n, x)` — Legendre polynomial
    #[cfg(feature = "specfn")]
    pub fn legendre_p(n: Expr, x: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::LegendreP, vec![n, x])
    }

    /// `chebyshev_t(n, x)` — Chebyshev polynomial of the first kind
    #[cfg(feature = "specfn")]
    pub fn chebyshev_t(n: Expr, x: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::ChebyshevT, vec![n, x])
    }

    /// `chebyshev_u(n, x)` — Chebyshev polynomial of the second kind
    #[cfg(feature = "specfn")]
    pub fn chebyshev_u(n: Expr, x: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::ChebyshevU, vec![n, x])
    }

    /// `hermite_h(n, x)` — Hermite polynomial
    #[cfg(feature = "specfn")]
    pub fn hermite_h(n: Expr, x: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::HermiteH, vec![n, x])
    }

    /// `laguerre_l(n, x)` — Laguerre polynomial
    #[cfg(feature = "specfn")]
    pub fn laguerre_l(n: Expr, x: Expr) -> Self {
        Expr::SpecialFn(crate::specfn::SpecialFnKind::LaguerreL, vec![n, x])
    }

    // ── Introspection ──────────────────────────────────────────────

    /// Returns `true` if this expression is the constant zero.
    pub fn is_zero(&self) -> bool {
        match self {
            Expr::Const(r) => r.is_zero(),
            #[cfg(feature = "bfloat")]
            Expr::Bfloat(b) => b.is_zero(),
            _ => false,
        }
    }

    /// Returns `true` if this expression is the constant one.
    pub fn is_one(&self) -> bool {
        match self {
            Expr::Const(r) => r.is_one(),
            #[cfg(feature = "bfloat")]
            Expr::Bfloat(b) => b.is_one(),
            _ => false,
        }
    }

    /// Returns `true` if this expression is a constant (any value).
    pub fn is_const(&self) -> bool {
        match self {
            Expr::Const(_) | Expr::Pi | Expr::E => true,
            #[cfg(feature = "bfloat")]
            Expr::Bfloat(_) => true,
            #[cfg(feature = "complex")]
            Expr::I => true,
            _ => false,
        }
    }

    /// Returns `true` if this expression is a variable.
    pub fn is_var(&self) -> bool {
        matches!(self, Expr::Var(_))
    }

    /// Collect all unique variable names in this expression.
    ///
    /// ```
    /// # use sym_core::expr::Expr;
    /// let expr = Expr::add(Expr::var("x"), Expr::mul(Expr::var("y"), Expr::var("x")));
    /// let mut vars = expr.variables();
    /// vars.sort();
    /// assert_eq!(vars, vec!["x", "y"]);
    /// ```
    pub fn variables(&self) -> Vec<String> {
        let mut vars = std::collections::HashSet::new();
        self.collect_vars(&mut vars);
        let mut sorted: Vec<String> = vars.into_iter().collect();
        sorted.sort();
        sorted
    }

    fn collect_vars(&self, vars: &mut std::collections::HashSet<String>) {
        match self {
            Expr::Var(name) => {
                vars.insert(name.clone());
            }
            Expr::Const(_) => {}
            #[cfg(feature = "bfloat")]
            Expr::Bfloat(_) => {}
            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b) => {
                a.collect_vars(vars);
                b.collect_vars(vars);
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
            | Expr::Factorial(a) => {
                a.collect_vars(vars);
            }
            Expr::Atan2(y, x) => {
                y.collect_vars(vars);
                x.collect_vars(vars);
            }
            Expr::Pi | Expr::E => {}
            #[cfg(feature = "complex")]
            Expr::I => {}
            Expr::Tensor(name, _indices) => {
                vars.insert(name.clone());
            }
            Expr::Derivative(a, b) | Expr::Integral(a, b) => {
                a.collect_vars(vars);
                b.collect_vars(vars);
            }
            Expr::Sum(a, b, lower, upper) => {
                a.collect_vars(vars);
                b.collect_vars(vars);
                lower.collect_vars(vars);
                upper.collect_vars(vars);
            }
            Expr::Matrix(mat) => {
                for row in mat {
                    for elem in row {
                        elem.collect_vars(vars);
                    }
                }
            }
            #[cfg(feature = "specfn")]
            Expr::SpecialFn(_, args) => {
                for arg in args {
                    arg.collect_vars(vars);
                }
            }
        }
    }

    /// Evaluate the expression numerically with the given variable bindings.
    ///
    /// Returns `None` if a variable is unbound or a numerical error occurs
    /// (e.g., division by zero, log of negative).
    ///
    /// ```
    /// # use sym_core::expr::Expr;
    /// # use std::collections::HashMap;
    /// let expr = Expr::add(Expr::var("x"), Expr::int(1));
    /// let mut env = HashMap::new();
    /// env.insert("x".to_string(), 2.0_f64);
    /// assert_eq!(expr.eval(&env), Some(3.0));
    /// ```
    pub fn eval(&self, env: &std::collections::HashMap<String, f64>) -> Option<f64> {
        match self {
            Expr::Var(name) => env.get(name).copied(),
            Expr::Const(r) => Some(Rat::to_f64(r)),
            #[cfg(feature = "bfloat")]
            Expr::Bfloat(b) => Some(b.to_f64()),
            Expr::Add(a, b) => Some(a.eval(env)? + b.eval(env)?),
            Expr::Sub(a, b) => Some(a.eval(env)? - b.eval(env)?),
            Expr::Mul(a, b) => Some(a.eval(env)? * b.eval(env)?),
            Expr::Div(a, b) => {
                let denom = b.eval(env)?;
                if denom == 0.0 {
                    None
                } else {
                    Some(a.eval(env)? / denom)
                }
            }
            Expr::Pow(base, exp) => {
                let b = base.eval(env)?;
                let e = exp.eval(env)?;
                if b == 0.0 && e <= 0.0 {
                    return None;
                }
                let res = b.powf(e);
                if res.is_finite() { Some(res) } else { None }
            }
            Expr::Neg(a) => Some(-a.eval(env)?),
            Expr::Sin(a) => Some(a.eval(env)?.sin()),
            Expr::Cos(a) => Some(a.eval(env)?.cos()),
            Expr::Tan(a) => Some(a.eval(env)?.tan()),
            Expr::Exp(a) => Some(a.eval(env)?.exp()),
            Expr::Ln(a) => {
                let v = a.eval(env)?;
                if v <= 0.0 { None } else { Some(v.ln()) }
            }
            Expr::Sqrt(a) => {
                let v = a.eval(env)?;
                if v < 0.0 { None } else { Some(v.sqrt()) }
            }
            Expr::Sinh(a) => Some(a.eval(env)?.sinh()),
            Expr::Cosh(a) => Some(a.eval(env)?.cosh()),
            Expr::Tanh(a) => Some(a.eval(env)?.tanh()),
            Expr::Asin(a) => {
                let v = a.eval(env)?;
                if (-1.0..=1.0).contains(&v) {
                    Some(v.asin())
                } else {
                    None
                }
            }
            Expr::Acos(a) => {
                let v = a.eval(env)?;
                if (-1.0..=1.0).contains(&v) {
                    Some(v.acos())
                } else {
                    None
                }
            }
            Expr::Atan(a) => Some(a.eval(env)?.atan()),
            Expr::Abs(a) => Some(a.eval(env)?.abs()),
            Expr::Atan2(y, x) => Some(y.eval(env)?.atan2(x.eval(env)?)),
            Expr::Pi => Some(std::f64::consts::PI),
            Expr::E => Some(std::f64::consts::E),
            #[cfg(feature = "complex")]
            Expr::I => None,
            Expr::Factorial(a) => {
                let v = a.eval(env)?;
                if !v.is_finite() || v < 0.0 || v.fract() != 0.0 || v > 170.0 {
                    None
                } else {
                    let n = v as u32;
                    let mut acc = 1.0_f64;
                    for k in 2..=n {
                        acc *= f64::from(k);
                    }
                    Some(acc)
                }
            }
            Expr::Tensor(_, _) => {
                // Tensor cannot be evaluated to a scalar natively right now
                None
            }
            Expr::Derivative(_, _)
            | Expr::Integral(_, _)
            | Expr::Sum(_, _, _, _)
            | Expr::Matrix(_) => {
                // Symbolic operators and matrices cannot be evaluated directly
                None
            }
            #[cfg(feature = "specfn")]
            Expr::SpecialFn(kind, args) => {
                if let Some(exact) = kind.eval_exact(args) {
                    exact.eval(env)
                } else {
                    None
                }
            }
        }
    }

    /// Count the total number of nodes in the expression tree.
    ///
    /// Useful for complexity metrics and testing.
    pub fn node_count(&self) -> usize {
        match self {
            Expr::Var(_) | Expr::Const(_) => 1,
            #[cfg(feature = "bfloat")]
            Expr::Bfloat(_) => 1,
            Expr::Add(a, b)
            | Expr::Sub(a, b)
            | Expr::Mul(a, b)
            | Expr::Div(a, b)
            | Expr::Pow(a, b) => 1 + a.node_count() + b.node_count(),
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
            | Expr::Factorial(a) => 1 + a.node_count(),
            Expr::Atan2(y, x) => 1 + y.node_count() + x.node_count(),
            Expr::Pi | Expr::E => 1,
            #[cfg(feature = "complex")]
            Expr::I => 1,
            Expr::Tensor(_, _) => 1,
            Expr::Derivative(a, b) | Expr::Integral(a, b) => 1 + a.node_count() + b.node_count(),
            Expr::Sum(a, b, c, d) => {
                1 + a.node_count() + b.node_count() + c.node_count() + d.node_count()
            }
            Expr::Matrix(mat) => {
                1 + mat
                    .iter()
                    .flat_map(|r| r.iter())
                    .map(|e| e.node_count())
                    .sum::<usize>()
            }
            #[cfg(feature = "specfn")]
            Expr::SpecialFn(_, args) => 1 + args.iter().map(|a| a.node_count()).sum::<usize>(),
        }
    }
}

// ── Display ────────────────────────────────────────────────────────

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Var(name) => write!(f, "{name}"),
            #[cfg(feature = "bfloat")]
            Expr::Bfloat(b) => write!(f, "{b}"),
            Expr::Const(r) => {
                if r.denom() == &BigInt::from(1) {
                    // Integer constant: display without denominator
                    if r.is_negative() {
                        write!(f, "({})", r.numer())
                    } else {
                        write!(f, "{}", r.numer())
                    }
                } else {
                    // Fraction: display as num/den
                    write!(f, "{}/{}", r.numer(), r.denom())
                }
            }
            Expr::Add(a, b) => write!(f, "({a} + {b})"),
            Expr::Sub(a, b) => write!(f, "({a} - {b})"),
            Expr::Mul(a, b) => write!(f, "({a} * {b})"),
            Expr::Div(a, b) => write!(f, "({a} / {b})"),
            Expr::Pow(a, b) => write!(f, "({a} ^ {b})"),
            Expr::Neg(a) => write!(f, "(-{a})"),
            Expr::Sin(a) => write!(f, "sin({a})"),
            Expr::Cos(a) => write!(f, "cos({a})"),
            Expr::Tan(a) => write!(f, "tan({a})"),
            Expr::Exp(a) => write!(f, "exp({a})"),
            Expr::Ln(a) => write!(f, "ln({a})"),
            Expr::Sqrt(a) => write!(f, "sqrt({a})"),
            Expr::Sinh(a) => write!(f, "sinh({a})"),
            Expr::Cosh(a) => write!(f, "cosh({a})"),
            Expr::Tanh(a) => write!(f, "tanh({a})"),
            Expr::Asin(a) => write!(f, "asin({a})"),
            Expr::Acos(a) => write!(f, "acos({a})"),
            Expr::Atan(a) => write!(f, "atan({a})"),
            Expr::Abs(a) => write!(f, "abs({a})"),
            Expr::Atan2(y, x) => write!(f, "atan2({y}, {x})"),
            Expr::Pi => write!(f, "%pi"),
            Expr::E => write!(f, "%e"),
            #[cfg(feature = "complex")]
            Expr::I => write!(f, "%i"),
            Expr::Factorial(a) => write!(f, "({a})!"),
            Expr::Tensor(name, indices) => {
                write!(f, "{}", name)?;
                for idx in indices {
                    write!(f, "{}", idx)?;
                }
                Ok(())
            }
            Expr::Derivative(expr, var) => write!(f, "diff({}, {})", expr, var),
            Expr::Integral(expr, var) => write!(f, "integrate({}, {})", expr, var),
            Expr::Sum(expr, var, lower, upper) => {
                write!(f, "sum({}, {}, {}, {})", expr, var, lower, upper)
            }
            Expr::Matrix(mat) => {
                write!(f, "matrix(")?;
                for (i, row) in mat.iter().enumerate() {
                    write!(f, "[")?;
                    for (j, elem) in row.iter().enumerate() {
                        write!(f, "{}", elem)?;
                        if j < row.len() - 1 {
                            write!(f, ", ")?;
                        }
                    }
                    write!(f, "]")?;
                    if i < mat.len() - 1 {
                        write!(f, ", ")?;
                    }
                }
                write!(f, ")")
            }
            #[cfg(feature = "specfn")]
            Expr::SpecialFn(kind, args) => {
                write!(f, "{}(", kind.as_str())?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
            }
        }
    }
}

// ── Operator Overloading for Ergonomic Construction ────────────────
//
// Allows writing: let expr = x.clone() + Expr::int(1);

impl std::ops::Add for Expr {
    type Output = Expr;
    fn add(self, rhs: Expr) -> Expr {
        Expr::add(self, rhs)
    }
}

impl std::ops::Sub for Expr {
    type Output = Expr;
    fn sub(self, rhs: Expr) -> Expr {
        Expr::sub(self, rhs)
    }
}

impl std::ops::Mul for Expr {
    type Output = Expr;
    fn mul(self, rhs: Expr) -> Expr {
        Expr::mul(self, rhs)
    }
}

impl std::ops::Div for Expr {
    type Output = Expr;
    fn div(self, rhs: Expr) -> Expr {
        Expr::div(self, rhs)
    }
}

impl std::ops::Neg for Expr {
    type Output = Expr;
    fn neg(self) -> Expr {
        Expr::neg(self)
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_var_display() {
        let x = Expr::var("x");
        assert_eq!(format!("{x}"), "x");
    }

    #[test]
    fn test_int_display() {
        let two = Expr::int(2);
        assert_eq!(format!("{two}"), "2");
    }

    #[test]
    fn test_negative_int_display() {
        let neg_three = Expr::int(-3);
        assert_eq!(format!("{neg_three}"), "(-3)");
    }

    #[test]
    fn test_frac_display() {
        let half = Expr::frac(1, 2);
        assert_eq!(format!("{half}"), "1/2");
    }

    #[test]
    fn test_from_f64_clean_fraction() {
        // 1.5 = 3/2
        let c = Expr::from_f64(1.5);
        assert_eq!(format!("{c}"), "3/2");
    }

    #[test]
    fn test_from_f64_integer() {
        let c = Expr::from_f64(100.0);
        assert_eq!(format!("{c}"), "100");
    }

    #[test]
    fn test_complex_expr_display() {
        let x = Expr::var("x");
        // sin(x) + cos(x)
        let expr = Expr::sin(x.clone()) + Expr::cos(x.clone());
        assert_eq!(format!("{expr}"), "(sin(x) + cos(x))");
    }

    #[test]
    fn test_nested_expr_display() {
        let x = Expr::var("x");
        // x^2 + 2*x + 1
        let expr = Expr::pow(x.clone(), Expr::int(2)) + Expr::int(2) * x.clone() + Expr::int(1);
        assert_eq!(format!("{expr}"), "(((x ^ 2) + (2 * x)) + 1)");
    }

    #[test]
    fn test_eval_simple() {
        let expr = Expr::var("x") + Expr::int(1);
        let mut env = HashMap::new();
        env.insert("x".to_string(), 2.0);
        assert_eq!(expr.eval(&env), Some(3.0));
    }

    #[test]
    fn test_eval_quadratic() {
        // x^2 + 2*x + 1 at x=3 => 9+6+1 = 16
        let x = Expr::var("x");
        let expr = Expr::pow(x.clone(), Expr::int(2)) + Expr::int(2) * x.clone() + Expr::int(1);
        let mut env = HashMap::new();
        env.insert("x".to_string(), 3.0);
        assert_eq!(expr.eval(&env), Some(16.0));
    }

    #[test]
    fn test_eval_trig() {
        let x = Expr::var("x");
        // sin(0) = 0
        let expr = Expr::sin(x.clone());
        let mut env = HashMap::new();
        env.insert("x".to_string(), 0.0);
        assert!((expr.eval(&env).unwrap() - 0.0).abs() < 1e-15);
    }

    #[test]
    fn test_eval_missing_var() {
        let expr = Expr::var("x") + Expr::var("y");
        let mut env = HashMap::new();
        env.insert("x".to_string(), 1.0);
        // y is missing
        assert_eq!(expr.eval(&env), None);
    }

    #[test]
    fn test_eval_div_by_zero() {
        let expr = Expr::div(Expr::int(1), Expr::int(0));
        let env = HashMap::new();
        assert_eq!(expr.eval(&env), None);
    }

    #[test]
    fn test_eval_ln_negative() {
        let expr = Expr::ln(Expr::int(-1));
        let env = HashMap::new();
        assert_eq!(expr.eval(&env), None);
    }

    #[test]
    fn test_variables() {
        let expr = Expr::add(Expr::var("x"), Expr::mul(Expr::var("y"), Expr::var("x")));
        let mut vars = expr.variables();
        vars.sort();
        assert_eq!(vars, vec!["x", "y"]);
    }

    #[test]
    fn test_variables_constants_only() {
        let expr = Expr::add(Expr::int(1), Expr::int(2));
        assert!(expr.variables().is_empty());
    }

    #[test]
    fn test_is_zero() {
        assert!(Expr::int(0).is_zero());
        assert!(!Expr::int(1).is_zero());
        assert!(!Expr::var("x").is_zero());
    }

    #[test]
    fn test_is_one() {
        assert!(Expr::int(1).is_one());
        assert!(!Expr::int(0).is_one());
        assert!(!Expr::var("x").is_one());
    }

    #[test]
    fn test_node_count() {
        // x + 1 => 3 nodes (Add, Var, Const)
        let expr = Expr::var("x") + Expr::int(1);
        assert_eq!(expr.node_count(), 3);
    }

    #[test]
    fn test_clone_equality() {
        let x = Expr::var("x");
        let a = Expr::sin(x.clone());
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn test_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let a = Expr::sin(Expr::var("x"));
        let b = Expr::sin(Expr::var("x"));

        let hash = |e: &Expr| {
            let mut h = DefaultHasher::new();
            e.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(&a), hash(&b));
    }

    #[test]
    fn test_operator_overloading() {
        let x = Expr::var("x");
        let y = Expr::var("y");

        // Test all operator overloads produce the right Display
        assert_eq!(format!("{}", x.clone() + y.clone()), "(x + y)");
        assert_eq!(format!("{}", x.clone() - y.clone()), "(x - y)");
        assert_eq!(format!("{}", x.clone() * y.clone()), "(x * y)");
        assert_eq!(format!("{}", x.clone() / y.clone()), "(x / y)");
        assert_eq!(format!("{}", -x.clone()), "(-x)");
    }

    #[test]
    fn test_rosenbrock_eval() {
        // Rosenbrock: (1 - x)^2 + 100*(y - x^2)^2
        // At (1, 1): value = 0
        let x = Expr::var("x");
        let y = Expr::var("y");

        let expr = Expr::pow(Expr::int(1) - x.clone(), Expr::int(2))
            + Expr::int(100)
                * Expr::pow(y.clone() - Expr::pow(x.clone(), Expr::int(2)), Expr::int(2));

        let mut env = HashMap::new();
        env.insert("x".to_string(), 1.0);
        env.insert("y".to_string(), 1.0);

        let result = expr.eval(&env).unwrap();
        assert!(
            (result - 0.0).abs() < 1e-15,
            "Rosenbrock at (1,1) should be 0, got {result}"
        );
    }

    #[test]
    fn test_hyperbolic_display_and_eval() {
        let expr = Expr::sinh(Expr::int(0));
        assert_eq!(format!("{expr}"), "sinh(0)");
        assert_eq!(expr.eval(&HashMap::new()), Some(0.0));
        let env = HashMap::new();
        assert!((Expr::cosh(Expr::int(0)).eval(&env).unwrap() - 1.0).abs() < 1e-15);
        assert!((Expr::tanh(Expr::int(0)).eval(&env).unwrap()).abs() < 1e-15);
    }

    #[test]
    fn test_inverse_trig_eval() {
        let env = HashMap::new();
        assert!(Expr::asin(Expr::int(0)).eval(&env).unwrap().abs() < 1e-15);
        assert!(Expr::acos(Expr::int(1)).eval(&env).unwrap().abs() < 1e-15);
        assert!(Expr::atan(Expr::int(0)).eval(&env).unwrap().abs() < 1e-15);
    }

    #[test]
    fn test_constants_display_and_eval() {
        let env = HashMap::new();
        assert_eq!(format!("{}", Expr::pi()), "%pi");
        assert_eq!(format!("{}", Expr::e()), "%e");
        assert!((Expr::pi().eval(&env).unwrap() - std::f64::consts::PI).abs() < 1e-15);
        assert!((Expr::e().eval(&env).unwrap() - std::f64::consts::E).abs() < 1e-15);
    }

    #[test]
    fn test_factorial_display_and_eval() {
        let env = HashMap::new();
        let f = Expr::factorial(Expr::int(5));
        assert_eq!(format!("{f}"), "(5)!");
        assert_eq!(f.eval(&env), Some(120.0));
        assert_eq!(Expr::factorial(Expr::frac(1, 2)).eval(&env), None);
        assert_eq!(Expr::factorial(Expr::int(1)).eval(&env), Some(1.0));
    }

    #[test]
    fn test_new_nodes_variables() {
        let expr = Expr::add(Expr::pi(), Expr::factorial(Expr::var("x")));
        assert_eq!(expr.variables(), vec!["x".to_string()]);
        assert_eq!(Expr::e().variables(), Vec::<String>::new());
    }
}
