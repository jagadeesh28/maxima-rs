//! Maxima expression string parser.
//!
//! Parses standard Maxima mathematical syntax:
//! - Arithmetic: `+`, `-`, `*`, `/`, `^`
//! - Functions: `sin`, `cos`, `tan`, `exp`, `ln`, `sqrt`
//! - Variables, numbers, and parentheses.

use winnow::ModalResult;
use winnow::ascii::{alphanumeric1, multispace0};
use winnow::combinator::opt;
use winnow::error::ContextError;
use winnow::prelude::*;

type PResult<O> = ModalResult<O>;

use crate::expr::Expr;
use crate::parser::ParseError;

/// Parse a Maxima expression string into an [`Expr`].
pub fn parse_maxima(input: &str) -> Result<Expr, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let mut state = trimmed;
    match expr(&mut state) {
        Ok(res) => {
            let rem = state.trim();
            if !rem.is_empty() {
                Err(ParseError::Syntax {
                    message: "Unexpected remaining input".to_string(),
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

// ── Recursive Descent with Precedence Climbing ─────────────────────

fn expr(i: &mut &str) -> PResult<Expr> {
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
        if i.starts_with('*') {
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
        let right = power(i)?; // right-associative
        Ok(Expr::pow(left, right))
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

    // 1. Parenthesized expression
    if i.starts_with('(') {
        *i = &i[1..];
        let inner = expr(i)?;
        let _ = multispace0(i)?;
        if i.starts_with(')') {
            *i = &i[1..];
            return Ok(inner);
        } else {
            return Err(winnow::error::ErrMode::Cut(ContextError::default()));
        }
    }

    // 2. Builtin function calls or identifier
    let ident = opt(alphanumeric1).parse_next(i)?;
    if let Some(id_str) = ident {
        let _ = multispace0(i)?;
        if i.starts_with('(') {
            *i = &i[1..];
            let arg = expr(i)?;
            let _ = multispace0(i)?;
            if i.starts_with(')') {
                *i = &i[1..];
                return match id_str {
                    "sin" => Ok(Expr::sin(arg)),
                    "cos" => Ok(Expr::cos(arg)),
                    "tan" => Ok(Expr::tan(arg)),
                    "exp" => Ok(Expr::exp(arg)),
                    "ln" | "log" => Ok(Expr::ln(arg)),
                    "sqrt" => Ok(Expr::sqrt(arg)),
                    _ => Err(winnow::error::ErrMode::Cut(ContextError::default())),
                };
            } else {
                return Err(winnow::error::ErrMode::Cut(ContextError::default()));
            }
        } else {
            // Check if it's an integer number
            if let Ok(num) = id_str.parse::<i64>() {
                return Ok(Expr::int(num));
            }
            // Otherwise it's a variable
            return Ok(Expr::var(id_str));
        }
    }

    Err(winnow::error::ErrMode::Backtrack(ContextError::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_arithmetic() {
        let parsed = parse_maxima("x + 1").unwrap();
        assert_eq!(parsed, Expr::add(Expr::var("x"), Expr::int(1)));
    }

    #[test]
    fn test_parse_multiplication_precedence() {
        let parsed = parse_maxima("x + 2 * y").unwrap();
        assert_eq!(
            parsed,
            Expr::add(Expr::var("x"), Expr::mul(Expr::int(2), Expr::var("y")))
        );
    }

    #[test]
    fn test_parse_functions() {
        let parsed = parse_maxima("sin(x)^2 + cos(x)^2").unwrap();
        assert_eq!(
            parsed,
            Expr::add(
                Expr::pow(Expr::sin(Expr::var("x")), Expr::int(2)),
                Expr::pow(Expr::cos(Expr::var("x")), Expr::int(2))
            )
        );
    }

    #[test]
    fn test_parse_nested() {
        let parsed = parse_maxima("exp(x + ln(y))").unwrap();
        assert_eq!(
            parsed,
            Expr::exp(Expr::add(Expr::var("x"), Expr::ln(Expr::var("y"))))
        );
    }
}
