//! Analytical differentiation engine.
//!
//! Provides deterministic symbolic differentiation directly on [`Expr`] trees.
//! Computes exact partial derivatives, gradients, Jacobians, and Hessians
//! using recursive calculus rules.
//!
//! # Calculus Rules Applied
//!
//! - **Constant**: $\frac{d}{dx}(c) = 0$
//! - **Variable**: $\frac{d}{dx}(x) = 1$, $\frac{d}{dx}(y) = 0$ ($y \neq x$)
//! - **Sum**: $\frac{d}{dx}(u + v) = u' + v'$
//! - **Difference**: $\frac{d}{dx}(u - v) = u' - v'$
//! - **Product**: $\frac{d}{dx}(u \cdot v) = u' \cdot v + u \cdot v'$
//! - **Quotient**: $\frac{d}{dx}\left(\frac{u}{v}\right) = \frac{u' \cdot v - u \cdot v'}{v^2}$
//! - **Power (Constant Exponent)**: $\frac{d}{dx}(u^n) = n \cdot u^{n-1} \cdot u'$
//! - **Exponential (Constant Base)**: $\frac{d}{dx}(c^v) = c^v \cdot \ln(c) \cdot v'$
//! - **General Power**: $\frac{d}{dx}(u^v) = u^v \cdot \left(v' \cdot \ln(u) + v \cdot \frac{u'}{u}\right)$
//! - **Trigonometric**:
//!   - $\frac{d}{dx}(\sin(u)) = \cos(u) \cdot u'$
//!   - $\frac{d}{dx}(\cos(u)) = -\sin(u) \cdot u'$
//!   - $\frac{d}{dx}(\tan(u)) = (1 + \tan(u)^2) \cdot u'$
//! - **Transcendental**:
//!   - $\frac{d}{dx}(\exp(u)) = \exp(u) \cdot u'$
//!   - $\frac{d}{dx}(\ln(u)) = \frac{u'}{u}$
//!   - $\frac{d}{dx}(\sqrt{u}) = \frac{u'}{2 \cdot \sqrt{u}}$

use crate::expr::Expr;
use crate::rational::Rat;

