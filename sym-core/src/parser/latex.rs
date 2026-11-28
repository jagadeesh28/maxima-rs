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
use crate::parser::{ParseError, RecursionDepthGuard};

/// Parse a LaTeX math string into an [`Expr`].
pub fn parse_latex(input: &str) -> Result<Expr, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyInput);
    }
    crate::parser::check_nesting_limit(trimmed, RecursionDepthGuard::DEFAULT_MAX_DEPTH)?;

    let mut guard = RecursionDepthGuard::new();
    let mut state = trimmed;
    match latex_expr(&mut state, &mut guard) {
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
        Err(_) if guard.exceeded => Err(ParseError::RecursionLimitExceeded),
        Err(e) => Err(ParseError::Syntax {
            message: format!("{e}"),
            remaining: state.to_string(),
        }),
    }
}

fn latex_expr(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
    guard
        .enter()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::default()))?;
    let res = additive(i, guard);
    guard.leave();
    res
}

fn additive(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
    let mut left = multiplicative(i, guard)?;

    loop {
        let _ = multispace0(i)?;
        if i.starts_with('+') {
            *i = &i[1..];
            let right = multiplicative(i, guard)?;
            left = Expr::add(left, right);
        } else if i.starts_with('-') {
            *i = &i[1..];
            let right = multiplicative(i, guard)?;
            left = Expr::sub(left, right);
        } else {
            break;
        }
    }

    Ok(left)
}

fn multiplicative(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
    let mut left = unary(i, guard)?;

    loop {
        let _ = multispace0(i)?;
        if i.starts_with(r"\cdot") && !i[5..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            *i = &i[5..];
            let right = unary(i, guard)?;
            left = Expr::mul(left, right);
        } else if i.starts_with(r"\times") && !i[6..].starts_with(|c: char| c.is_ascii_alphabetic())
        {
            *i = &i[6..];
            let right = unary(i, guard)?;
            left = Expr::mul(left, right);
        } else if i.starts_with(r"\,") {
            *i = &i[2..];
            let right = unary(i, guard)?;
            left = Expr::mul(left, right);
        } else if i.starts_with('*') {
            *i = &i[1..];
            let right = unary(i, guard)?;
            left = Expr::mul(left, right);
        } else if i.starts_with('/') {
            *i = &i[1..];
            let right = unary(i, guard)?;
            left = Expr::div(left, right);
        } else {
            break;
        }
    }

    Ok(left)
}

fn unary(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
    let _ = multispace0(i)?;
    if i.starts_with('-') {
        guard
            .enter()
            .map_err(|_| winnow::error::ErrMode::Cut(ContextError::default()))?;
        *i = &i[1..];
        let val = unary(i, guard);
        guard.leave();
        let val = val?;
        Ok(Expr::neg(val))
    } else if i.starts_with('+') {
        guard
            .enter()
            .map_err(|_| winnow::error::ErrMode::Cut(ContextError::default()))?;
        *i = &i[1..];
        let val = unary(i, guard);
        guard.leave();
        val
    } else {
        power(i, guard)
    }
}

fn power(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
    guard
        .enter()
        .map_err(|_| winnow::error::ErrMode::Cut(ContextError::default()))?;
    let res = (|| {
        let left = primary(i, guard)?;

        let _ = multispace0(i)?;
        if i.starts_with('^') {
            *i = &i[1..];
            let _ = multispace0(i)?;
            let exp = if i.starts_with('{') {
                *i = &i[1..];
                let inner = latex_expr(i, guard)?;
                let _ = multispace0(i)?;
                if i.starts_with('}') {
                    *i = &i[1..];
                    inner
                } else {
                    return Err(winnow::error::ErrMode::Cut(ContextError::default()));
                }
            } else {
                unary(i, guard)?
            };
            Ok(Expr::pow(left, exp))
        } else {
            Ok(left)
        }
    })();
    guard.leave();
    res
}

fn take_number<'a>(i: &mut &'a str) -> Option<&'a str> {
    let bytes = i.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return None;
    }
    let mut n = 0;
    while n < bytes.len() && bytes[n].is_ascii_digit() {
        n += 1;
    }
    if n < bytes.len() && bytes[n] == b'.' && n + 1 < bytes.len() && bytes[n + 1].is_ascii_digit() {
        n += 2;
        while n < bytes.len() && bytes[n].is_ascii_digit() {
            n += 1;
        }
    }
    if n < bytes.len() && (bytes[n] == b'e' || bytes[n] == b'E') {
        let mut exp_pos = n + 1;
        if exp_pos < bytes.len() && (bytes[exp_pos] == b'+' || bytes[exp_pos] == b'-') {
            exp_pos += 1;
        }
        if exp_pos < bytes.len() && bytes[exp_pos].is_ascii_digit() {
            n = exp_pos + 1;
            while n < bytes.len() && bytes[n].is_ascii_digit() {
                n += 1;
            }
        }
    }
    let (head, tail) = i.split_at(n);
    *i = tail;
    Some(head)
}

