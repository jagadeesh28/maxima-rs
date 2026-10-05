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
}