/// Compute the symbolic partial derivative of `expr` with respect to `var`.
///
/// Applies standard calculus rules recursively. Applies basic arithmetic
/// identity cleanups ($0 \cdot x = 0$, $1 \cdot x = x$, $0 + x = x$) during
/// traversal to produce compact derivative trees.
///
/// # Examples
///
/// ```
/// use sym_core::expr::Expr;
/// use sym_core::diff::differentiate;
///
/// let x = Expr::var("x");
///
/// // d/dx(x^2) = 2 * x
/// let f = Expr::pow(x.clone(), Expr::int(2));
/// let df = differentiate(&f, "x");
/// assert_eq!(format!("{df}"), "(2 * x)");
///
/// // d/dx(sin(x)) = cos(x)
/// let f_sin = Expr::sin(x.clone());
/// let df_sin = differentiate(&f_sin, "x");
/// assert_eq!(format!("{df_sin}"), "cos(x)");
/// ```
pub fn differentiate(expr: &Expr, var: &str) -> Expr {
    match expr {
        Expr::Var(name) => {
            if name == var {
                Expr::int(1)
            } else {
                Expr::int(0)
            }
        }
        Expr::Const(_) => Expr::int(0),
        Expr::Add(a, b) => {
            let da = differentiate(a, var);
            let db = differentiate(b, var);
            smart_add(da, db)
        }
        Expr::Sub(a, b) => {
            let da = differentiate(a, var);
            let db = differentiate(b, var);
            smart_sub(da, db)
        }
        Expr::Mul(a, b) => {
            let da = differentiate(a, var);
            let db = differentiate(b, var);
            let da_b = smart_mul(da, *b.clone());
            let a_db = smart_mul(*a.clone(), db);
            smart_add(da_b, a_db)
        }
        Expr::Div(a, b) => {
            let da = differentiate(a, var);
            let db = differentiate(b, var);

            if db.is_zero() {
                // Denominator is constant w.r.t var: d/dx(a / c) = da / c
                smart_div(da, *b.clone())
            } else if da.is_zero() {
                // Numerator is constant: d/dx(c / b) = -c * db / b^2
                let b_sq = smart_pow(*b.clone(), Expr::int(2));
                let neg_num = smart_neg(smart_mul(*a.clone(), db));
                smart_div(neg_num, b_sq)
            } else {
                // Full quotient rule: (da * b - a * db) / b^2
                let num = smart_sub(smart_mul(da, *b.clone()), smart_mul(*a.clone(), db));
                let b_sq = smart_pow(*b.clone(), Expr::int(2));
                smart_div(num, b_sq)
            }
        }
        Expr::Pow(base, exp) => {
            let du = differentiate(base, var);
            let dv = differentiate(exp, var);

            if dv.is_zero() {
                // Power rule: d/dx(u^n) = n * u^(n-1) * du
                if du.is_zero() {
                    Expr::int(0)
                } else if exp.is_one() {
                    du
                } else if exp.is_zero() {
                    Expr::int(0)
                } else {
                    let n_minus_1 = match exp.as_ref() {
                        Expr::Const(r) => Expr::constant(Rat::sub(r, &Rat::from_int(1))),
                        _ => smart_sub(*exp.clone(), Expr::int(1)),
                    };
                    let power_term = smart_pow(*base.clone(), n_minus_1);
                    smart_mul(smart_mul(*exp.clone(), power_term), du)
                }
            } else if du.is_zero() {
                // Exponential rule: d/dx(c^v) = c^v * ln(c) * dv
                let orig = Expr::pow(*base.clone(), *exp.clone());
                let ln_base = Expr::ln(*base.clone());
                smart_mul(smart_mul(orig, ln_base), dv)
            } else {
                // General rule: u^v * (dv * ln(u) + v * du / u)
                let orig = Expr::pow(*base.clone(), *exp.clone());
                let term1 = smart_mul(dv, Expr::ln(*base.clone()));
                let term2 = smart_div(smart_mul(*exp.clone(), du), *base.clone());
                let bracket = smart_add(term1, term2);
                smart_mul(orig, bracket)
            }
        }
        Expr::Neg(a) => {
            let da = differentiate(a, var);
            smart_neg(da)
        }
        Expr::Sin(a) => {
            let da = differentiate(a, var);
            smart_mul(Expr::cos(*a.clone()), da)
        }
        Expr::Cos(a) => {
            let da = differentiate(a, var);
            smart_mul(smart_neg(Expr::sin(*a.clone())), da)
        }
        Expr::Tan(a) => {
            let da = differentiate(a, var);
            // d/dx(tan(a)) = (1 + tan(a)^2) * da
            let tan_sq = smart_pow(Expr::tan(*a.clone()), Expr::int(2));
            let sec_sq = smart_add(Expr::int(1), tan_sq);
            smart_mul(sec_sq, da)
        }
        Expr::Exp(a) => {
            let da = differentiate(a, var);
            smart_mul(Expr::exp(*a.clone()), da)
        }
        Expr::Ln(a) => {
            let da = differentiate(a, var);
            // d/dx(ln(a)) = da / a
            smart_div(da, *a.clone())
        }
        Expr::Sqrt(a) => {
            let da = differentiate(a, var);
            // d/dx(sqrt(a)) = da / (2 * sqrt(a))
            let denom = smart_mul(Expr::int(2), Expr::sqrt(*a.clone()));
            smart_div(da, denom)
        }
        Expr::Sinh(a) => {
            let da = differentiate(a, var);
            smart_mul(Expr::cosh(*a.clone()), da)
        }
        Expr::Cosh(a) => {
            let da = differentiate(a, var);
            smart_mul(Expr::sinh(*a.clone()), da)
        }
        Expr::Tanh(a) => {
            let da = differentiate(a, var);
            // d/dx tanh(a) = (1 - tanh(a)^2) * a'
            let tanh_sq = smart_pow(Expr::tanh(*a.clone()), Expr::int(2));
            let sech_sq = smart_sub(Expr::int(1), tanh_sq);
            smart_mul(sech_sq, da)
        }
        Expr::Asin(a) => {
            let da = differentiate(a, var);
            // d/dx asin(a) = a' / sqrt(1 - a^2)
            let a_sq = smart_pow(*a.clone(), Expr::int(2));
            let denom = Expr::sqrt(smart_sub(Expr::int(1), a_sq));
            smart_div(da, denom)
        }
        Expr::Acos(a) => {
            let da = differentiate(a, var);
            // d/dx acos(a) = -a' / sqrt(1 - a^2)
            let a_sq = smart_pow(*a.clone(), Expr::int(2));
            let denom = Expr::sqrt(smart_sub(Expr::int(1), a_sq));
            smart_div(smart_neg(da), denom)
        }
        Expr::Atan(a) => {
            let da = differentiate(a, var);
            // d/dx atan(a) = a' / (1 + a^2)
            let a_sq = smart_pow(*a.clone(), Expr::int(2));
            let denom = smart_add(Expr::int(1), a_sq);
            smart_div(da, denom)
        }
        // π and e are constants with respect to any variable.
        Expr::Pi | Expr::E => Expr::int(0),
        // Factorials are integer-valued; treat as locally constant (Maxima parity).
        Expr::Factorial(_) => Expr::int(0),
        Expr::Tensor(name, _indices) => {
            // For now, differentiate of a tensor with respect to a scalar variable is 0
            // unless the tensor's exact name matches the variable name.
            if name == var {
                Expr::int(1)
            } else {
                Expr::int(0)
            }
        }
        Expr::Derivative(f, v) => {
            // d/dx (df/dy) = d^2 f / (dx dy)
            // Just wrap it in another Derivative node for now
            Expr::derivative(Expr::derivative(*f.clone(), *v.clone()), Expr::var(var))
        }
        Expr::Integral(f, v) => {
            // Leibniz rule could be applied, but keep it simple: un-evaluated derivative
            Expr::derivative(Expr::integral(*f.clone(), *v.clone()), Expr::var(var))
        }
        Expr::Sum(f, i, lower, upper) => {
            // d/dx(sum) = sum(d/dx) + ... keep it simple
            Expr::derivative(
                Expr::sum(*f.clone(), *i.clone(), *lower.clone(), *upper.clone()),
                Expr::var(var),
            )
        }
        Expr::Matrix(mat) => {
            // Element-wise differentiation
            let d_mat: Vec<Vec<Expr>> = mat
                .iter()
                .map(|row| row.iter().map(|e| differentiate(e, var)).collect())
                .collect();
            Expr::Matrix(d_mat)
        }
    }
}

