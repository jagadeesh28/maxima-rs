//! Monomials and variable contexts for sparse multivariate polynomials.
//!
//! A [`Monomial`] is an exponent vector over a fixed variable ordering. Exponent
//! vectors are stored as `Vec<u32>`; for up to [`PACKED_MAX_VARS`] variables whose
//! exponents fit in 8 bits, [`Monomial::packed`] returns a single 64-bit word.
//! Terms are ordered by graded reverse lexicographic (`grevlex`), the standard
//! ordering for sparse polynomial arithmetic.

use std::cmp::Ordering;
use std::collections::HashMap;

/// Maximum number of variables representable in a packed 64-bit exponent word
/// (8 variables × 8 bits, degree ≤ 255).
pub const PACKED_MAX_VARS: usize = 8;

/// A monomial: a product of variables raised to non-negative integer powers.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Monomial {
    exps: Vec<u32>,
}

impl Monomial {
    /// Build a monomial from an explicit exponent vector.
    pub fn from_exps(exps: Vec<u32>) -> Self {
        Self { exps }
    }

    /// The monomial `1` over `nvars` variables.
    pub fn one(nvars: usize) -> Self {
        Self {
            exps: vec![0; nvars],
        }
    }

    /// The exponent vector.
    pub fn exps(&self) -> &[u32] {
        &self.exps
    }

    /// Number of variables in the exponent vector.
    pub fn nvars(&self) -> usize {
        self.exps.len()
    }

    /// Exponent of the `i`-th variable (0 if out of range).
    pub fn exponent(&self, i: usize) -> u32 {
        self.exps.get(i).copied().unwrap_or(0)
    }

    /// Total degree (sum of exponents).
    pub fn total_degree(&self) -> u64 {
        self.exps.iter().map(|&e| u64::from(e)).sum()
    }

    /// True when every exponent is zero.
    pub fn is_one(&self) -> bool {
        self.exps.iter().all(|&e| e == 0)
    }

    /// Multiply two monomials (component-wise exponent addition).
    pub fn mul(&self, other: &Monomial) -> Monomial {
        debug_assert_eq!(self.exps.len(), other.exps.len());
        Monomial {
            exps: self
                .exps
                .iter()
                .zip(&other.exps)
                .map(|(&a, &b)| a + b)
                .collect(),
        }
    }

    /// Divide by `other`, returning `None` if not exactly divisible.
    pub fn div(&self, other: &Monomial) -> Option<Monomial> {
        debug_assert_eq!(self.exps.len(), other.exps.len());
        let mut exps = Vec::with_capacity(self.exps.len());
        for (&a, &b) in self.exps.iter().zip(&other.exps) {
            if a < b {
                return None;
            }
            exps.push(a - b);
        }
        Some(Monomial { exps })
    }

    /// Raise the monomial to the `n`-th power.
    pub fn pow(&self, n: u32) -> Monomial {
        Monomial {
            exps: self.exps.iter().map(|&e| e * n).collect(),
        }
    }

    /// Pack the exponent vector into one 64-bit word when possible.
    ///
    /// Requires at most [`PACKED_MAX_VARS`] variables with every exponent ≤ 255.
    pub fn packed(&self) -> Option<u64> {
        if self.exps.len() > PACKED_MAX_VARS {
            return None;
        }
        let mut word = 0u64;
        for (i, &e) in self.exps.iter().enumerate() {
            if e > 255 {
                return None;
            }
            word |= u64::from(e) << (8 * i);
        }
        Some(word)
    }

    /// Graded reverse lexicographic comparison (`Greater` = leading term).
    pub fn grevlex_cmp(&self, other: &Monomial) -> Ordering {
        if self.exps.len() != other.exps.len() {
            return self.exps.len().cmp(&other.exps.len());
        }
        match self.total_degree().cmp(&other.total_degree()) {
            Ordering::Equal => {}
            ord => return ord,
        }
        for i in (0..self.exps.len()).rev() {
            match self.exps[i].cmp(&other.exps[i]) {
                Ordering::Equal => continue,
                ord => return ord.reverse(),
            }
        }
        Ordering::Equal
    }
}

impl Ord for Monomial {
    fn cmp(&self, other: &Self) -> Ordering {
        self.grevlex_cmp(other)
    }
}

impl PartialOrd for Monomial {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Maps variable names to exponent-vector indices.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VarContext {
    vars: Vec<String>,
    index: HashMap<String, usize>,
}

impl VarContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or look up) a variable, returning its index.
    pub fn add(&mut self, name: &str) -> usize {
        if let Some(&i) = self.index.get(name) {
            return i;
        }
        let i = self.vars.len();
        self.vars.push(name.to_string());
        self.index.insert(name.to_string(), i);
        i
    }

    /// Look up a variable index without registering it.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.index.get(name).copied()
    }

    /// Number of variables.
    pub fn len(&self) -> usize {
        self.vars.len()
    }

    /// True if no variables are registered.
    pub fn is_empty(&self) -> bool {
        self.vars.is_empty()
    }

    /// Variable names in index order.
    pub fn vars(&self) -> &[String] {
        &self.vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_context_registers_once() {
        let mut ctx = VarContext::new();
        assert_eq!(ctx.add("x"), 0);
        assert_eq!(ctx.add("y"), 1);
        assert_eq!(ctx.add("x"), 0);
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx.vars().to_vec(), vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn monomial_multiply_and_degree() {
        let a = Monomial::from_exps(vec![2, 1]);
        let b = Monomial::from_exps(vec![1, 3]);
        let c = a.mul(&b);
        assert_eq!(c.exps(), &[3, 4]);
        assert_eq!(c.total_degree(), 7);
    }

    #[test]
    fn monomial_division() {
        let a = Monomial::from_exps(vec![3, 2]);
        let b = Monomial::from_exps(vec![1, 1]);
        assert_eq!(a.div(&b).unwrap().exps(), &[2, 1]);
        assert!(b.div(&a).is_none());
    }

    #[test]
    fn grevlex_orders_by_degree_then_reverse_lex() {
        let x2 = Monomial::from_exps(vec![2, 0]);
        let xy = Monomial::from_exps(vec![1, 1]);
        let y2 = Monomial::from_exps(vec![0, 2]);
        assert!(x2 > xy);
        assert!(xy > y2);
        let x3 = Monomial::from_exps(vec![3, 0]);
        assert!(x3 > y2);
    }

    #[test]
    fn packed_roundtrip_limits() {
        let m = Monomial::from_exps(vec![1, 2, 3]);
        let p = m.packed().unwrap();
        assert_eq!((p & 0xff) as u32, 1);
        assert_eq!(((p >> 8) & 0xff) as u32, 2);
        assert_eq!(((p >> 16) & 0xff) as u32, 3);
        let big = Monomial::from_exps(vec![0; PACKED_MAX_VARS + 1]);
        assert!(big.packed().is_none());
        let huge = Monomial::from_exps(vec![300]);
        assert!(huge.packed().is_none());
    }
}
