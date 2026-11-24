//! Precedence-aware LaTeX formatter for symbolic expressions.
//!
//! Converts [`Expr`] trees into clean, idiomatic LaTeX formulas with minimal,
//! mathematically necessary parentheses according to standard operator precedence:
//! - Precedence 1: Addition (`+`) and Subtraction (`-`)
//! - Precedence 2: Multiplication (`\cdot`)
//! - Precedence 3: Unary negation (`-`)
//! - Precedence 4: Exponentiation (`^`)
//! - Precedence 5: Fractions, functions, delimiters, and atoms

use crate::expr::Expr;
use num_traits::{One, Signed};

/// Operator precedence levels.
pub const PREC_NONE: u8 = 0;
pub const PREC_ADD_SUB: u8 = 1;
pub const PREC_MUL: u8 = 2;
pub const PREC_NEG: u8 = 3;
pub const PREC_POW: u8 = 4;
pub const PREC_ATOM: u8 = 5;

/// Convert an [`Expr`] AST into a precedence-aware LaTeX formatted string.
///
/// # Examples
/// ```
/// use sym_core::{to_latex, Expr};
///
/// let expr = Expr::add(Expr::var("x"), Expr::int(1));
/// assert_eq!(to_latex(&expr), "x + 1");
///
/// let prod = Expr::mul(Expr::add(Expr::var("x"), Expr::int(1)), Expr::var("y"));
/// assert_eq!(to_latex(&prod), r"\left( x + 1 \right) \cdot y");
///
/// let frac = Expr::div(Expr::add(Expr::var("x"), Expr::int(1)), Expr::int(2));
/// assert_eq!(to_latex(&frac), r"\frac{x + 1}{2}");
/// ```
pub fn to_latex(expr: &Expr) -> String {
    format_expr(expr, PREC_NONE, false)
}