/// Compute the gradient of `expr` with respect to a list of variables.
///
/// Returns a `Vec<Expr>` with one partial derivative per variable in `vars`.
///
/// # Examples
///
/// ```
/// use sym_core::expr::Expr;
/// use sym_core::diff::gradient;
///
/// let x = Expr::var("x");
/// let y = Expr::var("y");
///
/// // f(x, y) = x^2 + y^2
/// let f = Expr::pow(x, Expr::int(2)) + Expr::pow(y, Expr::int(2));
/// let grad = gradient(&f, &["x", "y"]);
/// assert_eq!(grad.len(), 2);
/// assert_eq!(format!("{}", grad[0]), "(2 * x)");
/// assert_eq!(format!("{}", grad[1]), "(2 * y)");
/// ```
pub fn gradient(expr: &Expr, vars: &[&str]) -> Vec<Expr> {
    vars.iter().map(|&v| differentiate(expr, v)).collect()
}

/// Compute the Jacobian matrix for a vector of expressions.
///
/// For vector $\mathbf{f} = [f_1, \dots, f_m]$ and variables $\mathbf{x} = [x_1, \dots, x_n]$,
/// the Jacobian $J$ is an $m \times n$ matrix where $J_{ij} = \frac{\partial f_i}{\partial x_j}$.
///
/// # Examples
///
/// ```
/// use sym_core::expr::Expr;
/// use sym_core::diff::jacobian;
///
/// let x = Expr::var("x");
/// let y = Expr::var("y");
///
/// // f1 = x * y,  f2 = x + y
/// let f1 = x.clone() * y.clone();
/// let f2 = x + y;
/// let jac = jacobian(&[f1, f2], &["x", "y"]);
/// assert_eq!(jac.len(), 2);
/// assert_eq!(jac[0].len(), 2);
/// assert_eq!(format!("{}", jac[0][0]), "y");
/// assert_eq!(format!("{}", jac[0][1]), "x");
/// assert_eq!(format!("{}", jac[1][0]), "1");
/// assert_eq!(format!("{}", jac[1][1]), "1");
/// ```
pub fn jacobian(exprs: &[Expr], vars: &[&str]) -> Vec<Vec<Expr>> {
    exprs.iter().map(|e| gradient(e, vars)).collect()
}

