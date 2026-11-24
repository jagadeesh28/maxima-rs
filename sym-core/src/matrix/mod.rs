//! Structured encoding of [`Expr::Matrix`] into an e-graph symbol and symbolic linear algebra.
//!
//! When `feature = "matrix"` is active, provides the Bareiss fraction-free LU determinant,
//! adjoint inverse, characteristic polynomial, trace, and eigenvalue computations citing Maxima's `newdet.lisp` (ADR-0010).

use std::str::FromStr;

use num_bigint::BigInt;
use num_rational::BigRational;

use crate::expr::Expr;
use crate::tensor::{Index, IndexType};

#[cfg(feature = "matrix")]
pub mod bareiss;

#[cfg(feature = "matrix")]
pub use bareiss::{
    Matrix, MatrixError, adjoint, charpoly, determinant, eigenvalues, invert, trace,
};

/// Encode a matrix (rows of expressions) into a single e-graph symbol.
pub fn encode_matrix_symbol(rows: &[Vec<Expr>]) -> String {
    let mut out: Vec<String> = Vec::new();
    encode_matrix(rows, &mut out);
    out.join(" ")
}

/// Decode a matrix symbol produced by [`encode_matrix_symbol`].
///
/// Returns `None` if the symbol is not a well-formed matrix encoding.
pub fn decode_matrix_symbol(s: &str) -> Option<Vec<Vec<Expr>>> {
    let toks: Vec<&str> = s.split(' ').collect();
    let mut pos = 0;
    let decoded = decode_expr(&toks, &mut pos)?;
    if pos != toks.len() {
        return None;
    }
    match decoded {
        Expr::Matrix(rows) => Some(rows),
        _ => None,
    }
}

fn escape_name(s: &str) -> String {
    s.replace('%', "%25").replace(' ', "%20")
}

fn unescape_name(s: &str) -> String {
    s.replace("%20", " ").replace("%25", "%")
}

fn encode_matrix(rows: &[Vec<Expr>], out: &mut Vec<String>) {
    out.push("MX".into());
    out.push(rows.len().to_string());
    for row in rows {
        out.push(row.len().to_string());
        for elem in row {
            encode_expr(elem, out);
        }
    }
}

fn encode_unary(op: &str, a: &Expr, out: &mut Vec<String>) {
    out.push(op.into());
    encode_expr(a, out);
}

fn encode_binary(op: &str, a: &Expr, b: &Expr, out: &mut Vec<String>) {
    out.push(op.into());
    encode_expr(a, out);
    encode_expr(b, out);
}

fn encode_expr(e: &Expr, out: &mut Vec<String>) {
    match e {
        Expr::Var(name) => {
            out.push("V".into());
            out.push(escape_name(name));
        }
        Expr::Const(r) => {
            out.push("C".into());
            out.push(format!("{}/{}", r.numer(), r.denom()));
        }
        Expr::Add(a, b) => encode_binary("+", a, b, out),
        Expr::Sub(a, b) => encode_binary("-", a, b, out),
        Expr::Mul(a, b) => encode_binary("*", a, b, out),
        Expr::Div(a, b) => encode_binary("/", a, b, out),
        Expr::Pow(a, b) => encode_binary("^", a, b, out),
        Expr::Neg(a) => encode_unary("neg", a, out),
        Expr::Sin(a) => encode_unary("sin", a, out),
        Expr::Cos(a) => encode_unary("cos", a, out),
        Expr::Tan(a) => encode_unary("tan", a, out),
        Expr::Exp(a) => encode_unary("exp", a, out),
        Expr::Ln(a) => encode_unary("ln", a, out),
        Expr::Sqrt(a) => encode_unary("sqrt", a, out),
        Expr::Sinh(a) => encode_unary("sh", a, out),
        Expr::Cosh(a) => encode_unary("ch", a, out),
        Expr::Tanh(a) => encode_unary("th", a, out),
        Expr::Asin(a) => encode_unary("as", a, out),
        Expr::Acos(a) => encode_unary("ac", a, out),
        Expr::Atan(a) => encode_unary("at", a, out),
        Expr::Factorial(a) => encode_unary("FAC", a, out),
        Expr::Pi => out.push("PI".into()),
        Expr::E => out.push("EE".into()),
        #[cfg(feature = "complex")]
        Expr::I => out.push("II".into()),
        Expr::Tensor(name, indices) => {
            out.push("TEN".into());
            out.push(escape_name(name));
            out.push(indices.len().to_string());
            for idx in indices {
                out.push(match idx.idx_type {
                    IndexType::Covariant => "C".into(),
                    IndexType::Contravariant => "U".into(),
                });
                out.push(escape_name(&idx.name));
            }
        }
        Expr::Derivative(a, b) => encode_binary("DR", a, b, out),
        Expr::Integral(a, b) => encode_binary("IN", a, b, out),
        Expr::Sum(a, b, lower, upper) => {
            out.push("SU".into());
            encode_expr(a, out);
            encode_expr(b, out);
            encode_expr(lower, out);
            encode_expr(upper, out);
        }
        Expr::Matrix(rows) => encode_matrix(rows, out),
        #[cfg(feature = "bfloat")]
        Expr::Bfloat(b) => {
            out.push("BF".into());
            out.push(b.prec.to_string());
            out.push(b.to_maxima_string());
        }
        #[cfg(feature = "specfn")]
        Expr::SpecialFn(kind, args) => {
            out.push("SF".into());
            out.push(kind.as_str().into());
            out.push(args.len().to_string());
            for arg in args {
                encode_expr(arg, out);
            }
        }
    }
}

fn take<'a>(toks: &[&'a str], pos: &mut usize) -> Option<&'a str> {
    let tok = *toks.get(*pos)?;
    *pos += 1;
    Some(tok)
}

