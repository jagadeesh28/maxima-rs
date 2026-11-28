//! Deductive inference and assumptions engine.
//!
//! Citations:
//! - Maxima 5.50: `src/compar.lisp` (`assume`, `forget`, `is`, `csign`, `asksign`, `compar-facts`).
//! - Maxima 5.50: `src/askp.lisp` (`ask-prop`, `asksign`).
//! - ADR-0011: Special Functions, Operational Laplace Transforms, and Scoped Assumptions Architecture.

use crate::expr::Expr;
use crate::rational::{BigRational, Rat};
use num_traits::Zero;
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Mathematical sign of an expression.
///
/// Maxima `src/compar.lisp` (`$pos`, `$neg`, `$zero`, `$nonneg`, `$nonpos`, `$pn`, `$complex`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sign {
    /// Strictly positive ($> 0$).
    Pos,
    /// Strictly negative ($< 0$).
    Neg,
    /// Identically zero ($= 0$).
    Zero,
    /// Non-negative ($\ge 0$).
    NonNeg,
    /// Non-positive ($\le 0$).
    NonPos,
    /// Non-real complex number.
    Complex,
    /// Indeterminate or unknown sign.
    Unknown,
}

impl fmt::Display for Sign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sign::Pos => write!(f, "pos"),
            Sign::Neg => write!(f, "neg"),
            Sign::Zero => write!(f, "zero"),
            Sign::NonNeg => write!(f, "nonneg"),
            Sign::NonPos => write!(f, "nonpos"),
            Sign::Complex => write!(f, "complex"),
            Sign::Unknown => write!(f, "unknown"),
        }
    }
}

/// Comparison operators for relational facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Comparison {
    /// $>$
    GreaterThan,
    /// $\ge$
    GreaterThanOrEqual,
    /// $<$
    LessThan,
    /// $\le$
    LessThanOrEqual,
    /// $=$
    Equal,
    /// $\ne$
    NotEqual,
}

impl fmt::Display for Comparison {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Comparison::GreaterThan => write!(f, ">"),
            Comparison::GreaterThanOrEqual => write!(f, ">="),
            Comparison::LessThan => write!(f, "<"),
            Comparison::LessThanOrEqual => write!(f, "<="),
            Comparison::Equal => write!(f, "="),
            Comparison::NotEqual => write!(f, "#"),
        }
    }
}

/// Three-valued predicate logic outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PredicateResult {
    /// Proven true.
    True,
    /// Proven false.
    False,
    /// Undecided by current facts.
    Unknown,
}

impl fmt::Display for PredicateResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PredicateResult::True => write!(f, "true"),
            PredicateResult::False => write!(f, "false"),
            PredicateResult::Unknown => write!(f, "unknown"),
        }
    }
}

/// Domain property kinds for expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyKind {
    /// Real number ($\mathbb{R}$).
    Real,
    /// Complex number ($\mathbb{C}$).
    Complex,
    /// Integer ($\mathbb{Z}$).
    Integer,
    /// Non-integer.
    NonInteger,
    /// Rational number ($\mathbb{Q}$).
    Rational,
    /// Irrational number.
    Irrational,
    /// Even integer.
    Even,
    /// Odd integer.
    Odd,
    /// Strictly positive.
    Pos,
    /// Strictly negative.
    Neg,
    /// Zero.
    Zero,
    /// Non-zero.
    NonZero,
}

impl fmt::Display for PropertyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyKind::Real => write!(f, "real"),
            PropertyKind::Complex => write!(f, "complex"),
            PropertyKind::Integer => write!(f, "integer"),
            PropertyKind::NonInteger => write!(f, "noninteger"),
            PropertyKind::Rational => write!(f, "rational"),
            PropertyKind::Irrational => write!(f, "irrational"),
            PropertyKind::Even => write!(f, "even"),
            PropertyKind::Odd => write!(f, "odd"),
            PropertyKind::Pos => write!(f, "pos"),
            PropertyKind::Neg => write!(f, "neg"),
            PropertyKind::Zero => write!(f, "zero"),
            PropertyKind::NonZero => write!(f, "nonzero"),
        }
    }
}