/// Recursively formats an [`Expr`] considering parent operator precedence and associativity.
fn format_expr(expr: &Expr, parent_prec: u8, is_right: bool) -> String {
    match expr {
        Expr::Var(name) => format_var_name(name),

        #[cfg(feature = "bfloat")]
        Expr::Bfloat(b) => b.to_latex(),

        Expr::Const(r) => {
            if r.denom().is_one() {
                let n = r.numer();
                if n.is_negative() {
                    let s = format!("{n}");
                    if parent_prec >= PREC_POW || (parent_prec == PREC_MUL && is_right) {
                        format!("\\left( {s} \\right)")
                    } else {
                        s
                    }
                } else {
                    format!("{n}")
                }
            } else if r.is_negative() {
                let s = format!("-\\frac{{{}}}{{{}}}", r.numer().abs(), r.denom());
                if parent_prec >= PREC_POW || (parent_prec == PREC_MUL && is_right) {
                    format!("\\left( {s} \\right)")
                } else {
                    s
                }
            } else {
                let s = format!("\\frac{{{}}}{{{}}}", r.numer(), r.denom());
                if parent_prec >= PREC_POW && !is_right {
                    format!("\\left( {s} \\right)")
                } else {
                    s
                }
            }
        }

        Expr::Pi => "\\pi".to_string(),
        Expr::E => "e".to_string(),
        #[cfg(feature = "complex")]
        Expr::I => "i".to_string(),

        Expr::Tensor(name, indices) => {
            let base = format_var_name(name);
            let mut s = base;
            for idx in indices {
                let idx_str = format_var_name(&idx.name);
                match idx.idx_type {
                    crate::tensor::IndexType::Covariant => s.push_str(&format!("_{{{idx_str}}}")),
                    crate::tensor::IndexType::Contravariant => {
                        s.push_str(&format!("^{{{idx_str}}}"))
                    }
                }
            }
            s
        }

        Expr::Add(a, b) => {
            let left = format_expr(a, PREC_ADD_SUB, false);
            let body = match b.as_ref() {
                Expr::Neg(inner) => {
                    let right = format_expr(inner, PREC_ADD_SUB, true);
                    format!("{left} - {right}")
                }
                Expr::Const(r) if r.is_negative() => {
                    let pos_r = -r.clone();
                    let right = format_expr(&Expr::Const(pos_r), PREC_ADD_SUB, true);
                    format!("{left} - {right}")
                }
                _ => {
                    let right = format_expr(b, PREC_ADD_SUB, true);
                    format!("{left} + {right}")
                }
            };
            if parent_prec > PREC_ADD_SUB || (parent_prec == PREC_ADD_SUB && is_right) {
                format!("\\left( {body} \\right)")
            } else {
                body
            }
        }

        Expr::Sub(a, b) => {
            let left = format_expr(a, PREC_ADD_SUB, false);
            // In subtraction, right child with prec <= PREC_ADD_SUB needs parentheses: a - (b + c)
            let right = format_expr(b, PREC_ADD_SUB + 1, true);
            let body = format!("{left} - {right}");
            if parent_prec > PREC_ADD_SUB || (parent_prec == PREC_ADD_SUB && is_right) {
                format!("\\left( {body} \\right)")
            } else {
                body
            }
        }

        Expr::Mul(a, b) => {
            let left = format_expr(a, PREC_MUL, false);
            let right = format_expr(b, PREC_MUL, true);
            let body = format!("{left} \\cdot {right}");
            if parent_prec >= PREC_POW {
                format!("\\left( {body} \\right)")
            } else {
                body
            }
        }

        Expr::Div(a, b) => {
            let num = format_expr(a, PREC_NONE, false);
            let den = format_expr(b, PREC_NONE, false);
            let frac = format!("\\frac{{{num}}}{{{den}}}");
            if parent_prec >= PREC_POW && !is_right {
                format!("\\left( {frac} \\right)")
            } else {
                frac
            }
        }

        Expr::Pow(a, b) => {
            let base = format_expr(a, PREC_POW, false);
            let exp = format_expr(b, PREC_NONE, false);
            let body = format!("{{{base}}}^{{{exp}}}");
            if parent_prec >= PREC_POW && !is_right {
                format!("\\left( {body} \\right)")
            } else {
                body
            }
        }

        Expr::Neg(a) => match a.as_ref() {
            Expr::Add(_, _) | Expr::Sub(_, _) | Expr::Neg(_) => {
                let inner = format_expr(a, PREC_NEG, false);
                format!("-\\left( {inner} \\right)")
            }
            Expr::Div(n, d) => {
                let num = format_expr(n, PREC_NONE, false);
                let den = format_expr(d, PREC_NONE, false);
                let frac = format!("-\\frac{{{num}}}{{{den}}}");
                if parent_prec >= PREC_POW && !is_right {
                    format!("\\left( {frac} \\right)")
                } else {
                    frac
                }
            }
            _ => {
                let inner = format_expr(a, PREC_NEG, false);
                let body = format!("-{inner}");
                if parent_prec >= PREC_POW || (parent_prec == PREC_MUL && is_right) {
                    format!("\\left( {body} \\right)")
                } else {
                    body
                }
            }
        },

        Expr::Sin(a) => format!("\\sin\\left( {} \\right)", format_expr(a, PREC_NONE, false)),
        Expr::Cos(a) => format!("\\cos\\left( {} \\right)", format_expr(a, PREC_NONE, false)),
        Expr::Tan(a) => format!("\\tan\\left( {} \\right)", format_expr(a, PREC_NONE, false)),
        Expr::Sinh(a) => format!(
            "\\sinh\\left( {} \\right)",
            format_expr(a, PREC_NONE, false)
        ),
        Expr::Cosh(a) => format!(
            "\\cosh\\left( {} \\right)",
            format_expr(a, PREC_NONE, false)
        ),
        Expr::Tanh(a) => format!(
            "\\tanh\\left( {} \\right)",
            format_expr(a, PREC_NONE, false)
        ),
        Expr::Asin(a) => format!(
            "\\arcsin\\left( {} \\right)",
            format_expr(a, PREC_NONE, false)
        ),
        Expr::Acos(a) => format!(
            "\\arccos\\left( {} \\right)",
            format_expr(a, PREC_NONE, false)
        ),
        Expr::Atan(a) => format!(
            "\\arctan\\left( {} \\right)",
            format_expr(a, PREC_NONE, false)
        ),

        Expr::Exp(a) => format!("e^{{{}}}", format_expr(a, PREC_NONE, false)),
        Expr::Ln(a) => format!("\\ln\\left( {} \\right)", format_expr(a, PREC_NONE, false)),
        Expr::Sqrt(a) => format!("\\sqrt{{{}}}", format_expr(a, PREC_NONE, false)),

        Expr::Factorial(a) => {
            let inner = match a.as_ref() {
                #[cfg(feature = "complex")]
                Expr::I | Expr::Var(_) | Expr::Pi | Expr::E => format_expr(a, PREC_ATOM, false),
                Expr::Const(r) if r.denom().is_one() && !r.is_negative() => {
                    format_expr(a, PREC_ATOM, false)
                }
                _ => format!("\\left( {} \\right)", format_expr(a, PREC_NONE, false)),
            };
            format!("{inner}!")
        }

        Expr::Derivative(body, var) => {
            if let Expr::Var(_) = body.as_ref() {
                format!(
                    "\\frac{{d {}}}{{d {}}}",
                    format_expr(body, PREC_NONE, false),
                    format_expr(var, PREC_NONE, false)
                )
            } else {
                format!(
                    "\\frac{{d}}{{d {}}} \\left( {} \\right)",
                    format_expr(var, PREC_NONE, false),
                    format_expr(body, PREC_NONE, false)
                )
            }
        }

        Expr::Integral(body, var) => {
            let body_str = match body.as_ref() {
                Expr::Add(_, _) | Expr::Sub(_, _) => {
                    format!("\\left( {} \\right)", format_expr(body, PREC_NONE, false))
                }
                _ => format_expr(body, PREC_NONE, false),
            };
            format!(
                "\\int {} \\, d{}",
                body_str,
                format_expr(var, PREC_NONE, false)
            )
        }

        Expr::Sum(body, var, low, high) => {
            let body_str = match body.as_ref() {
                Expr::Add(_, _) | Expr::Sub(_, _) => {
                    format!("\\left( {} \\right)", format_expr(body, PREC_NONE, false))
                }
                _ => format_expr(body, PREC_NONE, false),
            };
            format!(
                "\\sum_{{{}={}}}^{{{}}} {}",
                format_expr(var, PREC_NONE, false),
                format_expr(low, PREC_NONE, false),
                format_expr(high, PREC_NONE, false),
                body_str
            )
        }

        Expr::Matrix(rows) => {
            let rendered_rows: Vec<String> = rows
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|elem| format_expr(elem, PREC_NONE, false))
                        .collect::<Vec<_>>()
                        .join(" & ")
                })
                .collect();
            format!(
                "\\begin{{pmatrix}} {} \\end{{pmatrix}}",
                rendered_rows.join(" \\\\ ")
            )
        }

        #[cfg(feature = "specfn")]
        Expr::SpecialFn(kind, args) => kind.to_latex(args),
    }
}

