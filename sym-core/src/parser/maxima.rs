//! Maxima expression string parser.
//!
//! Parses standard Maxima mathematical syntax:
//! - Arithmetic: `+`, `-`, `*`, `/`, `^`
//! - Functions: `sin`, `cos`, `tan`, `exp`, `ln`, `sqrt`
//! - Variables, numbers, and parentheses.

use winnow::ModalResult;
use winnow::ascii::multispace0;
use winnow::error::ContextError;

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
        postfix(i)
    }
}

/// Postfix factorial `!` — binds tighter than `*`, `/`, and `^`.
fn postfix(i: &mut &str) -> PResult<Expr> {
    let mut base = primary(i)?;
    loop {
        let _ = multispace0(i)?;
        if i.starts_with('!') {
            *i = &i[1..];
            base = Expr::factorial(base);
        } else {
            break;
        }
    }
    Ok(base)
}

/// Consume a Maxima identifier (`[A-Za-z0-9_]`).
fn take_ident<'a>(i: &mut &'a str) -> Option<&'a str> {
    let bytes = i.as_bytes();
    let mut n = 0;
    while n < bytes.len() && (bytes[n].is_ascii_alphanumeric() || bytes[n] == b'_') {
        n += 1;
    }
    if n == 0 {
        None
    } else {
        let (head, tail) = i.split_at(n);
        *i = tail;
        Some(head)
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

    // 2. Maxima constants: %pi, %e (other %-symbols stay symbolic)
    if i.starts_with('%') {
        *i = &i[1..];
        let name =
            take_ident(i).ok_or_else(|| winnow::error::ErrMode::Cut(ContextError::default()))?;
        return match name {
            "pi" => Ok(Expr::pi()),
            "e" => Ok(Expr::e()),
            other => Ok(Expr::var(&format!("%{other}"))),
        };
    }

    // 3. Builtin function calls, matrix literals, or identifiers
    if let Some(id_str) = take_ident(i) {
        let name = id_str.to_string();
        let _ = multispace0(i)?;
        if i.starts_with('(') {
            *i = &i[1..];
            if name == "matrix" {
                return Ok(Expr::matrix(matrix_rows(i)?));
            }
            let args = call_args(i)?;
            return build_call(&name, args);
        }
        // Check if it's an integer number
        if let Ok(num) = name.parse::<i64>() {
            return Ok(Expr::int(num));
        }
        // Otherwise it's a variable
        return Ok(Expr::var(&name));
    }

    Err(winnow::error::ErrMode::Backtrack(ContextError::default()))
}

fn cut<T>() -> PResult<T> {
    Err(winnow::error::ErrMode::Cut(ContextError::default()))
}

/// Parse a comma-separated function argument list; the opening `(` is already consumed.
fn call_args(i: &mut &str) -> PResult<Vec<Expr>> {
    let mut args = Vec::new();
    let _ = multispace0(i)?;
    if i.starts_with(')') {
        *i = &i[1..];
        return Ok(args);
    }
    loop {
        args.push(expr(i)?);
        let _ = multispace0(i)?;
        if i.starts_with(',') {
            *i = &i[1..];
        } else if i.starts_with(')') {
            *i = &i[1..];
            break;
        } else {
            return cut();
        }
    }
    Ok(args)
}

/// Parse `matrix([a,b],[c,d])` rows; the opening `(` is already consumed.
fn matrix_rows(i: &mut &str) -> PResult<Vec<Vec<Expr>>> {
    let mut rows = Vec::new();
    loop {
        let _ = multispace0(i)?;
        if i.starts_with(')') {
            *i = &i[1..];
            break;
        }
        if !i.starts_with('[') {
            return cut();
        }
        *i = &i[1..];
        let mut row = Vec::new();
        let _ = multispace0(i)?;
        if i.starts_with(']') {
            *i = &i[1..];
        } else {
            loop {
                row.push(expr(i)?);
                let _ = multispace0(i)?;
                if i.starts_with(',') {
                    *i = &i[1..];
                } else if i.starts_with(']') {
                    *i = &i[1..];
                    break;
                } else {
                    return cut();
                }
            }
        }
        rows.push(row);
        let _ = multispace0(i)?;
        if i.starts_with(',') {
            *i = &i[1..];
        } else if i.starts_with(')') {
            *i = &i[1..];
            break;
        } else {
            return cut();
        }
    }
    Ok(rows)
}