/// Assumed mathematical fact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Fact {
    /// Relational fact: `lhs op rhs`.
    Rel(Expr, Comparison, Expr),
    /// Domain property fact: `kind(expr, prop)`.
    Prop(Expr, PropertyKind),
}

impl fmt::Display for Fact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fact::Rel(lhs, op, rhs) => write!(f, "{} {} {}", lhs, op, rhs),
            Fact::Prop(expr, prop) => write!(f, "kind({}, {})", expr, prop),
        }
    }
}

/// Interval bounds for a variable or expression.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Bound {
    /// Lower bound `(val, is_strict)`.
    lower: Option<(BigRational, bool)>,
    /// Upper bound `(val, is_strict)`.
    upper: Option<(BigRational, bool)>,
}

impl Bound {
    fn update_lower(&mut self, val: BigRational, strict: bool) -> Result<(), String> {
        if let Some((curr_val, curr_strict)) = &mut self.lower {
            if val > *curr_val {
                *curr_val = val;
                *curr_strict = strict;
            } else if val == *curr_val && strict {
                *curr_strict = true;
            }
        } else {
            self.lower = Some((val, strict));
        }
        self.check_contradiction()
    }

    fn update_upper(&mut self, val: BigRational, strict: bool) -> Result<(), String> {
        if let Some((curr_val, curr_strict)) = &mut self.upper {
            if val < *curr_val {
                *curr_val = val;
                *curr_strict = strict;
            } else if val == *curr_val && strict {
                *curr_strict = true;
            }
        } else {
            self.upper = Some((val, strict));
        }
        self.check_contradiction()
    }

    fn check_contradiction(&self) -> Result<(), String> {
        if let (Some((l_val, l_strict)), Some((u_val, u_strict))) = (&self.lower, &self.upper) {
            if l_val > u_val {
                return Err(format!(
                    "Contradictory bounds: lower {} > upper {}",
                    l_val, u_val
                ));
            }
            if l_val == u_val && (*l_strict || *u_strict) {
                return Err(format!(
                    "Contradictory strict bounds: lower {} == upper {}",
                    l_val, u_val
                ));
            }
        }
        Ok(())
    }
}

/// Scoped, immutable, thread-safe mathematical assumptions context.
///
/// Contains an inequality DAG, interval bounds database, and property classification.
/// Conforms to ADR-0011 (no global mutable state, safe for concurrent execution).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Assumptions {
    facts: Vec<Fact>,
    var_bounds: HashMap<String, Bound>,
    var_props: HashMap<String, HashSet<PropertyKind>>,
    /// Normalized directed inequality edges `(u, strict, v)` meaning `u > v` (if strict) or `u >= v`.
    edges: Vec<(Expr, bool, Expr)>,
    /// Symmetric `!=` pairs.
    not_equals: HashSet<(Expr, Expr)>,
}

impl Assumptions {
    /// Create an empty assumptions context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an assumed fact, returning a new updated context or an error on contradiction.
    pub fn assume(&self, fact: Fact) -> Result<Self, String> {
        let mut new_ctx = self.clone();
        match &fact {
            Fact::Rel(lhs, op, rhs) => {
                new_ctx.apply_relation(lhs, *op, rhs)?;
            }
            Fact::Prop(expr, prop) => {
                new_ctx.apply_property(expr, *prop)?;
            }
        }
        new_ctx.facts.push(fact);
        Ok(new_ctx)
    }

    /// Assume a relational fact: `lhs op rhs`.
    pub fn assume_rel(&self, lhs: Expr, op: Comparison, rhs: Expr) -> Result<Self, String> {
        self.assume(Fact::Rel(lhs, op, rhs))
    }

    /// Assume a domain property: `kind(expr, prop)`.
    pub fn assume_prop(&self, expr: Expr, prop: PropertyKind) -> Result<Self, String> {
        self.assume(Fact::Prop(expr, prop))
    }

    /// Remove an assumed fact, returning a new context rebuilt from remaining facts.
    pub fn forget(&self, fact: &Fact) -> Self {
        let mut new_ctx = Assumptions::new();
        for f in &self.facts {
            if f != fact
                && let Ok(updated) = new_ctx.assume(f.clone())
            {
                new_ctx = updated;
            }
        }
        new_ctx
    }

