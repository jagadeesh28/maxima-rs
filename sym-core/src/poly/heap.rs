//! Monagan-Pearce heap-based polynomial multiplication and pseudo-division.
//!
//! Both input polynomials are `grevlex`-sorted, so the product terms form a set of
//! sorted streams (one per term of the first factor). A binary max-heap merges those
//! streams in descending monomial order while accumulating equal monomials.

use super::monomial::Monomial;
use super::{Poly, Term, canonical_ctx, remap_terms};
use num_rational::BigRational;
use num_traits::Zero;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// A pending product `a_i * b_j` in the heap merge.
struct HeapItem {
    mono: Monomial,
    coeff: BigRational,
    i: usize,
    j: usize,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.mono == other.mono && self.i == other.i && self.j == other.j
    }
}

impl Eq for HeapItem {}

impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        self.mono
            .cmp(&other.mono)
            .then_with(|| self.i.cmp(&other.i))
            .then_with(|| self.j.cmp(&other.j))
    }
}

/// Multiply two polynomials using a heap merge over `grevlex`-ordered streams.
pub fn heap_mul(a: &Poly, b: &Poly) -> Poly {
    let ctx = canonical_ctx(&a.ctx, &b.ctx);
    let mut ta = remap_terms(a, &ctx);
    let mut tb = remap_terms(b, &ctx);
    ta.sort_by(|x, y| y.mono.cmp(&x.mono));
    tb.sort_by(|x, y| y.mono.cmp(&x.mono));
    if ta.is_empty() || tb.is_empty() {
        return Poly {
            terms: Vec::new(),
            ctx,
        };
    }

    let b0 = &tb[0];
    let mut heap: BinaryHeap<HeapItem> = BinaryHeap::with_capacity(ta.len());
    for (i, t) in ta.iter().enumerate() {
        heap.push(HeapItem {
            mono: t.mono.mul(&b0.mono),
            coeff: &t.coeff * &b0.coeff,
            i,
            j: 0,
        });
    }

    let mut result: Vec<Term> = Vec::new();
    while let Some(item) = heap.pop() {
        if item.j + 1 < tb.len() {
            let j = item.j + 1;
            let t = &ta[item.i];
            let u = &tb[j];
            heap.push(HeapItem {
                mono: t.mono.mul(&u.mono),
                coeff: &t.coeff * &u.coeff,
                i: item.i,
                j,
            });
        }
        if let Some(last) = result.last_mut()
            && last.mono == item.mono
        {
            last.coeff = &last.coeff + &item.coeff;
            continue;
        }
        result.push(Term {
            coeff: item.coeff,
            mono: item.mono,
        });
    }
    result.retain(|t| !t.coeff.is_zero());
    Poly { terms: result, ctx }
}

/// Pseudo-divide univariate polynomial `a` by `b`.
///
/// Returns `(q, r, s)` such that `s * a = q * b + r` and `deg(r) < deg(b)`.
/// Only single-variable polynomials with a non-constant divisor are supported.
pub fn pseudo_divmod(a: &Poly, b: &Poly) -> Result<(Poly, Poly, BigRational), super::PolyError> {
    use super::PolyError;
    use num_traits::One;

    if b.is_zero() {
        return Err(PolyError::DivisionByZero);
    }
    let ctx = canonical_ctx(&a.ctx, &b.ctx);
    if ctx.len() > 1 {
        return Err(PolyError::NotPolynomial(
            "pseudo-division requires univariate polynomials",
        ));
    }
    let mut r = remap_terms(a, &ctx);
    let mut bb = remap_terms(b, &ctx);
    r.sort_by(|x, y| y.mono.cmp(&x.mono));
    bb.sort_by(|x, y| y.mono.cmp(&x.mono));

    let lc_b = bb[0].coeff.clone();
    let db = bb[0].mono.total_degree();
    if db == 0 {
        return Err(PolyError::NotPolynomial(
            "pseudo-division requires a non-constant divisor",
        ));
    }

    let mut q: Vec<Term> = Vec::new();
    let mut s = BigRational::one();
    while let Some(lead) = r.first().cloned() {
        let dr = lead.mono.total_degree();
        if dr < db {
            break;
        }
        let diff = u32::try_from(dr - db).map_err(|_| PolyError::ExponentOverflow)?;
        let mon = Monomial::from_exps(vec![diff]);

        for t in &mut q {
            t.coeff = &t.coeff * &lc_b;
        }
        q.push(Term {
            coeff: lead.coeff.clone(),
            mono: mon.clone(),
        });

        let sub: Vec<Term> = bb
            .iter()
            .map(|bt| Term {
                coeff: &lead.coeff * &bt.coeff,
                mono: bt.mono.mul(&mon),
            })
            .collect();
        let mut nr: Vec<Term> = r
            .iter()
            .map(|t| Term {
                coeff: &t.coeff * &lc_b,
                mono: t.mono.clone(),
            })
            .collect();
        nr.extend(sub.into_iter().map(|t| Term {
            coeff: -t.coeff,
            mono: t.mono,
        }));
        r = super::normalize_terms(nr);
        s = &s * &lc_b;
    }

    Ok((
        Poly {
            terms: super::normalize_terms(q),
            ctx: ctx.clone(),
        },
        Poly {
            terms: super::normalize_terms(r),
            ctx,
        },
        s,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::Expr;
    use crate::rational::Rat;

    #[test]
    fn heap_mul_square() {
        let x = Expr::var("x");
        let y = Expr::var("y");
        let a = Poly::from_expr(&Expr::add(x.clone(), y.clone())).unwrap();
        let b = Poly::from_expr(&Expr::add(x, y)).unwrap();
        let p = heap_mul(&a, &b);
        assert_eq!(p.terms().len(), 3);
    }

    #[test]
    fn heap_mul_constants() {
        let a = Poly::from_expr(&Expr::int(3)).unwrap();
        let b = Poly::from_expr(&Expr::int(4)).unwrap();
        let p = heap_mul(&a, &b);
        assert_eq!(p.constant_value().unwrap(), Rat::from_int(12));
    }

    #[test]
    fn pseudo_division_divides_exactly() {
        let x = Expr::var("x");
        let a =
            Poly::from_expr(&Expr::sub(Expr::pow(x.clone(), Expr::int(2)), Expr::int(1))).unwrap();
        let b = Poly::from_expr(&Expr::sub(x, Expr::int(1))).unwrap();
        let (q, r, _s) = pseudo_divmod(&a, &b).unwrap();
        assert!(r.is_zero());
        assert_eq!(q.terms().len(), 2);
    }
}