/// Dispatch a parsed function name with its arguments.
fn build_call(name: &str, args: Vec<Expr>) -> PResult<Expr> {
    let mut args = args;
    let arity_one = |f: fn(Expr) -> Expr, args: &mut Vec<Expr>| -> PResult<Expr> {
        if args.len() != 1 {
            return cut();
        }
        Ok(f(args.remove(0)))
    };
    match name {
        "sin" => arity_one(Expr::sin, &mut args),
        "cos" => arity_one(Expr::cos, &mut args),
        "tan" => arity_one(Expr::tan, &mut args),
        "exp" => arity_one(Expr::exp, &mut args),
        "ln" | "log" => arity_one(Expr::ln, &mut args),
        "sqrt" => arity_one(Expr::sqrt, &mut args),
        "sinh" => arity_one(Expr::sinh, &mut args),
        "cosh" => arity_one(Expr::cosh, &mut args),
        "tanh" => arity_one(Expr::tanh, &mut args),
        "asin" => arity_one(Expr::asin, &mut args),
        "acos" => arity_one(Expr::acos, &mut args),
        "atan" => arity_one(Expr::atan, &mut args),
        "diff" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::derivative(args.remove(0), args.remove(0)))
        }
        "integrate" | "integral" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::integral(args.remove(0), args.remove(0)))
        }
        "sum" => {
            if args.len() != 4 {
                return cut();
            }
            let body = args.remove(0);
            let var = args.remove(0);
            let lower = args.remove(0);
            let upper = args.remove(0);
            Ok(Expr::sum(body, var, lower, upper))
        }
        _ => cut(),
    }
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

    #[test]
    fn test_parse_constants() {
        assert_eq!(parse_maxima("%pi").unwrap(), Expr::pi());
        assert_eq!(parse_maxima("%e").unwrap(), Expr::e());
        assert_eq!(
            parse_maxima("2 * %pi").unwrap(),
            Expr::mul(Expr::int(2), Expr::pi())
        );
        // Other %-symbols stay symbolic variables.
        assert_eq!(parse_maxima("%i").unwrap(), Expr::var("%i"));
    }

    #[test]
    fn test_parse_factorial() {
        assert_eq!(parse_maxima("5!").unwrap(), Expr::factorial(Expr::int(5)));
        assert_eq!(
            parse_maxima("2 * x!").unwrap(),
            Expr::mul(Expr::int(2), Expr::factorial(Expr::var("x")))
        );
        // Factorial binds tighter than `^`.
        assert_eq!(
            parse_maxima("x!^2").unwrap(),
            Expr::pow(Expr::factorial(Expr::var("x")), Expr::int(2))
        );
    }

    #[test]
    fn test_parse_hyperbolic_and_inverse_trig() {
        assert_eq!(
            parse_maxima("sinh(x) + cosh(x) + tanh(x)").unwrap(),
            Expr::add(
                Expr::add(Expr::sinh(Expr::var("x")), Expr::cosh(Expr::var("x"))),
                Expr::tanh(Expr::var("x")),
            )
        );
        assert_eq!(
            parse_maxima("asin(x) + acos(x) + atan(x)").unwrap(),
            Expr::add(
                Expr::add(Expr::asin(Expr::var("x")), Expr::acos(Expr::var("x"))),
                Expr::atan(Expr::var("x")),
            )
        );
    }

    #[test]
    fn test_parse_calculus_forms() {
        assert_eq!(
            parse_maxima("diff(f, x)").unwrap(),
            Expr::derivative(Expr::var("f"), Expr::var("x"))
        );
        assert_eq!(
            parse_maxima("integrate(f, x)").unwrap(),
            Expr::integral(Expr::var("f"), Expr::var("x"))
        );
        assert_eq!(
            parse_maxima("sum(i, i, 1, n)").unwrap(),
            Expr::sum(Expr::var("i"), Expr::var("i"), Expr::int(1), Expr::var("n"))
        );
    }

    #[test]
    fn test_parse_matrix() {
        assert_eq!(
            parse_maxima("matrix([a, b], [c, d])").unwrap(),
            Expr::matrix(vec![
                vec![Expr::var("a"), Expr::var("b")],
                vec![Expr::var("c"), Expr::var("d")],
            ])
        );
    }

    #[test]
    fn test_parse_display_roundtrip() {
        for s in ["%pi/2", "sinh(x)^2 - cosh(x)^2", "5!", "exp(x) + ln(y)"] {
            let parsed = parse_maxima(s).unwrap();
            let reparsed = parse_maxima(&parsed.to_string());
            assert!(
                reparsed.is_ok(),
                "display output for {s} was not parseable: {parsed}"
            );
        }
    }
}
