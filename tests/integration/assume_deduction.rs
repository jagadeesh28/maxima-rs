//! Integration tests for scoped deductive inference and assumptions context (Phase 44).
//!
//! Citations:
//! - Maxima 5.50: `src/compar.lisp` and `src/askp.lisp`.
//! - ADR-0011: Special Functions, Operational Laplace Transforms, and Scoped Assumptions Architecture.

use maxima_rs::prelude::*;
use sym_core::integrate::integrate_with_assumptions;
use sym_core::limit::limit_with_assumptions;

#[test]
fn test_assume_basic_bounds() {
    let ctx = Assumptions::new();
    let x = Expr::var("x");

    // Before assumptions: unknown sign
    assert_eq!(ctx.csign(&x), Sign::Unknown);
    assert_eq!(
        ctx.is(&x, Comparison::GreaterThan, &Expr::int(0)),
        PredicateResult::Unknown
    );

    // Assume x > 0
    let ctx_pos = ctx
        .assume_rel(x.clone(), Comparison::GreaterThan, Expr::int(0))
        .expect("assume x > 0");

    assert_eq!(ctx_pos.csign(&x), Sign::Pos);
    assert_eq!(
        ctx_pos.is(&x, Comparison::GreaterThan, &Expr::int(0)),
        PredicateResult::True
    );
    assert_eq!(
        ctx_pos.is(&x, Comparison::GreaterThanOrEqual, &Expr::int(0)),
        PredicateResult::True
    );
    assert_eq!(
        ctx_pos.is(&x, Comparison::LessThan, &Expr::int(0)),
        PredicateResult::False
    );
    assert_eq!(
        ctx_pos.is(&x, Comparison::Equal, &Expr::int(0)),
        PredicateResult::False
    );

    // Assume y <= -3
    let y = Expr::var("y");
    let ctx_y = ctx_pos
        .assume_rel(y.clone(), Comparison::LessThanOrEqual, Expr::int(-3))
        .expect("assume y <= -3");

    assert_eq!(ctx_y.csign(&y), Sign::Neg);
    assert_eq!(
        ctx_y.is(&y, Comparison::LessThan, &Expr::int(0)),
        PredicateResult::True
    );
    assert_eq!(
        ctx_y.is(&y, Comparison::LessThanOrEqual, &Expr::int(-3)),
        PredicateResult::True
    );
    assert_eq!(
        ctx_y.is(&y, Comparison::GreaterThan, &Expr::int(0)),
        PredicateResult::False
    );
}

#[test]
fn test_assume_contradiction_detection() {
    let ctx = Assumptions::new();
    let x = Expr::var("x");

    // Assume x > 5
    let ctx2 = ctx
        .assume_rel(x.clone(), Comparison::GreaterThan, Expr::int(5))
        .expect("assume x > 5");

    // Attempt contradictory assumption x < 2
    let res = ctx2.assume_rel(x.clone(), Comparison::LessThan, Expr::int(2));
    assert!(
        res.is_err(),
        "Contradictory lower > upper bounds must be rejected"
    );

    // Attempt contradictory strict bound x < 5
    let res_strict = ctx2.assume_rel(x.clone(), Comparison::LessThan, Expr::int(5));
    assert!(
        res_strict.is_err(),
        "Contradictory strict lower == upper bounds must be rejected"
    );

    // Attempt contradictory domain properties
    let n = Expr::var("n");
    let ctx_n = ctx
        .assume_prop(n.clone(), PropertyKind::Integer)
        .expect("assume n integer");
    let res_prop = ctx_n.assume_prop(n.clone(), PropertyKind::NonInteger);
    assert!(
        res_prop.is_err(),
        "Contradictory integer vs non-integer properties must be rejected"
    );
}

#[test]
fn test_transitive_inequalities_dag() {
    let ctx = Assumptions::new();
    let a = Expr::var("a");
    let b = Expr::var("b");
    let c = Expr::var("c");

    // Assume a > b and b > c
    let ctx = ctx
        .assume_rel(a.clone(), Comparison::GreaterThan, b.clone())
        .expect("assume a > b")
        .assume_rel(b.clone(), Comparison::GreaterThan, c.clone())
        .expect("assume b > c");

    // Transitive deduction: a > c must be true
    assert_eq!(
        ctx.is(&a, Comparison::GreaterThan, &c),
        PredicateResult::True
    );
    assert_eq!(
        ctx.is(&a, Comparison::GreaterThanOrEqual, &c),
        PredicateResult::True
    );
}

