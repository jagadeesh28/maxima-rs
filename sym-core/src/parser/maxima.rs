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
use crate::parser::{ParseError, RecursionDepthGuard};

/// Parse a Maxima expression string into an [`Expr`].
pub fn parse_maxima(input: &str) -> Result<Expr, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::EmptyInput);
    }
    crate::parser::check_nesting_limit(trimmed, RecursionDepthGuard::DEFAULT_MAX_DEPTH)?;

    let mut guard = RecursionDepthGuard::new();
    let mut state = trimmed;
    match expr(&mut state, &mut guard) {
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
        Err(_) if guard.exceeded => Err(ParseError::RecursionLimitExceeded),
        Err(e) => Err(ParseError::Syntax {
            message: format!("{e}"),
            remaining: state.to_string(),
        }),
    }
}

// ── Recursive Descent with Precedence Climbing ─────────────────────

fn expr(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
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
        if i.starts_with('*') {
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
        let left = postfix(i, guard)?;

        let _ = multispace0(i)?;
        if i.starts_with('^') {
            *i = &i[1..];
            let right = unary(i, guard)?; // right-associative, allows 2^-1 or x^-2
            Ok(Expr::pow(left, right))
        } else {
            Ok(left)
        }
    })();
    guard.leave();
    res
}

fn double_factorial(r: &num_rational::BigRational) -> Option<num_rational::BigRational> {
    use num_bigint::BigInt;
    use num_traits::{One, ToPrimitive};
    if !crate::rational::Rat::is_integer(r) {
        return None;
    }
    let n = r.numer().to_u32()?;
    if n > 200 {
        return None;
    }
    let mut acc = BigInt::one();
    let mut k = n;
    while k > 1 {
        acc *= BigInt::from(k);
        k -= 2;
    }
    Some(num_rational::BigRational::from_integer(acc))
}

/// Postfix factorial `!` and double factorial `!!` — binds tighter than `*`, `/`, and `^`.
fn postfix(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
    let mut base = primary(i, guard)?;
    loop {
        let _ = multispace0(i)?;
        if i.starts_with("!!") {
            *i = &i[2..];
            if let Expr::Const(ref r) = base
                && let Some(df) = double_factorial(r)
            {
                base = Expr::Const(df);
                continue;
            }
            base = Expr::factorial(Expr::factorial(base));
        } else if i.starts_with('!') {
            *i = &i[1..];
            base = Expr::factorial(base);
        } else {
            break;
        }
    }
    Ok(base)
}

/// Consume a number literal: integer, decimal (`2.5`), or scientific notation (`1e-3`, `2.5e+4`).
fn take_number<'a>(i: &mut &'a str) -> Option<&'a str> {
    let bytes = i.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return None;
    }
    let mut n = 0;
    while n < bytes.len() && bytes[n].is_ascii_digit() {
        n += 1;
    }
    // Check for decimal point followed by digit
    if n < bytes.len() && bytes[n] == b'.' && n + 1 < bytes.len() && bytes[n + 1].is_ascii_digit() {
        n += 2; // skip '.' and first digit
        while n < bytes.len() && bytes[n].is_ascii_digit() {
            n += 1;
        }
    }
    // Check for exponent 'e' or 'E'
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

/// Consume a Maxima identifier (`[A-Za-z_][A-Za-z0-9_]*`).
fn take_ident<'a>(i: &mut &'a str) -> Option<&'a str> {
    let bytes = i.as_bytes();
    if bytes.is_empty() || (!bytes[0].is_ascii_alphabetic() && bytes[0] != b'_') {
        return None;
    }
    let mut n = 1;
    while n < bytes.len() && (bytes[n].is_ascii_alphanumeric() || bytes[n] == b'_') {
        n += 1;
    }
    let (head, tail) = i.split_at(n);
    *i = tail;
    Some(head)
}