    /// List all currently assumed facts.
    pub fn facts(&self) -> &[Fact] {
        &self.facts
    }

    /// Determine whether a predicate `lhs op rhs` is true, false, or unknown.
    ///
    /// Maxima `src/compar.lisp` (`$is`).
    pub fn is(&self, lhs: &Expr, op: Comparison, rhs: &Expr) -> PredicateResult {
        if lhs == rhs {
            return match op {
                Comparison::Equal
                | Comparison::GreaterThanOrEqual
                | Comparison::LessThanOrEqual => PredicateResult::True,
                Comparison::NotEqual | Comparison::GreaterThan | Comparison::LessThan => {
                    PredicateResult::False
                }
            };
        }

        // Direct interval check if lhs is Var and rhs is Const
        if let (Expr::Var(v), Expr::Const(r)) = (lhs, rhs)
            && let Some(b) = self.var_bounds.get(v)
        {
            if let Some((l_val, l_strict)) = &b.lower
                && l_val >= r
            {
                match op {
                    Comparison::GreaterThan if l_val > r || *l_strict => {
                        return PredicateResult::True;
                    }
                    Comparison::GreaterThanOrEqual => return PredicateResult::True,
                    Comparison::LessThan => return PredicateResult::False,
                    Comparison::LessThanOrEqual if l_val > r || *l_strict => {
                        return PredicateResult::False;
                    }
                    Comparison::Equal if l_val > r || *l_strict => {
                        return PredicateResult::False;
                    }
                    Comparison::NotEqual if l_val > r || *l_strict => {
                        return PredicateResult::True;
                    }
                    _ => {}
                }
            }
            if let Some((u_val, u_strict)) = &b.upper
                && u_val <= r
            {
                match op {
                    Comparison::LessThan if u_val < r || *u_strict => {
                        return PredicateResult::True;
                    }
                    Comparison::LessThanOrEqual => return PredicateResult::True,
                    Comparison::GreaterThan => return PredicateResult::False,
                    Comparison::GreaterThanOrEqual if u_val < r || *u_strict => {
                        return PredicateResult::False;
                    }
                    Comparison::Equal if u_val < r || *u_strict => {
                        return PredicateResult::False;
                    }
                    Comparison::NotEqual if u_val < r || *u_strict => {
                        return PredicateResult::True;
                    }
                    _ => {}
                }
            }
        }

        // Transitive DAG reachability check (supports >, >=, <, <=, =, != and negation)
        match op {
            Comparison::GreaterThan => {
                if self.dag_path(lhs, rhs) == Some(true) {
                    return PredicateResult::True;
                }
                if self.dag_path(rhs, lhs).is_some() {
                    return PredicateResult::False;
                }
            }
            Comparison::GreaterThanOrEqual => {
                if self.dag_path(lhs, rhs).is_some() {
                    return PredicateResult::True;
                }
                if self.dag_path(rhs, lhs) == Some(true) {
                    return PredicateResult::False;
                }
            }
            Comparison::LessThan => {
                if self.dag_path(rhs, lhs) == Some(true) {
                    return PredicateResult::True;
                }
                if self.dag_path(lhs, rhs).is_some() {
                    return PredicateResult::False;
                }
            }
            Comparison::LessThanOrEqual => {
                if self.dag_path(rhs, lhs).is_some() {
                    return PredicateResult::True;
                }
                if self.dag_path(lhs, rhs) == Some(true) {
                    return PredicateResult::False;
                }
            }
            Comparison::Equal => {
                if self.dag_path(lhs, rhs) == Some(false) && self.dag_path(rhs, lhs) == Some(false)
                {
                    return PredicateResult::True;
                }
                if self.dag_path(lhs, rhs) == Some(true)
                    || self.dag_path(rhs, lhs) == Some(true)
                    || self.has_not_equal(lhs, rhs)
                {
                    return PredicateResult::False;
                }
            }
            Comparison::NotEqual => {
                if self.dag_path(lhs, rhs) == Some(true)
                    || self.dag_path(rhs, lhs) == Some(true)
                    || self.has_not_equal(lhs, rhs)
                {
                    return PredicateResult::True;
                }
                if self.dag_path(lhs, rhs) == Some(false) && self.dag_path(rhs, lhs) == Some(false)
                {
                    return PredicateResult::False;
                }
            }
        }

        // Check difference sign: (lhs - rhs)
        let diff = Expr::sub(lhs.clone(), rhs.clone());
        match self.csign(&diff) {
            Sign::Pos => match op {
                Comparison::GreaterThan | Comparison::GreaterThanOrEqual | Comparison::NotEqual => {
                    PredicateResult::True
                }
                Comparison::LessThan | Comparison::LessThanOrEqual | Comparison::Equal => {
                    PredicateResult::False
                }
            },
            Sign::Neg => match op {
                Comparison::LessThan | Comparison::LessThanOrEqual | Comparison::NotEqual => {
                    PredicateResult::True
                }
                Comparison::GreaterThan | Comparison::GreaterThanOrEqual | Comparison::Equal => {
                    PredicateResult::False
                }
            },
            Sign::Zero => match op {
                Comparison::Equal
                | Comparison::GreaterThanOrEqual
                | Comparison::LessThanOrEqual => PredicateResult::True,
                Comparison::NotEqual | Comparison::GreaterThan | Comparison::LessThan => {
                    PredicateResult::False
                }
            },
            Sign::NonNeg => match op {
                Comparison::GreaterThanOrEqual => PredicateResult::True,
                Comparison::LessThan => PredicateResult::False,
                _ => PredicateResult::Unknown,
            },
            Sign::NonPos => match op {
                Comparison::LessThanOrEqual => PredicateResult::True,
                Comparison::GreaterThan => PredicateResult::False,
                _ => PredicateResult::Unknown,
            },
            _ => PredicateResult::Unknown,
        }
    }