/// Compute the Hessian matrix of second partial derivatives.
///
/// For scalar expression $f$ and variables $[x_1, \dots, x_n]$, the Hessian
/// $H$ is an $n \times n$ matrix where $H_{ij} = \frac{\partial^2 f}{\partial x_i \partial x_j}$.
///
/// # Examples
///
/// ```
/// use sym_core::expr::Expr;
/// use sym_core::diff::hessian;
///
/// let x = Expr::var("x");
/// let y = Expr::var("y");
///
/// // f(x, y) = x^2 * y
/// let f = Expr::pow(x, Expr::int(2)) * y;
/// let h = hessian(&f, &["x", "y"]);
/// assert_eq!(h.len(), 2);
/// assert_eq!(h[0].len(), 2);
/// // H_xx = 2 * y
/// assert_eq!(format!("{}", h[0][0]), "(2 * y)");
/// // H_xy = 2 * x
/// assert_eq!(format!("{}", h[0][1]), "(2 * x)");
/// // H_yx = 2 * x
/// assert_eq!(format!("{}", h[1][0]), "(2 * x)");
/// // H_yy = 0
/// assert_eq!(format!("{}", h[1][1]), "0");
/// ```
pub fn hessian(expr: &Expr, vars: &[&str]) -> Vec<Vec<Expr>> {
    let grad = gradient(expr, vars);
    grad.iter().map(|g_i| gradient(g_i, vars)).collect()
}

// ── Smart Algebraic Constructors for Compact Derivatives ──────────

fn smart_add(a: Expr, b: Expr) -> Expr {
    if a.is_zero() {
        b
    } else if b.is_zero() {
        a
    } else {
        Expr::add(a, b)
    }
}

fn smart_sub(a: Expr, b: Expr) -> Expr {
    if b.is_zero() {
        a
    } else if a.is_zero() {
        smart_neg(b)
    } else {
        Expr::sub(a, b)
    }
}

fn smart_mul(a: Expr, b: Expr) -> Expr {
    if a.is_zero() || b.is_zero() {
        Expr::int(0)
    } else if a.is_one() {
        b
    } else if b.is_one() {
        a
    } else {
        Expr::mul(a, b)
    }
}

fn smart_div(a: Expr, b: Expr) -> Expr {
    if a.is_zero() {
        Expr::int(0)
    } else if b.is_one() {
        a
    } else {
        Expr::div(a, b)
    }
}

fn smart_pow(base: Expr, exp: Expr) -> Expr {
    if exp.is_zero() {
        Expr::int(1)
    } else if exp.is_one() {
        base
    } else if base.is_zero() {
        Expr::int(0)
    } else if base.is_one() {
        Expr::int(1)
    } else {
        Expr::pow(base, exp)
    }
}

