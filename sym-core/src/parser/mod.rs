//! Multi-format mathematical parsers and error types.
//!
//! Provides parsers for:
//! - Maxima string syntax ([`parse_maxima`]): `"(sin(x)^2 + cos(x)^2) / 2"`
//! - LaTeX formula scalar math syntax ([`parse_latex`]): `r"\frac{\sin(x)^2 + \cos(x)^2}{2}"`

pub mod latex;
pub mod maxima;

pub use latex::parse_latex;
pub use maxima::parse_maxima;

use thiserror::Error;

/// Parse error representing syntax failures across supported mathematical notations.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    /// Syntax error with description and remaining unparsed input.
    #[error("Parse error: {message} at '{remaining}'")]
    Syntax { message: String, remaining: String },
    /// Empty input provided to parser.
    #[error("Empty input provided")]
    EmptyInput,
    /// Unexpected token encountered.
    #[error("Unexpected token '{token}' at position {position}")]
    UnexpectedToken { token: String, position: usize },
    /// Recursion depth limit exceeded.
    #[error("Recursion limit exceeded (max 256)")]
    RecursionLimitExceeded,
}

/// Guard against stack overflow from deeply nested expressions.
#[derive(Debug, Clone)]
pub struct RecursionDepthGuard {
    depth: usize,
    max_depth: usize,
    pub exceeded: bool,
}

impl RecursionDepthGuard {
    pub const DEFAULT_MAX_DEPTH: usize = 256;

    pub fn new() -> Self {
        Self {
            depth: 0,
            max_depth: Self::DEFAULT_MAX_DEPTH,
            exceeded: false,
        }
    }

    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            depth: 0,
            max_depth,
            exceeded: false,
        }
    }

    pub fn enter(&mut self) -> Result<(), ParseError> {
        if self.depth >= self.max_depth {
            self.exceeded = true;
            return Err(ParseError::RecursionLimitExceeded);
        }
        self.depth += 1;
        Ok(())
    }

    pub fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    pub fn depth(&self) -> usize {
        self.depth
    }
}

impl Default for RecursionDepthGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if bracket/parenthesis/brace nesting depth in input exceeds limit.
pub fn check_nesting_limit(input: &str, limit: usize) -> Result<(), ParseError> {
    let mut depth: usize = 0;
    for b in input.bytes() {
        if b == b'(' || b == b'[' || b == b'{' {
            depth += 1;
            if depth > limit {
                return Err(ParseError::RecursionLimitExceeded);
            }
        } else if b == b')' || b == b']' || b == b'}' {
            depth = depth.saturating_sub(1);
        }
    }
    Ok(())
}

/// Parse an integer, decimal, or scientific notation string into an exact [`num_rational::BigRational`].
pub fn parse_exact_rational(s: &str) -> Option<num_rational::BigRational> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::Pow;

    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (mantissa, exp_val): (&str, i32) = if let Some(pos) = s.find(['e', 'E']) {
        let (m, e) = s.split_at(pos);
        let e_str = &e[1..];
        let exp = e_str.parse::<i32>().ok()?;
        (m, exp)
    } else {
        (s, 0)
    };

    let (digits_big, frac_len): (BigInt, i32) =
        if let Some((int_part, frac_part)) = mantissa.split_once('.') {
            let combined = format!("{}{}", int_part, frac_part);
            let digits = combined.parse::<BigInt>().ok()?;
            (digits, frac_part.len() as i32)
        } else {
            let digits = mantissa.parse::<BigInt>().ok()?;
            (digits, 0)
        };

    let net_exp = exp_val - frac_len;
    let ten = BigInt::from(10);
    if net_exp >= 0 {
        let factor = ten.pow(net_exp as u32);
        Some(BigRational::from_integer(digits_big * factor))
    } else {
        let divisor = ten.pow((-net_exp) as u32);
        Some(BigRational::new(digits_big, divisor))
    }
}