    /// Determine the sign of an expression under current assumptions.
    ///
    /// Maxima `src/compar.lisp` (`$csign`).
    pub fn csign(&self, expr: &Expr) -> Sign {
        match expr {
            Expr::Const(r) => {
                if r.is_zero() {
                    Sign::Zero
                } else if r > &BigRational::from_integer(0.into()) {
                    Sign::Pos
                } else {
                    Sign::Neg
                }
            }
            Expr::Pi | Expr::E => Sign::Pos,
            #[cfg(feature = "complex")]
            Expr::I => Sign::Complex,
            Expr::Var(v) => {
                if let Some(b) = self.var_bounds.get(v) {
                    let zero = BigRational::from_integer(0.into());
                    if let Some((l_val, l_strict)) = &b.lower {
                        if l_val > &zero || (l_val == &zero && *l_strict) {
                            return Sign::Pos;
                        }
                        if l_val == &zero && !l_strict {
                            if let Some((u_val, _)) = &b.upper
                                && u_val == &zero
                            {
                                return Sign::Zero;
                            }
                            return Sign::NonNeg;
                        }
                    }
                    if let Some((u_val, u_strict)) = &b.upper {
                        if u_val < &zero || (u_val == &zero && *u_strict) {
                            return Sign::Neg;
                        }
                        if u_val == &zero && !u_strict {
                            if let Some((l_val, _)) = &b.lower
                                && l_val == &zero
                            {
                                return Sign::Zero;
                            }
                            return Sign::NonPos;
                        }
                    }
                }
                if let Some(props) = self.var_props.get(v) {
                    if props.contains(&PropertyKind::Pos) {
                        return Sign::Pos;
                    }
                    if props.contains(&PropertyKind::Neg) {
                        return Sign::Neg;
                    }
                    if props.contains(&PropertyKind::Zero) {
                        return Sign::Zero;
                    }
                }
                Sign::Unknown
            }
            Expr::Neg(a) => match self.csign(a) {
                Sign::Pos => Sign::Neg,
                Sign::Neg => Sign::Pos,
                Sign::Zero => Sign::Zero,
                Sign::NonNeg => Sign::NonPos,
                Sign::NonPos => Sign::NonNeg,
                Sign::Complex => Sign::Complex,
                Sign::Unknown => Sign::Unknown,
            },
            Expr::Add(a, b) => {
                let sa = self.csign(a);
                let sb = self.csign(b);
                if sa == Sign::Zero {
                    return sb;
                }
                if sb == Sign::Zero {
                    return sa;
                }
                if sa == Sign::Pos && sb == Sign::Pos {
                    return Sign::Pos;
                }
                if sa == Sign::Neg && sb == Sign::Neg {
                    return Sign::Neg;
                }
                if sa == Sign::NonNeg && sb == Sign::NonNeg {
                    return Sign::NonNeg;
                }
                if (sa == Sign::Pos && sb == Sign::NonNeg)
                    || (sa == Sign::NonNeg && sb == Sign::Pos)
                {
                    return Sign::Pos;
                }
                if (sa == Sign::Neg && sb == Sign::NonPos)
                    || (sa == Sign::NonPos && sb == Sign::Neg)
                {
                    return Sign::Neg;
                }
                if sa == Sign::NonPos && sb == Sign::NonPos {
                    return Sign::NonPos;
                }
                Sign::Unknown
            }
            Expr::Sub(a, b) => {
                if let (Expr::Const(ra), Expr::Const(rb)) = (a.as_ref(), b.as_ref()) {
                    return self.csign(&Expr::constant(ra - rb));
                }
                if a == b {
                    return Sign::Zero;
                }
                let sa = self.csign(a);
                let sb = self.csign(b);
                if sb == Sign::Zero {
                    return sa;
                }
                if sa == Sign::Zero {
                    return match sb {
                        Sign::Pos => Sign::Neg,
                        Sign::Neg => Sign::Pos,
                        Sign::NonNeg => Sign::NonPos,
                        Sign::NonPos => Sign::NonNeg,
                        other => other,
                    };
                }
                if sa == Sign::Pos && (sb == Sign::Neg || sb == Sign::NonPos) {
                    return Sign::Pos;
                }
                if sa == Sign::Neg && (sb == Sign::Pos || sb == Sign::NonNeg) {
                    return Sign::Neg;
                }
                if sa == Sign::NonNeg && sb == Sign::NonPos {
                    return Sign::NonNeg;
                }
                if sa == Sign::NonPos && sb == Sign::NonNeg {
                    return Sign::NonPos;
                }
                Sign::Unknown
            }
            Expr::Mul(a, b) => {
                let sa = self.csign(a);
                let sb = self.csign(b);
                if sa == Sign::Zero || sb == Sign::Zero {
                    return Sign::Zero;
                }
                match (sa, sb) {
                    (Sign::Pos, Sign::Pos) | (Sign::Neg, Sign::Neg) => Sign::Pos,
                    (Sign::Pos, Sign::Neg) | (Sign::Neg, Sign::Pos) => Sign::Neg,
                    (Sign::Pos, Sign::NonNeg) | (Sign::NonNeg, Sign::Pos) => Sign::NonNeg,
                    (Sign::Pos, Sign::NonPos)
                    | (Sign::NonPos, Sign::Pos)
                    | (Sign::Neg, Sign::NonNeg)
                    | (Sign::NonNeg, Sign::Neg) => Sign::NonPos,
                    (Sign::Neg, Sign::NonPos) | (Sign::NonPos, Sign::Neg) => Sign::NonNeg,
                    (Sign::NonNeg, Sign::NonNeg) | (Sign::NonPos, Sign::NonPos) => Sign::NonNeg,
                    (Sign::NonNeg, Sign::NonPos) | (Sign::NonPos, Sign::NonNeg) => Sign::NonPos,
                    (Sign::Complex, Sign::Pos | Sign::Neg)
                    | (Sign::Pos | Sign::Neg, Sign::Complex) => Sign::Complex,
                    _ => Sign::Unknown,
                }
            }
            Expr::Div(a, b) => {
                let sb = self.csign(b);
                if sb == Sign::Zero {
                    return Sign::Unknown;
                }
                let sa = self.csign(a);
                if sa == Sign::Zero {
                    return Sign::Zero;
                }
                match (sa, sb) {
                    (Sign::Pos, Sign::Pos) | (Sign::Neg, Sign::Neg) => Sign::Pos,
                    (Sign::Pos, Sign::Neg) | (Sign::Neg, Sign::Pos) => Sign::Neg,
                    (Sign::NonNeg, Sign::Pos) => Sign::NonNeg,
                    (Sign::NonNeg, Sign::Neg) => Sign::NonPos,
                    (Sign::NonPos, Sign::Pos) => Sign::NonPos,
                    (Sign::NonPos, Sign::Neg) => Sign::NonNeg,
                    _ => Sign::Unknown,
                }
            }
            Expr::Pow(base, exp) => {
                #[cfg(feature = "complex")]
                if matches!(base.as_ref(), Expr::I)
                    && let Expr::Const(r) = exp.as_ref()
                    && Rat::is_integer(r)
                {
                    let (u, v) = crate::complex::rectform(expr);
                    return if v.is_zero() {
                        self.csign(&u)
                    } else {
                        Sign::Complex
                    };
                }
                let sb = self.csign(base);
                let se = self.csign(exp);
                #[cfg(feature = "complex")]
                if sb == Sign::Complex || se == Sign::Complex {
                    return Sign::Complex;
                }
                if sb == Sign::Zero {
                    return match se {
                        Sign::Pos => Sign::Zero,
                        _ => Sign::Unknown,
                    };
                }
                if let Expr::Const(r) = exp.as_ref() {
                    if Rat::is_integer(r) {
                        let int_val = r.to_integer();
                        if &int_val % 2 == 0.into() {
                            return match sb {
                                Sign::Pos | Sign::Neg => Sign::Pos,
                                Sign::Zero => Sign::Zero,
                                _ => Sign::NonNeg,
                            };
                        } else {
                            return sb;
                        }
                    } else if sb == Sign::Neg {
                        return Sign::Complex;
                    }
                }
                if sb == Sign::Pos && se != Sign::Complex {
                    return Sign::Pos;
                }
                Sign::Unknown
            }
            Expr::Exp(inner) => {
                #[cfg(feature = "complex")]
                {
                    let (u, v) = crate::complex::rectform(expr);
                    if !v.is_zero() {
                        return Sign::Complex;
                    }
                    if u != *expr {
                        return self.csign(&u);
                    }
                }
                match self.csign(inner) {
                    Sign::Pos | Sign::Neg | Sign::Zero | Sign::NonNeg | Sign::NonPos => Sign::Pos,
                    Sign::Complex => Sign::Complex,
                    Sign::Unknown => Sign::Pos,
                }
            }
            Expr::Sqrt(inner) => match self.csign(inner) {
                Sign::Pos => Sign::Pos,
                Sign::Zero => Sign::Zero,
                Sign::NonNeg => Sign::NonNeg,
                Sign::Neg => Sign::Complex,
                _ => Sign::Unknown,
            },
            _ => Sign::Unknown,
        }
    }

