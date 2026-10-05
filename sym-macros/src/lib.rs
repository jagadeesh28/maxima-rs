//! Procedural macro crate for compile-time symbolic mathematics in `maxima-rs`.
//!
//! Provides two procedural macros:
//! - [`sym_fn!`]: Generates zero-overhead Rust functions with compile-time optimization
//!   and analytical differentiation.
//! - [`sym_eval!`]: Evaluates and simplifies symbolic expressions at compile time.

use proc_macro::TokenStream;

mod config;
mod expand;
mod parse_sym_eval;
mod parse_sym_fn;

/// Define a mathematically optimized function with optional analytical differentiation at compile time.
///
/// # Example
///
/// ```rust,ignore
/// sym_fn! {
///     pub fn rosenbrock(x: f64, y: f64) -> (f64, [f64; 2]) {
///         expr: (1.0 - x).powi(2) + 100.0 * (y - x.powi(2)).powi(2),
///         gradient: [x, y],
///     }
/// }
/// ```
#[proc_macro]
pub fn sym_fn(input: TokenStream) -> TokenStream {
    expand::expand_sym_fn(input)
}

/// Simplify and optimize an inline mathematical expression at compile time.
///
/// # Example
///
/// ```rust,ignore
/// let val = sym_eval!((x + 0.0) * 1.0);
/// ```
#[proc_macro]
pub fn sym_eval(input: TokenStream) -> TokenStream {
    expand::expand_sym_eval(input)
}
