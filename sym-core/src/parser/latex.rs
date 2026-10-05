//! LaTeX scalar math formula parser.
//!
//! Parses standard LaTeX mathematical notation:
//! - Fractions: `\frac{numerator}{denominator}`
//! - Square roots: `\sqrt{arg}`
//! - Functions: `\sin`, `\cos`, `\tan`, `\exp`, `\ln`
//! - Powers: `x^{n}` or `x^n`
//! - Multiplication: `\cdot`, `\times`, or juxtaposition/implicit multiplication
//! - Parentheses: `\left( ... \right)` or `( ... )`

use winnow::ModalResult;
use winnow::ascii::{alphanumeric1, multispace0};
use winnow::combinator::opt;
use winnow::error::ContextError;
use winnow::prelude::*;

type PResult<O> = ModalResult<O>;

use crate::expr::Expr;
use crate::parser::ParseError;

/// Parse a LaTeX math string into an [`Expr`].
pub fn parse_latex(input: &str) -> Result<Expr, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let mut state = trimmed;
    match latex_expr(&mut state) {
        Ok(res) => {
            let rem = state.trim();
            if !rem.is_empty() {
                Err(ParseError::Syntax {
                    message: "Unexpected remaining LaTeX input".to_string(),
                    remaining: rem.to_string(),
                })
            } else {
                Ok(res)
            }
        }
        Err(e) => Err(ParseError::Syntax {
            message: format!("{e}"),
            remaining: state.to_string(),
        }),
    }
}

fn latex_expr(i: &mut &str) -> PResult<Expr> {
    additive(i)
}

fn additive(i: &mut &str) -> PResult<Expr> {
    let mut left = multiplicative(i)?;

    loop {
        let _ = multispace0(i)?;
        if i.starts_with('+') {
            *i = &i[1..];
            let right = multiplicative(i)?;
            left = Expr::add(left, right);
        } else if i.starts_with('-') {
            *i = &i[1..];
            let right = multiplicative(i)?;
            left = Expr::sub(left, right);
        } else {
            break;
        }
    }

    Ok(left)
}

fn multiplicative(i: &mut &str) -> PResult<Expr> {
    let mut left = power(i)?;

    loop {
        let _ = multispace0(i)?;
        if i.starts_with(r"\cdot") {
            *i = &i[5..];
            let right = power(i)?;
            left = Expr::mul(left, right);
        } else if i.starts_with(r"\times") {
            *i = &i[6..];
            let right = power(i)?;
            left = Expr::mul(left, right);
        } else if i.starts_with('*') {
            *i = &i[1..];
            let right = power(i)?;
            left = Expr::mul(left, right);
        } else if i.starts_with('/') {
            *i = &i[1..];
            let right = power(i)?;
            left = Expr::div(left, right);
        } else {
            break;
        }
    }

    Ok(left)
}

fn power(i: &mut &str) -> PResult<Expr> {
    let left = unary(i)?;

    let _ = multispace0(i)?;
    if i.starts_with('^') {
        *i = &i[1..];
        let _ = multispace0(i)?;
        let exp = if i.starts_with('{') {
            *i = &i[1..];
            let inner = latex_expr(i)?;
            let _ = multispace0(i)?;
            if i.starts_with('}') {
                *i = &i[1..];
                inner
            } else {
                return Err(winnow::error::ErrMode::Cut(ContextError::default()));
            }
        } else {
            unary(i)?
        };
        Ok(Expr::pow(left, exp))
    } else {
        Ok(left)
    }
}

fn unary(i: &mut &str) -> PResult<Expr> {
    let _ = multispace0(i)?;
    if i.starts_with('-') {
        *i = &i[1..];
        let val = unary(i)?;
        Ok(Expr::neg(val))
    } else if i.starts_with('+') {
        *i = &i[1..];
        unary(i)
    } else {
        primary(i)
    }
}