    /// Query the sign of an expression (non-interactive proxy for `csign`).
    ///
    /// Maxima `src/askp.lisp` (`asksign`).
    pub fn asksign(&self, expr: &Expr) -> Sign {
        self.csign(expr)
    }

    /// Check if expression possesses a domain property.
    pub fn has_property(&self, expr: &Expr, prop: PropertyKind) -> PredicateResult {
        match expr {
            Expr::Const(r) => match prop {
                PropertyKind::Real | PropertyKind::Rational => PredicateResult::True,
                PropertyKind::Complex => PredicateResult::False,
                PropertyKind::Integer => {
                    if Rat::is_integer(r) {
                        PredicateResult::True
                    } else {
                        PredicateResult::False
                    }
                }
                PropertyKind::NonInteger => {
                    if !Rat::is_integer(r) {
                        PredicateResult::True
                    } else {
                        PredicateResult::False
                    }
                }
                PropertyKind::Even => {
                    if Rat::is_integer(r) && &r.to_integer() % 2 == 0.into() {
                        PredicateResult::True
                    } else {
                        PredicateResult::False
                    }
                }
                PropertyKind::Odd => {
                    if Rat::is_integer(r) && &r.to_integer() % 2 != 0.into() {
                        PredicateResult::True
                    } else {
                        PredicateResult::False
                    }
                }
                PropertyKind::Pos => {
                    if r > &BigRational::from_integer(0.into()) {
                        PredicateResult::True
                    } else {
                        PredicateResult::False
                    }
                }
                PropertyKind::Neg => {
                    if r < &BigRational::from_integer(0.into()) {
                        PredicateResult::True
                    } else {
                        PredicateResult::False
                    }
                }
                PropertyKind::Zero => {
                    if r.is_zero() {
                        PredicateResult::True
                    } else {
                        PredicateResult::False
                    }
                }
                PropertyKind::NonZero => {
                    if !r.is_zero() {
                        PredicateResult::True
                    } else {
                        PredicateResult::False
                    }
                }
                _ => PredicateResult::Unknown,
            },
            Expr::Var(v) => {
                if let Some(props) = self.var_props.get(v)
                    && props.contains(&prop)
                {
                    return PredicateResult::True;
                }
                PredicateResult::Unknown
            }
            _ => PredicateResult::Unknown,
        }
    }