fn parse_number_to_expr(s: &str) -> Option<Expr> {
    if let Ok(num) = s.parse::<i64>() {
        return Some(Expr::int(num));
    }
    crate::parser::parse_exact_rational(s).map(Expr::Const)
}

fn primary(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
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

            current_row.push(latex_expr(i, guard)?);
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
                if i.starts_with('{') {
                    *i = &i[1..];
                    let mut content = String::new();
                    let mut brace_depth = 1usize;
                    while let Some(ch) = i.chars().next() {
                        *i = &i[ch.len_utf8()..];
                        if ch == '{' {
                            brace_depth += 1;
                            content.push(ch);
                        } else if ch == '}' {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                break;
                            }
                            content.push(ch);
                        } else {
                            content.push(ch);
                        }
                    }
                    if brace_depth != 0 {
                        return Err(winnow::error::ErrMode::Cut(ContextError::default()));
                    }
                    if let Some((v, val)) = content.split_once('=') {
                        var = Expr::var(v.trim());
                        lower = parse_latex(val).unwrap_or(Expr::int(0));
                    } else {
                        lower = parse_latex(&content).unwrap_or(Expr::int(0));
                    }
                } else {
                    lower = primary(i, guard)?;
                }
            } else if i.starts_with('^') {
                *i = &i[1..];
                upper = if i.starts_with('{') {
                    parse_braced(i, guard)?
                } else {
                    primary(i, guard)?
                };
            } else {
                break;
            }
        }
        let body = primary(i, guard)?;
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
                    parse_braced(i, guard)?
                } else {
                    primary(i, guard)?
                };
            } else if i.starts_with('^') {
                *i = &i[1..];
                _upper = if i.starts_with('{') {
                    parse_braced(i, guard)?
                } else {
                    primary(i, guard)?
                };
            } else {
                break;
            }
        }
        let body = primary(i, guard)?;
        let _ = multispace0(i)?;
        let mut int_var = Expr::var("x");
        if i.starts_with(r"\,") {
            let after_sp = i[2..].trim_start();
            if let Some(rest) = after_sp.strip_prefix('d') {
                let v_name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !v_name.is_empty() {
                    int_var = Expr::var(&v_name);
                    *i = &after_sp[1 + v_name.len()..];
                }
            }
        }
        return Ok(Expr::integral(body, int_var));
    }

    // 1. \frac{num}{denom}
    if i.starts_with(r"\frac") {
        *i = &i[5..];

        let temp_i = *i;
        if let Some(stripped) = temp_i.strip_prefix(r"{\partial") {
            *i = stripped;
            let _ = multispace0(i)?;

            let _f = if i.starts_with('}') {
                *i = &i[1..];
                let _ = multispace0(i)?;
                if i.starts_with(r"{\partial") {
                    *i = &i[9..];
                    let _ = multispace0(i)?;
                    let v = primary(i, guard)?;
                    let _ = multispace0(i)?;
                    if i.starts_with('}') {
                        *i = &i[1..];
                    }
                    let body = primary(i, guard)?;
                    return Ok(Expr::derivative(body, v));
                } else {
                    return Err(winnow::error::ErrMode::Cut(ContextError::default()));
                }
            } else {
                let f_expr = primary(i, guard)?;
                let _ = multispace0(i)?;
                if i.starts_with('}') {
                    *i = &i[1..];
                }
                let _ = multispace0(i)?;
                if i.starts_with(r"{\partial") {
                    *i = &i[9..];
                    let _ = multispace0(i)?;
                    let v = primary(i, guard)?;
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

        let num = parse_braced(i, guard)?;
        let den = parse_braced(i, guard)?;
        return Ok(Expr::div(num, den));
    }

    // 2. \sqrt{arg}
    if i.starts_with(r"\sqrt") {
        *i = &i[5..];
        let arg = parse_braced(i, guard)?;
        return Ok(Expr::sqrt(arg));
    }

    // 3. Trig and Transcendental functions (\sin, \cos, \sinh, etc.)
    type TrigConstructor = fn(Expr) -> Expr;
    static TRIG_FUNCS: &[(&str, TrigConstructor)] = &[
        (r"\sinh", Expr::sinh as fn(Expr) -> Expr),
        (r"\cosh", Expr::cosh as fn(Expr) -> Expr),
        (r"\tanh", Expr::tanh as fn(Expr) -> Expr),
        (r"\arcsin", Expr::asin as fn(Expr) -> Expr),
        (r"\arccos", Expr::acos as fn(Expr) -> Expr),
        (r"\arctan", Expr::atan as fn(Expr) -> Expr),
        (r"\asin", Expr::asin as fn(Expr) -> Expr),
        (r"\acos", Expr::acos as fn(Expr) -> Expr),
        (r"\atan", Expr::atan as fn(Expr) -> Expr),
        (r"\sin", Expr::sin as fn(Expr) -> Expr),
        (r"\cos", Expr::cos as fn(Expr) -> Expr),
        (r"\tan", Expr::tan as fn(Expr) -> Expr),
        (r"\exp", Expr::exp as fn(Expr) -> Expr),
        (r"\ln", Expr::ln as fn(Expr) -> Expr),
    ];
    for &(cmd, constructor) in TRIG_FUNCS {
        if i.starts_with(cmd) && !i[cmd.len()..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            *i = &i[cmd.len()..];
            let _ = multispace0(i)?;
            let mut power_opt = None;
            if i.starts_with('^') {
                *i = &i[1..];
                let _ = multispace0(i)?;
                let exp = if i.starts_with('{') {
                    parse_braced(i, guard)?
                } else {
                    unary(i, guard)?
                };
                power_opt = Some(exp);
            }

            let arg = primary(i, guard)?;
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
        let inner = latex_expr(i, guard)?;
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
        let inner = latex_expr(i, guard)?;
        let _ = multispace0(i)?;
        if i.starts_with(')') {
            *i = &i[1..];
            return Ok(inner);
        } else {
            return Err(winnow::error::ErrMode::Cut(ContextError::default()));
        }
    }

    if i.starts_with('{') {
        return parse_braced(i, guard);
    }

    // 5. Greek letters and constants
    static GREEK_LETTERS: &[&str] = &[
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
    ];
    for greek in GREEK_LETTERS {
        if i.starts_with(greek) && !i[greek.len()..].starts_with(|c: char| c.is_ascii_alphabetic())
        {
            *i = &i[greek.len()..];
            let name = &greek[1..];
            if name == "pi" && !i.trim_start().starts_with('_') {
                return Ok(Expr::Pi);
            }
            return parse_tensor_indices(i, name, guard);
        }
    }

    // 6. Numbers (integers, decimals, scientific notation)
    if let Some(num_str) = take_number(i) {
        if let Some(num_expr) = parse_number_to_expr(num_str) {
            return Ok(num_expr);
        } else {
            return Err(winnow::error::ErrMode::Cut(ContextError::default()));
        }
    }

    // 7. Variables and constants
    let ident = opt(alphanumeric1).parse_next(i)?;
    if let Some(id_str) = ident {
        return parse_tensor_indices(i, id_str, guard);
    }

    Err(winnow::error::ErrMode::Backtrack(ContextError::default()))
}

fn parse_tensor_indices(
    i: &mut &str,
    base_name: &str,
    guard: &mut RecursionDepthGuard,
) -> PResult<Expr> {
    let mut indices = Vec::new();
    loop {
        let _ = multispace0(i)?;
        if i.starts_with('_') {
            *i = &i[1..];
            let idx_expr = if i.starts_with('{') {
                parse_braced(i, guard)?
            } else {
                primary(i, guard)?
            };
            match idx_expr {
                Expr::Var(n) => {
                    indices.push(crate::tensor::Index {
                        name: n,
                        idx_type: crate::tensor::IndexType::Covariant,
                    });
                }
                Expr::Const(c) => {
                    indices.push(crate::tensor::Index {
                        name: c.to_string(),
                        idx_type: crate::tensor::IndexType::Covariant,
                    });
                }
                _ => {}
            }
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

fn parse_braced(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
    let _ = multispace0(i)?;
    if i.starts_with('{') {
        *i = &i[1..];
        let inner = latex_expr(i, guard)?;
        let _ = multispace0(i)?;
        if i.starts_with('}') {
            *i = &i[1..];
            Ok(inner)
        } else {
            Err(winnow::error::ErrMode::Cut(ContextError::default()))
        }
    } else {
        primary(i, guard)
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

    #[test]
    fn test_parse_latex_precedence() {
        let parsed = parse_latex("-x^2").unwrap();
        assert_eq!(parsed, Expr::neg(Expr::pow(Expr::var("x"), Expr::int(2))));
        let mut env = std::collections::HashMap::new();
        env.insert("x".to_string(), 3.0);
        assert_eq!(parsed.eval(&env), Some(-9.0));
    }

    #[test]
    fn test_parse_latex_scientific() {
        let parsed = parse_latex("1e-3").unwrap();
        assert_eq!(parsed.eval(&Default::default()), Some(0.001));
    }

    #[test]
    fn test_parse_latex_recursion_guard() {
        let deep_expr = format!("{}x{}", "(".repeat(300), ")".repeat(300));
        let res = parse_latex(&deep_expr);
        assert_eq!(res, Err(ParseError::RecursionLimitExceeded));

        let deep_pow = vec!["2"; 1000].join("^");
        assert_eq!(
            parse_latex(&deep_pow),
            Err(ParseError::RecursionLimitExceeded)
        );

        // UTF-8 sum, \pi -> Expr::Pi, word boundary \sinc, x_{0} subscript
        assert_eq!(parse_latex(r"\pi").unwrap(), Expr::Pi);
        assert!(parse_latex(r"\sinc(x)").is_err());
        assert!(parse_latex(r"\sum_{i=\alpha}^{n} i").is_ok());
        assert!(matches!(parse_latex("x_{0}").unwrap(), Expr::Tensor(_, _)));
    }
}