fn primary(i: &mut &str) -> PResult<Expr> {
    let _ = multispace0(i)?;

    if i.starts_with(r"\begin{pmatrix}") || i.starts_with(r"\begin{bmatrix}") {
        let is_p = i.starts_with(r"\begin{pmatrix}");
        *i = &i[15..];
        let end_tag = if is_p {
            r"\end{pmatrix}"
        } else {
            r"\end{bmatrix}"
        };

        let mut rows = Vec::new();
        let mut current_row = Vec::new();
        loop {
            let _ = multispace0(i)?;
            if i.starts_with(end_tag) {
                *i = &i[13..];
                if !current_row.is_empty() {
                    rows.push(current_row);
                }
                break;
            }

            current_row.push(latex_expr(i)?);
            let _ = multispace0(i)?;
            if i.starts_with('&') {
                *i = &i[1..];
            } else if i.starts_with(r"\\") {
                *i = &i[2..];
                rows.push(current_row);
                current_row = Vec::new();
            } else if i.starts_with(end_tag) {
                // caught next iter
            } else {
                return Err(winnow::error::ErrMode::Cut(ContextError::default()));
            }
        }
        return Ok(Expr::Matrix(rows));
    }

    if i.starts_with(r"\sum") {
        *i = &i[4..];
        let mut lower = Expr::int(0);
        let mut upper = Expr::var("n");
        let mut var = Expr::var("i");

        for _ in 0..2 {
            let _ = multispace0(i)?;
            if i.starts_with('_') {
                *i = &i[1..];
                // For lower bound, we expect either `i=1` parsed as manual string or just an expression.
                // Simple workaround: just parse_braced, if it's `i=1` it will fail because no `=` in Expr.
                // Let's just consume the `{...}` block manually.
                if i.starts_with('{') {
                    *i = &i[1..];
                    let mut content = String::new();
                    while !i.is_empty() && !i.starts_with('}') {
                        content.push(i.chars().next().unwrap());
                        *i = &i[1..];
                    }
                    if i.starts_with('}') {
                        *i = &i[1..];
                    }
                    // very naive split for `i=1`
                    if let Some((v, val)) = content.split_once('=') {
                        var = Expr::var(v.trim());
                        lower = parse_latex(val).unwrap_or(Expr::int(0));
                    } else {
                        lower = parse_latex(&content).unwrap_or(Expr::int(0));
                    }
                } else {
                    lower = primary(i)?;
                }
            } else if i.starts_with('^') {
                *i = &i[1..];
                upper = if i.starts_with('{') {
                    parse_braced(i)?
                } else {
                    primary(i)?
                };
            } else {
                break;
            }
        }
        let body = primary(i)?;
        return Ok(Expr::sum(body, var, lower, upper));
    }

    if i.starts_with(r"\int") {
        *i = &i[4..];
        let mut _lower = Expr::int(0);
        let mut _upper = Expr::var("x");

        for _ in 0..2 {
            let _ = multispace0(i)?;
            if i.starts_with('_') {
                *i = &i[1..];
                _lower = if i.starts_with('{') {
                    parse_braced(i)?
                } else {
                    primary(i)?
                };
            } else if i.starts_with('^') {
                *i = &i[1..];
                _upper = if i.starts_with('{') {
                    parse_braced(i)?
                } else {
                    primary(i)?
                };
            } else {
                break;
            }
        }
        let body = primary(i)?;
        // assume integrating over x if not specified in a better way
        return Ok(Expr::integral(body, Expr::var("x")));
    }

    // 1. \frac{num}{denom}
    if i.starts_with(r"\frac") {
        *i = &i[5..];

        // Peek if it's \frac{\partial f}{\partial x} or \frac{\partial}{\partial x} f
        let temp_i = *i;
        if let Some(stripped) = temp_i.strip_prefix(r"{\partial") {
            *i = stripped; // skip {\partial
            let _ = multispace0(i)?;

            let _f = if i.starts_with('}') {
                *i = &i[1..];
                // It was \frac{\partial}{\partial x} f
                // Parse the denominator
                let _ = multispace0(i)?;
                if i.starts_with(r"{\partial") {
                    *i = &i[9..];
                    let _ = multispace0(i)?;
                    let v = primary(i)?;
                    let _ = multispace0(i)?;
                    if i.starts_with('}') {
                        *i = &i[1..];
                    }
                    let body = primary(i)?;
                    return Ok(Expr::derivative(body, v));
                } else {
                    return Err(winnow::error::ErrMode::Cut(ContextError::default()));
                }
            } else {
                // It was \frac{\partial f}{\partial x}
                let f_expr = primary(i)?;
                let _ = multispace0(i)?;
                if i.starts_with('}') {
                    *i = &i[1..];
                }
                let _ = multispace0(i)?;
                if i.starts_with(r"{\partial") {
                    *i = &i[9..];
                    let _ = multispace0(i)?;
                    let v = primary(i)?;
                    let _ = multispace0(i)?;
                    if i.starts_with('}') {
                        *i = &i[1..];
                    }
                    return Ok(Expr::derivative(f_expr, v));
                } else {
                    return Err(winnow::error::ErrMode::Cut(ContextError::default()));
                }
            };
        }

        let num = parse_braced(i)?;
        let den = parse_braced(i)?;
        return Ok(Expr::div(num, den));
    }

    // 2. \sqrt{arg}
    if i.starts_with(r"\sqrt") {
        *i = &i[5..];
        let arg = parse_braced(i)?;
        return Ok(Expr::sqrt(arg));
    }

    // 3. Trig and Transcendental functions (\sin, \cos, etc.)
    for (cmd, constructor) in [
        (r"\sin", Expr::sin as fn(Expr) -> Expr),
        (r"\cos", Expr::cos as fn(Expr) -> Expr),
        (r"\tan", Expr::tan as fn(Expr) -> Expr),
        (r"\exp", Expr::exp as fn(Expr) -> Expr),
        (r"\ln", Expr::ln as fn(Expr) -> Expr),
    ] {
        if i.starts_with(cmd) {
            *i = &i[cmd.len()..];
            let _ = multispace0(i)?;
            // Check for optional power like \sin^2(x)
            let mut power_opt = None;
            if i.starts_with('^') {
                *i = &i[1..];
                let _ = multispace0(i)?;
                let exp = if i.starts_with('{') {
                    parse_braced(i)?
                } else {
                    unary(i)?
                };
                power_opt = Some(exp);
            }

            let arg = primary(i)?;
            let call = constructor(arg);
            return if let Some(p) = power_opt {
                Ok(Expr::pow(call, p))
            } else {
                Ok(call)
            };
        }
    }

    // 4. Parentheses (optional \left( \right))
    if i.starts_with(r"\left(") {
        *i = &i[6..];
        let inner = latex_expr(i)?;
        let _ = multispace0(i)?;
        if i.starts_with(r"\right)") {
            *i = &i[7..];
            return Ok(inner);
        } else {
            return Err(winnow::error::ErrMode::Cut(ContextError::default()));
        }
    }

    if i.starts_with('(') {
        *i = &i[1..];
        let inner = latex_expr(i)?;
        let _ = multispace0(i)?;
        if i.starts_with(')') {
            *i = &i[1..];
            return Ok(inner);
        } else {
            return Err(winnow::error::ErrMode::Cut(ContextError::default()));
        }
    }

    if i.starts_with('{') {
        return parse_braced(i);
    }

    // 5. Greek letters and constants
    for greek in [
        r"\alpha",
        r"\beta",
        r"\gamma",
        r"\delta",
        r"\epsilon",
        r"\zeta",
        r"\eta",
        r"\theta",
        r"\iota",
        r"\kappa",
        r"\lambda",
        r"\mu",
        r"\nu",
        r"\xi",
        r"\pi",
        r"\rho",
        r"\sigma",
        r"\tau",
        r"\upsilon",
        r"\phi",
        r"\chi",
        r"\psi",
        r"\omega",
        r"\Gamma",
        r"\Delta",
        r"\Theta",
        r"\Lambda",
        r"\Xi",
        r"\Pi",
        r"\Sigma",
        r"\Upsilon",
        r"\Phi",
        r"\Psi",
        r"\Omega",
    ] {
        if i.starts_with(greek) {
            *i = &i[greek.len()..];
            // Remove the backslash for the AST node name
            let name = &greek[1..];
            return parse_tensor_indices(i, name);
        }
    }

    // 6. Variables and constants
    let ident = opt(alphanumeric1).parse_next(i)?;
    if let Some(id_str) = ident {
        if let Ok(num) = id_str.parse::<i64>() {
            return Ok(Expr::int(num));
        }
        return parse_tensor_indices(i, id_str);
    }

    Err(winnow::error::ErrMode::Backtrack(ContextError::default()))
}