    // Helper: apply relation internally
    fn apply_relation(&mut self, lhs: &Expr, op: Comparison, rhs: &Expr) -> Result<(), String> {
        if lhs == rhs {
            return match op {
                Comparison::Equal
                | Comparison::GreaterThanOrEqual
                | Comparison::LessThanOrEqual => Ok(()),
                Comparison::GreaterThan | Comparison::LessThan | Comparison::NotEqual => {
                    Err(format!("Contradictory self-relation: {lhs} {op} {rhs}"))
                }
            };
        }

        // Check DAG contradictions before mutating
        match op {
            Comparison::GreaterThan => {
                if self.dag_path(rhs, lhs).is_some() {
                    return Err(format!("Contradictory inequality: {lhs} > {rhs}"));
                }
            }
            Comparison::GreaterThanOrEqual => {
                if self.dag_path(rhs, lhs) == Some(true) {
                    return Err(format!("Contradictory inequality: {lhs} >= {rhs}"));
                }
            }
            Comparison::LessThan => {
                if self.dag_path(lhs, rhs).is_some() {
                    return Err(format!("Contradictory inequality: {lhs} < {rhs}"));
                }
            }
            Comparison::LessThanOrEqual => {
                if self.dag_path(lhs, rhs) == Some(true) {
                    return Err(format!("Contradictory inequality: {lhs} <= {rhs}"));
                }
            }
            Comparison::Equal => {
                if self.dag_path(lhs, rhs) == Some(true)
                    || self.dag_path(rhs, lhs) == Some(true)
                    || self.has_not_equal(lhs, rhs)
                {
                    return Err(format!("Contradictory equality: {lhs} = {rhs}"));
                }
            }
            Comparison::NotEqual => {
                if self.dag_path(lhs, rhs) == Some(false) && self.dag_path(rhs, lhs) == Some(false)
                {
                    return Err(format!("Contradictory disequality: {lhs} != {rhs}"));
                }
            }
        }

        // Var op Const
        if let (Expr::Var(v), Expr::Const(r)) = (lhs, rhs) {
            let b = self.var_bounds.entry(v.clone()).or_default();
            match op {
                Comparison::GreaterThan => b.update_lower(r.clone(), true)?,
                Comparison::GreaterThanOrEqual => b.update_lower(r.clone(), false)?,
                Comparison::LessThan => b.update_upper(r.clone(), true)?,
                Comparison::LessThanOrEqual => b.update_upper(r.clone(), false)?,
                Comparison::Equal => {
                    b.update_lower(r.clone(), false)?;
                    b.update_upper(r.clone(), false)?;
                }
                Comparison::NotEqual => {}
            }
        }
        // Const op Var (inverted)
        else if let (Expr::Const(r), Expr::Var(v)) = (lhs, rhs) {
            let b = self.var_bounds.entry(v.clone()).or_default();
            match op {
                Comparison::GreaterThan => b.update_upper(r.clone(), true)?,
                Comparison::GreaterThanOrEqual => b.update_upper(r.clone(), false)?,
                Comparison::LessThan => b.update_lower(r.clone(), true)?,
                Comparison::LessThanOrEqual => b.update_lower(r.clone(), false)?,
                Comparison::Equal => {
                    b.update_lower(r.clone(), false)?;
                    b.update_upper(r.clone(), false)?;
                }
                Comparison::NotEqual => {}
            }
        }

        match op {
            Comparison::GreaterThan => self.edges.push((lhs.clone(), true, rhs.clone())),
            Comparison::GreaterThanOrEqual => self.edges.push((lhs.clone(), false, rhs.clone())),
            Comparison::LessThan => self.edges.push((rhs.clone(), true, lhs.clone())),
            Comparison::LessThanOrEqual => self.edges.push((rhs.clone(), false, lhs.clone())),
            Comparison::Equal => {
                self.edges.push((lhs.clone(), false, rhs.clone()));
                self.edges.push((rhs.clone(), false, lhs.clone()));
            }
            Comparison::NotEqual => {
                self.not_equals.insert((lhs.clone(), rhs.clone()));
                self.not_equals.insert((rhs.clone(), lhs.clone()));
            }
        }
        Ok(())
    }