/// Checks whether an identifier is a standard Greek symbol name.
fn is_greek(name: &str) -> bool {
    matches!(
        name,
        "alpha"
            | "beta"
            | "gamma"
            | "delta"
            | "epsilon"
            | "zeta"
            | "eta"
            | "theta"
            | "iota"
            | "kappa"
            | "lambda"
            | "mu"
            | "nu"
            | "xi"
            | "pi"
            | "rho"
            | "sigma"
            | "tau"
            | "upsilon"
            | "phi"
            | "chi"
            | "psi"
            | "omega"
            | "Gamma"
            | "Delta"
            | "Theta"
            | "Lambda"
            | "Xi"
            | "Pi"
            | "Sigma"
            | "Upsilon"
            | "Phi"
            | "Psi"
            | "Omega"
            | "varepsilon"
            | "vartheta"
            | "varpi"
            | "varrho"
            | "varsigma"
            | "varphi"
    )
}

/// Formats a variable or identifier name into clean LaTeX.
///
/// Handles Greek symbols (`alpha` -> `\alpha`), solver constants (`_C1` -> `C_{1}`),
/// and subscripts (`x_0` -> `x_{0}`, `theta_1` -> `\theta_{1}`).
pub fn format_var_name(name: &str) -> String {
    if let Some(rest) = name
        .strip_prefix("_C")
        .filter(|r| !r.is_empty() && r.chars().all(|c| c.is_ascii_digit()))
    {
        return format!("C_{{{rest}}}");
    }
    if let Some(rest) = name
        .strip_prefix("_c")
        .filter(|r| !r.is_empty() && r.chars().all(|c| c.is_ascii_digit()))
    {
        return format!("c_{{{rest}}}");
    }
    if let Some((base, sub)) = name.split_once('_') {
        let base_fmt = if is_greek(base) {
            format!("\\{base}")
        } else {
            escape_latex(base)
        };
        let sub_fmt = if is_greek(sub) {
            format!("\\{sub}")
        } else {
            escape_latex(sub)
        };
        return format!("{base_fmt}_{{{sub_fmt}}}");
    }
    if is_greek(name) {
        format!("\\{name}")
    } else {
        escape_latex(name)
    }
}