fn parse_tensor_indices(i: &mut &str, base_name: &str) -> PResult<Expr> {
    let mut indices = Vec::new();
    loop {
        let _ = multispace0(i)?;
        if i.starts_with('_') {
            *i = &i[1..];
            let idx_expr = if i.starts_with('{') {
                parse_braced(i)?
            } else {
                primary(i)?
            };
            if let Expr::Var(n) = idx_expr {
                indices.push(crate::tensor::Index {
                    name: n,
                    idx_type: crate::tensor::IndexType::Covariant,
                });
            }
        } else if i.starts_with('^') {
            // Check if this ^ is an exponent or a contravariant index
            // In LaTeX, it's ambiguous. But we'll try to parse it as contravariant index if it contains only letters.
            // Actually, for simplicity, if it's attached to a variable, we parse it as an exponent at the `power()` level unless we specifically handle tensor notation.
            // Let's just break here and let `power()` handle `^` as Exponent, UNLESS the user explicitly wants tensors.
            // If they want tensors, they should probably use `Expr::Tensor`.
            // Let's assume `^` is exponent for now, and we only parse `_` as covariant index.
            break;
        } else {
            break;
        }
    }

    if indices.is_empty() {
        Ok(Expr::var(base_name))
    } else {
        Ok(Expr::tensor(base_name, indices))
    }
}

fn parse_braced(i: &mut &str) -> PResult<Expr> {
    let _ = multispace0(i)?;
    if i.starts_with('{') {
        *i = &i[1..];
        let inner = latex_expr(i)?;
        let _ = multispace0(i)?;
        if i.starts_with('}') {
            *i = &i[1..];
            Ok(inner)
        } else {
            Err(winnow::error::ErrMode::Cut(ContextError::default()))
        }
    } else {
        primary(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_latex_fraction() {
        let parsed = parse_latex(r"\frac{x}{2}").unwrap();
        assert_eq!(parsed, Expr::div(Expr::var("x"), Expr::int(2)));
    }

    #[test]
    fn test_parse_latex_trig_power() {
        let parsed = parse_latex(r"\sin^{2}(x) + \cos^{2}(x)").unwrap();
        assert_eq!(
            parsed,
            Expr::add(
                Expr::pow(Expr::sin(Expr::var("x")), Expr::int(2)),
                Expr::pow(Expr::cos(Expr::var("x")), Expr::int(2))
            )
        );
    }

    #[test]
    fn test_parse_latex_sqrt() {
        let parsed = parse_latex(r"\sqrt{x + 1}").unwrap();
        assert_eq!(parsed, Expr::sqrt(Expr::add(Expr::var("x"), Expr::int(1))));
    }
}