    // Helper: apply domain property internally
    fn apply_property(&mut self, expr: &Expr, prop: PropertyKind) -> Result<(), String> {
        if let Expr::Var(v) = expr {
            let props = self.var_props.entry(v.clone()).or_default();
            let incompatible = [
                (PropertyKind::Integer, PropertyKind::NonInteger),
                (PropertyKind::Even, PropertyKind::Odd),
                (PropertyKind::Rational, PropertyKind::Irrational),
                (PropertyKind::Real, PropertyKind::Complex),
                (PropertyKind::Pos, PropertyKind::Neg),
                (PropertyKind::Pos, PropertyKind::Zero),
                (PropertyKind::Neg, PropertyKind::Zero),
                (PropertyKind::Zero, PropertyKind::NonZero),
            ];
            for (a, b) in incompatible {
                if (prop == a && props.contains(&b)) || (prop == b && props.contains(&a)) {
                    return Err(format!("Contradiction: {v} cannot be both {a} and {b}"));
                }
            }
            props.insert(prop);
        }
        Ok(())
    }

    fn has_not_equal(&self, a: &Expr, b: &Expr) -> bool {
        self.not_equals.contains(&(a.clone(), b.clone()))
    }

    /// Returns `Some(true)` if `start > target` is derivable (path with >=1 strict edge),
    /// `Some(false)` if `start >= target` is derivable (non-strict path), or `None` if unreachable.
    fn dag_path(&self, start: &Expr, target: &Expr) -> Option<bool> {
        if start == target {
            return Some(false);
        }
        // BFS tracking best (strict=true > strict=false)
        let mut best: HashMap<Expr, bool> = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        best.insert(start.clone(), false);
        queue.push_back((start.clone(), false));

        while let Some((curr, curr_strict)) = queue.pop_front() {
            for (u, edge_strict, v) in &self.edges {
                if *u == curr {
                    let next_strict = curr_strict || *edge_strict;
                    let should_visit = match best.get(v) {
                        None => true,
                        Some(&prev) => next_strict && !prev,
                    };
                    if should_visit {
                        best.insert(v.clone(), next_strict);
                        queue.push_back((v.clone(), next_strict));
                    }
                }
            }
        }
        best.get(target).copied()
    }
}