/// Escapes LaTeX special characters if needed.
fn escape_latex(s: &str) -> String {
    s.replace('\\', "\\backslash ")
        .replace('^', "\\^{}")
        .replace('{', "\\{")
        .replace('}', "\\}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precedence_arithmetic() {
        // x + 1
        let e1 = Expr::add(Expr::var("x"), Expr::int(1));
        assert_eq!(to_latex(&e1), "x + 1");

        // (x + 1) * y
        let e2 = Expr::mul(e1.clone(), Expr::var("y"));
        assert_eq!(to_latex(&e2), r"\left( x + 1 \right) \cdot y");

        // x * y + z
        let e3 = Expr::add(Expr::mul(Expr::var("x"), Expr::var("y")), Expr::var("z"));
        assert_eq!(to_latex(&e3), r"x \cdot y + z");

        // x - (y + z)
        let e4 = Expr::sub(Expr::var("x"), Expr::add(Expr::var("y"), Expr::var("z")));
        assert_eq!(to_latex(&e4), r"x - \left( y + z \right)");

        // x - (y - z)
        let e5 = Expr::sub(Expr::var("x"), Expr::sub(Expr::var("y"), Expr::var("z")));
        assert_eq!(to_latex(&e5), r"x - \left( y - z \right)");
    }

    #[test]
    fn test_powers_and_fractions() {
        // (x + 1)^2
        let p1 = Expr::pow(Expr::add(Expr::var("x"), Expr::int(1)), Expr::int(2));
        assert_eq!(to_latex(&p1), r"{\left( x + 1 \right)}^{2}");

        // (x * y)^2
        let p2 = Expr::pow(Expr::mul(Expr::var("x"), Expr::var("y")), Expr::int(2));
        assert_eq!(to_latex(&p2), r"{\left( x \cdot y \right)}^{2}");

        // (1/2)^2
        let p3 = Expr::pow(Expr::frac(1, 2), Expr::int(2));
        assert_eq!(to_latex(&p3), r"{\left( \frac{1}{2} \right)}^{2}");

        // Fraction: (x + 1) / (y - 1)
        let f1 = Expr::div(
            Expr::add(Expr::var("x"), Expr::int(1)),
            Expr::sub(Expr::var("y"), Expr::int(1)),
        );
        assert_eq!(to_latex(&f1), r"\frac{x + 1}{y - 1}");
    }

    #[test]
    fn test_greek_and_subscripts() {
        assert_eq!(to_latex(&Expr::var("alpha")), r"\alpha");
        assert_eq!(to_latex(&Expr::var("theta_1")), r"\theta_{1}");
        assert_eq!(to_latex(&Expr::var("_C1")), r"C_{1}");
        assert_eq!(to_latex(&Expr::var("_C2")), r"C_{2}");
        assert_eq!(to_latex(&Expr::var("x_0")), r"x_{0}");
    }

    #[test]
    fn test_matrix_and_sum() {
        let mat = Expr::Matrix(vec![
            vec![Expr::int(1), Expr::var("x")],
            vec![Expr::var("y"), Expr::int(2)],
        ]);
        assert_eq!(
            to_latex(&mat),
            r"\begin{pmatrix} 1 & x \\ y & 2 \end{pmatrix}"
        );

        let summ = Expr::Sum(
            Box::new(Expr::var("i")),
            Box::new(Expr::var("i")),
            Box::new(Expr::int(1)),
            Box::new(Expr::var("n")),
        );
        assert_eq!(to_latex(&summ), r"\sum_{i=1}^{n} i");

        let summ_compound = Expr::Sum(
            Box::new(Expr::add(Expr::var("i"), Expr::int(1))),
            Box::new(Expr::var("i")),
            Box::new(Expr::int(1)),
            Box::new(Expr::var("n")),
        );
        assert_eq!(
            to_latex(&summ_compound),
            r"\sum_{i=1}^{n} \left( i + 1 \right)"
        );
    }
}