fn smart_neg(a: Expr) -> Expr {
    if a.is_zero() {
        Expr::int(0)
    } else if let Expr::Neg(inner) = a {
        *inner
    } else {
        Expr::neg(a)
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_diff_var_same() {
        assert_eq!(differentiate(&Expr::var("x"), "x"), Expr::int(1));
    }

    #[test]
    fn test_diff_var_different() {
        assert_eq!(differentiate(&Expr::var("y"), "x"), Expr::int(0));
    }

    #[test]
    fn test_diff_constant() {
        assert_eq!(differentiate(&Expr::int(42), "x"), Expr::int(0));
        assert_eq!(differentiate(&Expr::frac(3, 7), "x"), Expr::int(0));
    }

    #[test]
    fn test_diff_polynomial() {
        // d/dx(x^3 + 2*x^2 + 5*x + 7) = 3*x^2 + 4*x + 5
        let x = Expr::var("x");
        let poly = Expr::pow(x.clone(), Expr::int(3))
            + Expr::int(2) * Expr::pow(x.clone(), Expr::int(2))
            + Expr::int(5) * x.clone()
            + Expr::int(7);

        let dpoly = differentiate(&poly, "x");

        // Verify numerically at x = 2.0:
        // derivative is 3*(4) + 4*(2) + 5 = 12 + 8 + 5 = 25
        let mut env = HashMap::new();
        env.insert("x".to_string(), 2.0);
        let val = dpoly.eval(&env).unwrap();
        assert!((val - 25.0).abs() < 1e-12, "expected 25.0, got {val}");
    }

    #[test]
    fn test_diff_product_rule() {
        // d/dx(x * y) w.r.t x => y
        let x = Expr::var("x");
        let y = Expr::var("y");
        let f = x.clone() * y.clone();
        assert_eq!(differentiate(&f, "x"), y);
        assert_eq!(differentiate(&f, "y"), x);
    }

    #[test]
    fn test_diff_quotient_rule() {
        // d/dx(1 / x) = -1 / x^2
        let x = Expr::var("x");
        let f = Expr::div(Expr::int(1), x.clone());
        let df = differentiate(&f, "x");

        let mut env = HashMap::new();
        env.insert("x".to_string(), 2.0);
        // at x=2, -1/4 = -0.25
        assert_eq!(df.eval(&env), Some(-0.25));
    }

    #[test]
    fn test_diff_sin_cos() {
        let x = Expr::var("x");
        let f = Expr::sin(x.clone());
        assert_eq!(differentiate(&f, "x"), Expr::cos(x.clone()));

        let g = Expr::cos(x.clone());
        assert_eq!(differentiate(&g, "x"), -Expr::sin(x.clone()));
    }

    #[test]
    fn test_diff_exp_ln() {
        let x = Expr::var("x");
        let f = Expr::exp(x.clone());
        assert_eq!(differentiate(&f, "x"), Expr::exp(x.clone()));

        let g = Expr::ln(x.clone());
        assert_eq!(differentiate(&g, "x"), Expr::div(Expr::int(1), x));
    }

    #[test]
    fn test_diff_tan() {
        let x = Expr::var("x");
        let f = Expr::tan(x.clone());
        let df = differentiate(&f, "x");

        // at x = 0: 1 + tan(0)^2 = 1
        let mut env = HashMap::new();
        env.insert("x".to_string(), 0.0);
        assert!((df.eval(&env).unwrap() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_diff_sqrt() {
        let x = Expr::var("x");
        let f = Expr::sqrt(x.clone());
        let df = differentiate(&f, "x");

        // at x = 4: 1 / (2 * 2) = 0.25
        let mut env = HashMap::new();
        env.insert("x".to_string(), 4.0);
        assert!((df.eval(&env).unwrap() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn test_diff_chain_rule() {
        // d/dx(sin(x^2)) = 2*x*cos(x^2)
        let x = Expr::var("x");
        let f = Expr::sin(Expr::pow(x.clone(), Expr::int(2)));
        let df = differentiate(&f, "x");

        // at x = 1.0: 2 * 1 * cos(1) ≈ 1.0806046117
        let mut env = HashMap::new();
        env.insert("x".to_string(), 1.0);
        let expected = 2.0 * 1.0_f64.cos();
        let actual = df.eval(&env).unwrap();
        assert!((actual - expected).abs() < 1e-12);
    }

    #[test]
    fn test_diff_rosenbrock_gradient() {
        // Rosenbrock: f(x, y) = (1 - x)^2 + 100*(y - x^2)^2
        // df/dx = -2*(1 - x) - 400*x*(y - x^2)
        // df/dy = 200*(y - x^2)
        let x = Expr::var("x");
        let y = Expr::var("y");
        let f = Expr::pow(Expr::int(1) - x.clone(), Expr::int(2))
            + Expr::int(100)
                * Expr::pow(y.clone() - Expr::pow(x.clone(), Expr::int(2)), Expr::int(2));

        let grad = gradient(&f, &["x", "y"]);
        assert_eq!(grad.len(), 2);

        // At minimum (1, 1), gradient must be exactly [0, 0]
        let mut env = HashMap::new();
        env.insert("x".to_string(), 1.0);
        env.insert("y".to_string(), 1.0);

        let gx = grad[0].eval(&env).unwrap();
        let gy = grad[1].eval(&env).unwrap();
        assert!((gx - 0.0).abs() < 1e-12, "gx should be 0, got {gx}");
        assert!((gy - 0.0).abs() < 1e-12, "gy should be 0, got {gy}");

        // At (0, 0):
        // df/dx = -2*(1) - 400*0 = -2
        // df/dy = 200*(0) = 0
        env.insert("x".to_string(), 0.0);
        env.insert("y".to_string(), 0.0);
        let gx0 = grad[0].eval(&env).unwrap();
        let gy0 = grad[1].eval(&env).unwrap();
        assert!(
            (gx0 - (-2.0)).abs() < 1e-12,
            "gx at (0,0) should be -2, got {gx0}"
        );
        assert!(
            (gy0 - 0.0).abs() < 1e-12,
            "gy at (0,0) should be 0, got {gy0}"
        );
    }

    #[test]
    fn test_hessian_symmetry() {
        // Schwarz's theorem: H_xy == H_yx for smooth f
        let x = Expr::var("x");
        let y = Expr::var("y");
        let f = Expr::sin(x.clone() * y.clone()) + Expr::exp(x.clone() + y.clone());

        let h = hessian(&f, &["x", "y"]);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].len(), 2);

        let mut env = HashMap::new();
        env.insert("x".to_string(), 0.5);
        env.insert("y".to_string(), 0.7);

        let h_xy = h[0][1].eval(&env).unwrap();
        let h_yx = h[1][0].eval(&env).unwrap();
        assert!(
            (h_xy - h_yx).abs() < 1e-12,
            "Hessian cross partials must match: H_xy={h_xy}, H_yx={h_yx}"
        );
    }

    #[test]
    fn test_diff_hyperbolic() {
        let x = Expr::var("x");
        let mut env = HashMap::new();
        env.insert("x".to_string(), 0.3);

        let d_sinh = differentiate(&Expr::sinh(x.clone()), "x");
        assert!((d_sinh.eval(&env).unwrap() - 0.3_f64.cosh()).abs() < 1e-12);

        let d_cosh = differentiate(&Expr::cosh(x.clone()), "x");
        assert!((d_cosh.eval(&env).unwrap() - 0.3_f64.sinh()).abs() < 1e-12);

        let d_tanh = differentiate(&Expr::tanh(x), "x");
        let expected = 1.0 - 0.3_f64.tanh().powi(2);
        assert!((d_tanh.eval(&env).unwrap() - expected).abs() < 1e-12);
    }

    #[test]
    fn test_diff_inverse_trig() {
        let x = Expr::var("x");
        let mut env = HashMap::new();
        env.insert("x".to_string(), 0.3);

        let expected_asin = 1.0 / (1.0_f64 - 0.09).sqrt();
        let d_asin = differentiate(&Expr::asin(x.clone()), "x");
        assert!((d_asin.eval(&env).unwrap() - expected_asin).abs() < 1e-12);

        let d_acos = differentiate(&Expr::acos(x.clone()), "x");
        assert!((d_acos.eval(&env).unwrap() + expected_asin).abs() < 1e-12);

        let d_atan = differentiate(&Expr::atan(x), "x");
        assert!((d_atan.eval(&env).unwrap() - 1.0 / 1.09_f64).abs() < 1e-12);
    }

    #[test]
    fn test_diff_constants_and_factorial() {
        assert_eq!(differentiate(&Expr::pi(), "x"), Expr::int(0));
        assert_eq!(differentiate(&Expr::e(), "x"), Expr::int(0));
        assert_eq!(
            differentiate(&Expr::factorial(Expr::int(5)), "x"),
            Expr::int(0)
        );
    }
}