/// Standalone helper: assume a single fact and return an [`Assumptions`] context.
pub fn assume(lhs: Expr, op: Comparison, rhs: Expr) -> Result<Assumptions, String> {
    Assumptions::new().assume_rel(lhs, op, rhs)
}

/// Standalone helper: forget a fact from an [`Assumptions`] context.
pub fn forget(assumptions: &Assumptions, fact: &Fact) -> Assumptions {
    assumptions.forget(fact)
}

/// Standalone helper: compute sign using optional assumptions context.
pub fn csign(expr: &Expr, assumptions: Option<&Assumptions>) -> Sign {
    if let Some(ctx) = assumptions {
        ctx.csign(expr)
    } else {
        Assumptions::new().csign(expr)
    }
}

/// Standalone helper: asksign query using optional assumptions context.
pub fn asksign(expr: &Expr, assumptions: Option<&Assumptions>) -> Sign {
    csign(expr, assumptions)
}

/// Standalone helper: evaluate predicate using optional assumptions context.
pub fn is(
    lhs: &Expr,
    op: Comparison,
    rhs: &Expr,
    assumptions: Option<&Assumptions>,
) -> PredicateResult {
    if let Some(ctx) = assumptions {
        ctx.is(lhs, op, rhs)
    } else {
        Assumptions::new().is(lhs, op, rhs)
    }
}