#[test]
fn test_arithmetic_sign_propagation() {
    let ctx = Assumptions::new();
    let x = Expr::var("x");
    let y = Expr::var("y");

    let ctx = ctx
        .assume_rel(x.clone(), Comparison::GreaterThan, Expr::int(0))
        .expect("x > 0")
        .assume_rel(y.clone(), Comparison::LessThan, Expr::int(0))
        .expect("y < 0");

    // x * y must be negative
    let xy = Expr::mul(x.clone(), y.clone());
    assert_eq!(ctx.csign(&xy), Sign::Neg);

    // x - y must be positive (Pos - Neg = Pos)
    let x_minus_y = Expr::sub(x.clone(), y.clone());
    assert_eq!(ctx.csign(&x_minus_y), Sign::Pos);

    // x^2 must be positive
    let x_squared = Expr::pow(x.clone(), Expr::int(2));
    assert_eq!(ctx.csign(&x_squared), Sign::Pos);

    // exp(y) must be positive
    let exp_y = Expr::exp(y.clone());
    assert_eq!(ctx.csign(&exp_y), Sign::Pos);
}

#[test]
fn test_domain_properties() {
    let ctx = Assumptions::new();
    let n = Expr::var("n");

    let ctx = ctx
        .assume_prop(n.clone(), PropertyKind::Integer)
        .expect("n integer")
        .assume_prop(n.clone(), PropertyKind::Even)
        .expect("n even");

    assert_eq!(
        ctx.has_property(&n, PropertyKind::Integer),
        PredicateResult::True
    );
    assert_eq!(
        ctx.has_property(&n, PropertyKind::Even),
        PredicateResult::True
    );
    assert_eq!(
        ctx.has_property(&n, PropertyKind::Odd),
        PredicateResult::Unknown
    );

    // Numeric constants satisfy built-in properties
    assert_eq!(
        ctx.has_property(&Expr::int(4), PropertyKind::Even),
        PredicateResult::True
    );
    assert_eq!(
        ctx.has_property(&Expr::int(7), PropertyKind::Odd),
        PredicateResult::True
    );
    assert_eq!(
        ctx.has_property(&Expr::frac(1, 2), PropertyKind::Integer),
        PredicateResult::False
    );
}

#[test]
fn test_forget_mechanism() {
    let ctx = Assumptions::new();
    let x = Expr::var("x");

    let ctx_pos = ctx
        .assume_rel(x.clone(), Comparison::GreaterThan, Expr::int(0))
        .expect("assume x > 0");
    assert_eq!(ctx_pos.csign(&x), Sign::Pos);

    let fact = Fact::Rel(x.clone(), Comparison::GreaterThan, Expr::int(0));
    let ctx_forgotten = ctx_pos.forget(&fact);

    // After forget, sign returns to unknown
    assert_eq!(ctx_forgotten.csign(&x), Sign::Unknown);
    assert_eq!(
        ctx_forgotten.is(&x, Comparison::GreaterThan, &Expr::int(0)),
        PredicateResult::Unknown
    );
}

#[test]
fn test_integration_with_assumptions() {
    let x = Expr::var("x");
    let a = Expr::var("a");
    let integrand = Expr::pow(x.clone(), a.clone());

    // Without assumptions: integral noun form (a might be -1)
    let res_no_assume = sym_core::integrate::integrate(&integrand, "x");
    assert!(matches!(res_no_assume, Expr::Integral(_, _)));

    // With assumption a > 0: a != -1 guaranteed => x^(a+1) / (a+1)
    let ctx = Assumptions::new()
        .assume_rel(a.clone(), Comparison::GreaterThan, Expr::int(0))
        .expect("assume a > 0");

    let res_assume = integrate_with_assumptions(&integrand, "x", Some(&ctx));
    let expected = Expr::div(
        Expr::pow(x, Expr::add(a.clone(), Expr::int(1))),
        Expr::add(a, Expr::int(1)),
    );
    assert_eq!(res_assume, expected);
}

#[test]
fn test_limit_with_assumptions() {
    let x = Expr::var("x");
    let a = Expr::var("a");
    let expr = Expr::pow(x.clone(), a.clone());
    let inf = Expr::var("inf");

    // With assumption a > 0: lim_{x -> inf} x^a = inf
    let ctx_pos = Assumptions::new()
        .assume_rel(a.clone(), Comparison::GreaterThan, Expr::int(0))
        .expect("assume a > 0");

    let lim_pos = limit_with_assumptions(&expr, "x", &inf, Some(&ctx_pos));
    assert_eq!(lim_pos, Some(Expr::var("inf")));

    // With assumption a < 0: lim_{x -> inf} x^a = 0
    let ctx_neg = Assumptions::new()
        .assume_rel(a.clone(), Comparison::LessThan, Expr::int(0))
        .expect("assume a < 0");

    let lim_neg = limit_with_assumptions(&expr, "x", &inf, Some(&ctx_neg));
    assert_eq!(lim_neg, Some(Expr::int(0)));
}