fn primary(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Expr> {
    let _ = multispace0(i)?;

    // 1. Parenthesized expression
    if i.starts_with('(') {
        *i = &i[1..];
        let inner = expr(i, guard)?;
        let _ = multispace0(i)?;
        if i.starts_with(')') {
            *i = &i[1..];
            return Ok(inner);
        } else {
            return Err(winnow::error::ErrMode::Cut(ContextError::default()));
        }
    }

    // 2. Maxima constants: %pi, %e, %i, % (last result), and other %-symbols
    if i.starts_with('%') {
        *i = &i[1..];
        let Some(name) = take_ident(i) else {
            return Ok(Expr::var("%"));
        };
        return match name {
            "pi" => Ok(Expr::pi()),
            "e" => Ok(Expr::e()),
            #[cfg(feature = "complex")]
            "i" => Ok(Expr::i()),
            other => Ok(Expr::var(&format!("%{other}"))),
        };
    }

    // 3. Numbers: integers, decimals, scientific notation
    if let Some(num_str) = take_number(i) {
        if let Some(num_expr) = parse_number_to_expr(num_str) {
            return Ok(num_expr);
        } else {
            return cut();
        }
    }

    // 4. Builtin function calls, matrix literals, or identifiers
    if let Some(id_str) = take_ident(i) {
        let name = id_str.to_string();
        let _ = multispace0(i)?;
        if i.starts_with('(') {
            *i = &i[1..];
            if name == "matrix" {
                return Ok(Expr::matrix(matrix_rows(i, guard)?));
            }
            let args = call_args(i, guard)?;
            return build_call(&name, args);
        }
        return Ok(Expr::var(&name));
    }

    Err(winnow::error::ErrMode::Backtrack(ContextError::default()))
}

fn cut<T>() -> PResult<T> {
    Err(winnow::error::ErrMode::Cut(ContextError::default()))
}

/// Parse a comma-separated function argument list; the opening `(` is already consumed.
fn call_args(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Vec<Expr>> {
    let mut args = Vec::new();
    let _ = multispace0(i)?;
    if i.starts_with(')') {
        *i = &i[1..];
        return Ok(args);
    }
    loop {
        args.push(expr(i, guard)?);
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
fn matrix_rows(i: &mut &str, guard: &mut RecursionDepthGuard) -> PResult<Vec<Vec<Expr>>> {
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
                row.push(expr(i, guard)?);
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
        "abs" => arity_one(Expr::abs, &mut args),
        "atan2" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::atan2(args.remove(0), args.remove(0)))
        }
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
        #[cfg(feature = "specfn")]
        "gamma" => {
            if args.len() != 1 {
                return cut();
            }
            Ok(Expr::gamma(args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "beta" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::beta(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "psi" => {
            if args.is_empty() || args.len() > 2 {
                return cut();
            }
            Ok(Expr::special_fn(crate::specfn::SpecialFnKind::Psi, args))
        }
        #[cfg(feature = "specfn")]
        "erf" => {
            if args.len() != 1 {
                return cut();
            }
            Ok(Expr::erf(args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "erfc" => {
            if args.len() != 1 {
                return cut();
            }
            Ok(Expr::erfc(args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "bessel_j" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::bessel_j(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "bessel_y" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::bessel_y(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "bessel_i" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::bessel_i(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "bessel_k" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::bessel_k(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "airy_ai" => {
            if args.len() != 1 {
                return cut();
            }
            Ok(Expr::airy_ai(args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "airy_dai" => {
            if args.len() != 1 {
                return cut();
            }
            Ok(Expr::special_fn(
                crate::specfn::SpecialFnKind::AiryAiPrime,
                args,
            ))
        }
        #[cfg(feature = "specfn")]
        "airy_bi" => {
            if args.len() != 1 {
                return cut();
            }
            Ok(Expr::airy_bi(args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "airy_dbi" => {
            if args.len() != 1 {
                return cut();
            }
            Ok(Expr::special_fn(
                crate::specfn::SpecialFnKind::AiryBiPrime,
                args,
            ))
        }
        #[cfg(feature = "specfn")]
        "legendre_p" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::legendre_p(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "chebyshev_t" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::chebyshev_t(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "chebyshev_u" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::chebyshev_u(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "hermite" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::hermite_h(args.remove(0), args.remove(0)))
        }
        #[cfg(feature = "specfn")]
        "laguerre" => {
            if args.len() != 2 {
                return cut();
            }
            Ok(Expr::laguerre_l(args.remove(0), args.remove(0)))
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
        // %i evaluates to Expr::i() when complex feature enabled, otherwise symbolic variable.
        #[cfg(feature = "complex")]
        assert_eq!(parse_maxima("%i").unwrap(), Expr::i());
        #[cfg(not(feature = "complex"))]
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

    #[test]
    fn test_operator_precedence_negative_power() {
        // -x^2 must parse as -(x^2)
        let parsed = parse_maxima("-x^2").unwrap();
        assert_eq!(parsed, Expr::neg(Expr::pow(Expr::var("x"), Expr::int(2))));
        let mut env = std::collections::HashMap::new();
        env.insert("x".to_string(), 3.0);
        assert_eq!(parsed.eval(&env), Some(-9.0));

        // -2^2 == -4
        let p2 = parse_maxima("-2^2").unwrap();
        assert_eq!(p2, Expr::neg(Expr::pow(Expr::int(2), Expr::int(2))));
        assert_eq!(p2.eval(&Default::default()), Some(-4.0));

        // 2^-1 == 0.5
        let p3 = parse_maxima("2^-1").unwrap();
        assert_eq!(p3, Expr::pow(Expr::int(2), Expr::neg(Expr::int(1))));
        assert_eq!(p3.eval(&Default::default()), Some(0.5));

        // -x! == -(x!)
        let p4 = parse_maxima("-x!").unwrap();
        assert_eq!(p4, Expr::neg(Expr::factorial(Expr::var("x"))));

        // (-x)^2 == (-x)^2
        let p5 = parse_maxima("(-x)^2").unwrap();
        assert_eq!(p5, Expr::pow(Expr::neg(Expr::var("x")), Expr::int(2)));
        assert_eq!(p5.eval(&env), Some(9.0));

        // exp(-x^2) == Exp(Neg(Pow(x, 2)))
        let p6 = parse_maxima("exp(-x^2)").unwrap();
        assert_eq!(
            p6,
            Expr::exp(Expr::neg(Expr::pow(Expr::var("x"), Expr::int(2))))
        );
    }

    #[test]
    fn test_scientific_notation() {
        let e1 = parse_maxima("1e-3").unwrap();
        assert_eq!(e1.eval(&Default::default()), Some(0.001));

        let e2 = parse_maxima("2.5e+4").unwrap();
        assert_eq!(e2.eval(&Default::default()), Some(25000.0));

        let e3 = parse_maxima("x + 1e-3").unwrap();
        let mut env = std::collections::HashMap::new();
        env.insert("x".to_string(), 1.0);
        assert!((e3.eval(&env).unwrap() - 1.001).abs() < 1e-10);
    }

    #[test]
    fn test_recursion_limit_guard() {
        let deep_expr = format!("{}x{}", "(".repeat(300), ")".repeat(300));
        let res = parse_maxima(&deep_expr);
        assert_eq!(res, Err(ParseError::RecursionLimitExceeded));

        // Power chain depth 1000 must return RecursionLimitExceeded instead of stack overflow
        let deep_pow = vec!["2"; 1000].join("^");
        let res_pow = parse_maxima(&deep_pow);
        assert_eq!(res_pow, Err(ParseError::RecursionLimitExceeded));

        // Double factorial 5!! == 15
        let df = parse_maxima("5!!").unwrap();
        assert_eq!(df.eval(&Default::default()), Some(15.0));

        // Huge exponent rejected
        assert!(parse_maxima("1e1000000000").is_err());
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
        Diff(Expr, String),
        Limit(Expr, String, Expr),
        Solve(Expr, String),
        Ode2(Expr, String, String),
        Integrate(Expr, String),
        Defint(Expr, String, Expr, Expr),
        Taylor(Expr, String, Expr, i64),
        Laplace(Expr, String, String),
        Ilt(Expr, String, String),
        Ic1(Expr, String, Expr, Expr),
        Ic2(Expr, String, Expr, Expr, Expr),
        Bc2(Expr, String, Expr, Expr, Expr, Expr),
        Determinant(Expr),
        Invert(Expr),
        Charpoly(Expr, String),
        Trace(Expr),
        Eigenvalues(Expr),
        Adjoint(Expr),
        Transpose(Expr),
        Rectform(Expr),
        Polarform(Expr),
        Realpart(Expr),
        Imagpart(Expr),
        Conjugate(Expr),
        Carg(Expr),
        Cabs(Expr),
        Bfloat(Expr, usize),
        Csign(Expr),
        Assume(Expr),
        Is(Expr),
        Orthopoly(String, usize, Expr),
    }

    impl CasCommand {
        /// Primary expression carried by the command.
        pub fn primary_expr(&self) -> &Expr {
            match self {
                Self::Expand(e)
                | Self::Factor(e)
                | Self::Ratsimp(e)
                | Self::Partfrac(e, _)
                | Self::Diff(e, _)
                | Self::Limit(e, _, _)
                | Self::Solve(e, _)
                | Self::Ode2(e, _, _)
                | Self::Integrate(e, _)
                | Self::Defint(e, _, _, _)
                | Self::Taylor(e, _, _, _)
                | Self::Laplace(e, _, _)
                | Self::Ilt(e, _, _)
                | Self::Ic1(e, _, _, _)
                | Self::Ic2(e, _, _, _, _)
                | Self::Bc2(e, _, _, _, _, _)
                | Self::Determinant(e)
                | Self::Invert(e)
                | Self::Charpoly(e, _)
                | Self::Trace(e)
                | Self::Eigenvalues(e)
                | Self::Adjoint(e)
                | Self::Transpose(e)
                | Self::Rectform(e)
                | Self::Polarform(e)
                | Self::Realpart(e)
                | Self::Imagpart(e)
                | Self::Conjugate(e)
                | Self::Carg(e)
                | Self::Cabs(e)
                | Self::Bfloat(e, _)
                | Self::Csign(e)
                | Self::Assume(e)
                | Self::Is(e)
                | Self::Orthopoly(_, _, e) => e,
            }
        }

        /// Canonical WebUI operation ID for this command.
        pub fn operation_id(&self) -> &'static str {
            match self {
                Self::Expand(_) => "expand",
                Self::Factor(_) => "factor",
                Self::Ratsimp(_) => "ratsimp",
                Self::Partfrac(_, _) => "partfrac",
                Self::Diff(_, _) => "differentiate",
                Self::Limit(_, _, _) => "limit",
                Self::Solve(_, _) => "solve",
                Self::Ode2(_, _, _) => "ode2",
                Self::Integrate(_, _) => "integrate",
                Self::Defint(_, _, _, _) => "defint",
                Self::Taylor(_, _, _, _) => "taylor",
                Self::Laplace(_, _, _) => "laplace",
                Self::Ilt(_, _, _) => "ilt",
                Self::Ic1(_, _, _, _) => "ic1",
                Self::Ic2(_, _, _, _, _) => "ic2",
                Self::Bc2(_, _, _, _, _, _) => "bc2",
                Self::Determinant(_) => "determinant",
                Self::Invert(_) => "invert",
                Self::Charpoly(_, _) => "charpoly",
                Self::Trace(_) => "trace",
                Self::Eigenvalues(_) => "eigenvalues",
                Self::Adjoint(_) => "adjoint",
                Self::Transpose(_) => "transpose",
                Self::Rectform(_) => "rectform",
                Self::Polarform(_) => "polarform",
                Self::Realpart(_) => "realpart",
                Self::Imagpart(_) => "imagpart",
                Self::Conjugate(_) => "conjugate",
                Self::Carg(_) => "carg",
                Self::Cabs(_) => "cabs",
                Self::Bfloat(_, _) => "bfloat",
                Self::Csign(_) => "csign",
                Self::Assume(_) => "assume",
                Self::Is(_) => "is",
                Self::Orthopoly(_, _, _) => "orthopoly",
            }
        }
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
        if !s[close + 1..].trim().is_empty() {
            return Err(err("unexpected trailing tokens after ')'"));
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
            "diff" => {
                need(2)?;
                Ok(CasCommand::Diff(e(0)?, v(1)))
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
                Ok(CasCommand::Defint(
                    e(0)?,
                    v(1),
                    parse_point(args[2])?,
                    parse_point(args[3])?,
                ))
            }
            "taylor" => {
                need(4)?;
                let order = args[3]
                    .trim()
                    .parse::<i64>()
                    .map_err(|_| err("bad taylor order"))?;
                Ok(CasCommand::Taylor(e(0)?, v(1), e(2)?, order))
            }
            "laplace" => {
                need(3)?;
                Ok(CasCommand::Laplace(e(0)?, v(1), v(2)))
            }
            "ilt" => {
                need(3)?;
                Ok(CasCommand::Ilt(e(0)?, v(1), v(2)))
            }
            "ic1" => {
                need(4)?;
                Ok(CasCommand::Ic1(e(0)?, v(1), e(2)?, e(3)?))
            }
            "ic2" => {
                need(5)?;
                Ok(CasCommand::Ic2(e(0)?, v(1), e(2)?, e(3)?, e(4)?))
            }
            "bc2" => {
                need(6)?;
                Ok(CasCommand::Bc2(e(0)?, v(1), e(2)?, e(3)?, e(4)?, e(5)?))
            }
            "determinant" => {
                need(1)?;
                Ok(CasCommand::Determinant(e(0)?))
            }
            "invert" => {
                need(1)?;
                Ok(CasCommand::Invert(e(0)?))
            }
            "charpoly" => {
                need(2)?;
                Ok(CasCommand::Charpoly(e(0)?, v(1)))
            }
            "trace" | "mat_trace" => {
                need(1)?;
                Ok(CasCommand::Trace(e(0)?))
            }
            "eigenvalues" => {
                need(1)?;
                Ok(CasCommand::Eigenvalues(e(0)?))
            }
            "adjoint" => {
                need(1)?;
                Ok(CasCommand::Adjoint(e(0)?))
            }
            "transpose" => {
                need(1)?;
                Ok(CasCommand::Transpose(e(0)?))
            }
            "rectform" => {
                need(1)?;
                Ok(CasCommand::Rectform(e(0)?))
            }
            "polarform" => {
                need(1)?;
                Ok(CasCommand::Polarform(e(0)?))
            }
            "realpart" => {
                need(1)?;
                Ok(CasCommand::Realpart(e(0)?))
            }
            "imagpart" => {
                need(1)?;
                Ok(CasCommand::Imagpart(e(0)?))
            }
            "conjugate" => {
                need(1)?;
                Ok(CasCommand::Conjugate(e(0)?))
            }
            "carg" => {
                need(1)?;
                Ok(CasCommand::Carg(e(0)?))
            }
            "cabs" => {
                need(1)?;
                Ok(CasCommand::Cabs(e(0)?))
            }
            "bfloat" => {
                if args.len() == 1 {
                    Ok(CasCommand::Bfloat(e(0)?, 16))
                } else if args.len() == 2 {
                    let prec = args[1]
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| err("bad bfloat precision"))?;
                    Ok(CasCommand::Bfloat(e(0)?, prec))
                } else {
                    Err(err("wrong argument count"))
                }
            }
            "csign" => {
                need(1)?;
                Ok(CasCommand::Csign(e(0)?))
            }
            "assume" => {
                need(1)?;
                Ok(CasCommand::Assume(e(0)?))
            }
            "is" => {
                need(1)?;
                Ok(CasCommand::Is(e(0)?))
            }
            "legendre_p" | "chebyshev_t" | "chebyshev_u" | "hermite_h" | "laguerre_l" => {
                need(2)?;
                let deg = args[0]
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| err("bad orthopoly degree"))?;
                Ok(CasCommand::Orthopoly(name.to_string(), deg, e(1)?))
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
            CasCommand::Diff(e, v) => Some(crate::diff::differentiate(e, v)),
            CasCommand::Limit(e, v, p) => crate::limit::limit(e, v, p),
            CasCommand::Solve(e, v) => {
                let roots = crate::solve::solve(e, v).0;
                if roots.is_empty() {
                    None
                } else if roots.len() == 1 {
                    roots.into_iter().next()
                } else {
                    Some(Expr::matrix(vec![roots]))
                }
            }
            CasCommand::Ode2(e, y, x) => crate::solve::ode2(e, y, x).map(|s| s.solution),
            CasCommand::Integrate(e, v) => Some(crate::integrate::integrate(e, v)),
            CasCommand::Defint(e, v, a, b) => match defint(e, v, a, b) {
                DefiniteResult::Value(x) => Some(x),
                _ => None,
            },
            CasCommand::Taylor(e, v, c, o) => crate::series::taylor(e, v, c, *o),
            #[cfg(feature = "transform")]
            CasCommand::Laplace(e, t, s) => crate::transform::try_laplace(e, t, s).ok(),
            #[cfg(feature = "transform")]
            CasCommand::Ilt(e, s, t) => crate::transform::try_ilt(e, s, t).ok(),
            #[cfg(feature = "transform")]
            CasCommand::Ic1(sol, x, x0, y0) => crate::transform::ic1(sol, x, "_C1", x0, y0).ok(),
            #[cfg(feature = "transform")]
            CasCommand::Ic2(sol, x, x0, y0, dy0) => {
                crate::transform::ic2(sol, x, "_C1", "_C2", x0, y0, dy0).ok()
            }
            #[cfg(feature = "transform")]
            CasCommand::Bc2(sol, x, x0, y0, x1, y1) => {
                crate::transform::bc2(sol, x, "_C1", "_C2", (x0, y0), (x1, y1)).ok()
            }
            #[cfg(feature = "matrix")]
            CasCommand::Determinant(e) => {
                let m = crate::matrix::Matrix::from_expr(e).ok()?;
                crate::matrix::determinant(&m).ok()
            }
            #[cfg(feature = "matrix")]
            CasCommand::Invert(e) => {
                let m = crate::matrix::Matrix::from_expr(e).ok()?;
                crate::matrix::invert(&m).ok().map(|inv| inv.to_expr())
            }
            #[cfg(feature = "matrix")]
            CasCommand::Charpoly(e, v) => {
                let m = crate::matrix::Matrix::from_expr(e).ok()?;
                crate::matrix::charpoly(&m, v).ok()
            }
            #[cfg(feature = "matrix")]
            CasCommand::Trace(e) => {
                let m = crate::matrix::Matrix::from_expr(e).ok()?;
                crate::matrix::trace(&m).ok()
            }
            #[cfg(feature = "matrix")]
            CasCommand::Eigenvalues(e) => {
                let m = crate::matrix::Matrix::from_expr(e).ok()?;
                crate::matrix::eigenvalues(&m)
                    .ok()
                    .map(|evs| Expr::matrix(vec![evs]))
            }
            #[cfg(feature = "matrix")]
            CasCommand::Adjoint(e) => {
                let m = crate::matrix::Matrix::from_expr(e).ok()?;
                crate::matrix::adjoint(&m).ok().map(|adj| adj.to_expr())
            }
            #[cfg(feature = "matrix")]
            CasCommand::Transpose(e) => {
                let m = crate::matrix::Matrix::from_expr(e).ok()?;
                Some(m.transpose().to_expr())
            }
            #[cfg(feature = "complex")]
            CasCommand::Rectform(e) => Some(crate::complex::rectform_expr(e)),
            #[cfg(feature = "complex")]
            CasCommand::Polarform(e) => Some(crate::complex::polarform_expr(e)),
            #[cfg(feature = "complex")]
            CasCommand::Realpart(e) => Some(crate::complex::realpart(e)),
            #[cfg(feature = "complex")]
            CasCommand::Imagpart(e) => Some(crate::complex::imagpart(e)),
            #[cfg(feature = "complex")]
            CasCommand::Conjugate(e) => Some(crate::complex::conjugate(e)),
            #[cfg(feature = "complex")]
            CasCommand::Carg(e) => Some(crate::complex::carg(e)),
            #[cfg(feature = "complex")]
            CasCommand::Cabs(e) => Some(crate::complex::cabs(e)),
            #[cfg(feature = "bfloat")]
            CasCommand::Bfloat(e, prec) => {
                Some(crate::bfloat::bfloat(e, Some((*prec).clamp(1, 200))))
            }
            #[cfg(feature = "assume")]
            CasCommand::Csign(e) => Some(Expr::var(
                &format!("{:?}", crate::assume::Assumptions::new().csign(e)).to_lowercase(),
            )),
            #[cfg(feature = "specfn")]
            CasCommand::Orthopoly(kind, deg, x) => {
                if *deg > 100 {
                    return None;
                }
                match kind.as_str() {
                    "legendre_p" => Some(crate::specfn::orthopoly::legendre_p_poly(*deg, x)),
                    "chebyshev_t" => Some(crate::specfn::orthopoly::chebyshev_t_poly(*deg, x)),
                    "chebyshev_u" => Some(crate::specfn::orthopoly::chebyshev_u_poly(*deg, x)),
                    "hermite_h" => Some(crate::specfn::orthopoly::hermite_h_poly(*deg, x)),
                    "laguerre_l" => Some(crate::specfn::orthopoly::laguerre_l_poly(*deg, x)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

#[cfg(feature = "cas")]
pub use cas_commands::{CasCommand, execute_cas, parse_maxima_cas};
