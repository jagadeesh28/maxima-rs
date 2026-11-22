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
#[cfg(feature = "cas")]
mod cas_commands {
    use super::parse_maxima;
    use crate::expr::Expr;
    use crate::integrate::defint::{DefiniteResult, defint};
    use crate::parser::ParseError;

    /// A parsed Maxima CAS top-level command.
    #[derive(Debug, Clone, PartialEq)]
    pub enum CasCommand {
        Expand(Expr),
        Factor(Expr),
        Ratsimp(Expr),
        Partfrac(Expr, String),
        Limit(Expr, String, Expr),
        Solve(Expr, String),
        Ode2(Expr, String, String),
        Integrate(Expr, String),
        Defint(Expr, String, Expr, Expr),
        Taylor(Expr, String, Expr, i64),
    }

    fn split_top_level(s: &str) -> Vec<&str> {
        let mut depth = 0i32;
        let mut start = 0usize;
        let mut out = Vec::new();
        for (i, ch) in s.char_indices() {
            match ch {
                '(' | '[' => depth += 1,
                ')' | ']' => depth -= 1,
                ',' if depth == 0 => {
                    out.push(s[start..i].trim());
                    start = i + 1;
                }
                _ => {}
            }
        }
        out.push(s[start..].trim());
        out.retain(|x| !x.is_empty());
        out
    }

    fn parse_point(s: &str) -> Result<Expr, ParseError> {
        match s.trim() {
            "inf" => Ok(Expr::var("inf")),
            "minf" => Ok(Expr::var("minf")),
            other => parse_maxima(other),
        }
    }

    /// Parse a Maxima CAS command such as `expand((x + y)^2)`.
    pub fn parse_maxima_cas(input: &str) -> Result<CasCommand, ParseError> {
        let s = input.trim();
        let err = |m: &str| ParseError::Syntax {
            message: m.to_string(),
            remaining: s.to_string(),
        };
        let open = s.find('(').ok_or_else(|| err("expected command call"))?;
        let close = s.rfind(')').ok_or_else(|| err("missing ')'"))?;
        if close < open {
            return Err(err("malformed command"));
        }
        let name = &s[..open];
        let args = split_top_level(&s[open + 1..close]);
        let need = |n: usize| -> Result<(), ParseError> {
            if args.len() == n {
                Ok(())
            } else {
                Err(err("wrong argument count"))
            }
        };
        let e = |i: usize| parse_maxima(args[i]);
        let v = |i: usize| args[i].to_string();
        match name {
            "expand" => {
                need(1)?;
                Ok(CasCommand::Expand(e(0)?))
            }
            "factor" => {
                need(1)?;
                Ok(CasCommand::Factor(e(0)?))
            }
            "ratsimp" => {
                need(1)?;
                Ok(CasCommand::Ratsimp(e(0)?))
            }
            "partfrac" => {
                need(2)?;
                Ok(CasCommand::Partfrac(e(0)?, v(1)))
            }
            "limit" => {
                need(3)?;
                Ok(CasCommand::Limit(e(0)?, v(1), parse_point(args[2])?))
            }
            "solve" => {
                need(2)?;
                Ok(CasCommand::Solve(e(0)?, v(1)))
            }
            "ode2" => {
                need(3)?;
                Ok(CasCommand::Ode2(e(0)?, v(1), v(2)))
            }
            "integrate" => {
                need(2)?;
                Ok(CasCommand::Integrate(e(0)?, v(1)))
            }
            "defint" => {
                need(4)?;
                Ok(CasCommand::Defint(e(0)?, v(1), e(2)?, e(3)?))
            }
            "taylor" => {
                need(4)?;
                let order = args[3]
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| err("bad taylor order"))?;
                Ok(CasCommand::Taylor(e(0)?, v(1), e(2)?, order))
            }
            _ => Err(err("unknown CAS command")),
        }
    }

    /// Execute a parsed CAS command, returning the resulting expression.
    pub fn execute_cas(cmd: &CasCommand) -> Option<Expr> {
        match cmd {
            CasCommand::Expand(e) => Some(crate::poly::expand(e)),
            CasCommand::Factor(e) => Some(crate::poly::factor(e)),
            CasCommand::Ratsimp(e) => crate::poly::ratsimp(e).ok(),
            CasCommand::Partfrac(e, v) => crate::poly::partfrac(e, v).ok(),
            CasCommand::Limit(e, v, p) => crate::limit::limit(e, v, p),
            CasCommand::Solve(e, v) => crate::solve::solve(e, v).0.into_iter().next(),
            CasCommand::Ode2(e, y, x) => crate::solve::ode2(e, y, x).map(|s| s.solution),
            CasCommand::Integrate(e, v) => Some(crate::integrate::integrate(e, v)),
            CasCommand::Defint(e, v, a, b) => match defint(e, v, a, b) {
                DefiniteResult::Value(x) => Some(x),
                _ => None,
            },
            CasCommand::Taylor(e, v, c, o) => crate::series::taylor(e, v, c, *o),
        }
    }
}

#[cfg(feature = "cas")]
pub use cas_commands::{CasCommand, execute_cas, parse_maxima_cas};