fn take_count(toks: &[&str], pos: &mut usize) -> Option<usize> {
    take(toks, pos)?.parse().ok()
}

fn decode_expr(toks: &[&str], pos: &mut usize) -> Option<Expr> {
    let op = take(toks, pos)?;
    Some(match op {
        "V" => Expr::var(&unescape_name(take(toks, pos)?)),
        "C" => {
            let raw = take(toks, pos)?;
            let (num, den) = raw.split_once('/')?;
            let num = BigInt::from_str(num).ok()?;
            let den = BigInt::from_str(den).ok()?;
            Expr::constant(BigRational::new(num, den))
        }
        "+" => Expr::add(decode_expr(toks, pos)?, decode_expr(toks, pos)?),
        "-" => Expr::sub(decode_expr(toks, pos)?, decode_expr(toks, pos)?),
        "*" => Expr::mul(decode_expr(toks, pos)?, decode_expr(toks, pos)?),
        "/" => Expr::div(decode_expr(toks, pos)?, decode_expr(toks, pos)?),
        "^" => Expr::pow(decode_expr(toks, pos)?, decode_expr(toks, pos)?),
        "neg" => Expr::neg(decode_expr(toks, pos)?),
        "sin" => Expr::sin(decode_expr(toks, pos)?),
        "cos" => Expr::cos(decode_expr(toks, pos)?),
        "tan" => Expr::tan(decode_expr(toks, pos)?),
        "exp" => Expr::exp(decode_expr(toks, pos)?),
        "ln" => Expr::ln(decode_expr(toks, pos)?),
        "sqrt" => Expr::sqrt(decode_expr(toks, pos)?),
        "sh" => Expr::sinh(decode_expr(toks, pos)?),
        "ch" => Expr::cosh(decode_expr(toks, pos)?),
        "th" => Expr::tanh(decode_expr(toks, pos)?),
        "as" => Expr::asin(decode_expr(toks, pos)?),
        "ac" => Expr::acos(decode_expr(toks, pos)?),
        "at" => Expr::atan(decode_expr(toks, pos)?),
        "FAC" => Expr::factorial(decode_expr(toks, pos)?),
        "PI" => Expr::Pi,
        "EE" => Expr::E,
        #[cfg(feature = "complex")]
        "II" => Expr::I,
        "TEN" => {
            let name = unescape_name(take(toks, pos)?);
            let n = take_count(toks, pos)?;
            let mut indices = Vec::with_capacity(n);
            for _ in 0..n {
                let kind = take(toks, pos)?;
                let idx_name = unescape_name(take(toks, pos)?);
                indices.push(match kind {
                    "C" => Index::cov(&idx_name),
                    "U" => Index::con(&idx_name),
                    _ => return None,
                });
            }
            Expr::Tensor(name, indices)
        }
        "DR" => Expr::derivative(decode_expr(toks, pos)?, decode_expr(toks, pos)?),
        "IN" => Expr::integral(decode_expr(toks, pos)?, decode_expr(toks, pos)?),
        "SU" => Expr::sum(
            decode_expr(toks, pos)?,
            decode_expr(toks, pos)?,
            decode_expr(toks, pos)?,
            decode_expr(toks, pos)?,
        ),
        "MX" => {
            let nrows = take_count(toks, pos)?;
            let mut rows = Vec::with_capacity(nrows);
            for _ in 0..nrows {
                let ncols = take_count(toks, pos)?;
                let mut row = Vec::with_capacity(ncols);
                for _ in 0..ncols {
                    row.push(decode_expr(toks, pos)?);
                }
                rows.push(row);
            }
            Expr::Matrix(rows)
        }
        #[cfg(feature = "bfloat")]
        "BF" => {
            let prec = take_count(toks, pos)?;
            let s = take(toks, pos)?;
            let bw = crate::bfloat::BfloatWrapper::parse(s, prec).ok()?;
            Expr::Bfloat(bw)
        }
        #[cfg(feature = "specfn")]
        "SF" => {
            let kind_str = take(toks, pos)?;
            let kind = crate::specfn::SpecialFnKind::from_name(kind_str)?;
            let nargs = take_count(toks, pos)?;
            let mut args = Vec::with_capacity(nargs);
            for _ in 0..nargs {
                args.push(decode_expr(toks, pos)?);
            }
            Expr::SpecialFn(kind, args)
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_symbol_roundtrip() {
        let mat = Expr::matrix(vec![
            vec![Expr::var("a"), Expr::frac(-1, 2)],
            vec![Expr::sinh(Expr::var("x")), Expr::factorial(Expr::int(5))],
        ]);
        let rows = match &mat {
            Expr::Matrix(rows) => rows.clone(),
            _ => unreachable!(),
        };
        let decoded = decode_matrix_symbol(&encode_matrix_symbol(&rows)).expect("decode matrix");
        assert_eq!(Expr::Matrix(decoded), mat);
    }

    #[test]
    fn test_nested_matrix_and_tensor_roundtrip() {
        let inner = Expr::matrix(vec![vec![Expr::pi(), Expr::E]]);
        let outer = Expr::matrix(vec![vec![
            inner,
            Expr::tensor("T", vec![Index::con("mu"), Index::cov("nu")]),
        ]]);
        let rows = match &outer {
            Expr::Matrix(rows) => rows.clone(),
            _ => unreachable!(),
        };
        let decoded = decode_matrix_symbol(&encode_matrix_symbol(&rows)).expect("decode");
        assert_eq!(Expr::Matrix(decoded), outer);
    }

    #[test]
    fn test_invalid_matrix_symbol() {
        assert!(decode_matrix_symbol("matrix_unsupported_in_egraph").is_none());
        assert!(decode_matrix_symbol("MX 1 2 V x").is_none());
    }
}
