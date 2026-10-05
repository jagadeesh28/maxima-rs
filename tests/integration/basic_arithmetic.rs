//! Golden integration tests for basic arithmetic simplification and Maxima provenance.

use maxima_rs::prelude::*;
use sym_core::saturate::SaturationConfig;

#[test]
fn test_addition_identities() {
    let config = SaturationConfig::default();
    let x = Expr::var("x");

    // x + 0 -> x
    let e1 = Expr::add(x.clone(), Expr::int(0));
    assert_eq!(optimize(&e1, &config), x);

    // 0 + x -> x
    let e2 = Expr::add(Expr::int(0), x.clone());
    assert_eq!(optimize(&e2, &config), x);

    // x - 0 -> x
    let e3 = Expr::sub(x.clone(), Expr::int(0));
    assert_eq!(optimize(&e3, &config), x);

    // x - x -> 0
    let e4 = Expr::sub(x.clone(), x.clone());
    assert_eq!(optimize(&e4, &config), Expr::int(0));
}

#[test]
fn test_multiplication_identities() {
    let config = SaturationConfig::default();
    let x = Expr::var("x");

    // x * 1 -> x
    let e1 = Expr::mul(x.clone(), Expr::int(1));
    assert_eq!(optimize(&e1, &config), x);

    // 1 * x -> x
    let e2 = Expr::mul(Expr::int(1), x.clone());
    assert_eq!(optimize(&e2, &config), x);

    // x * 0 -> 0
    let e3 = Expr::mul(x.clone(), Expr::int(0));
    assert_eq!(optimize(&e3, &config), Expr::int(0));

    // 0 * x -> 0
    let e4 = Expr::mul(Expr::int(0), x.clone());
    assert_eq!(optimize(&e4, &config), Expr::int(0));

    // x / 1 -> x
    let e5 = Expr::div(x.clone(), Expr::int(1));
    assert_eq!(optimize(&e5, &config), x);

    // x / x -> 1
    let e6 = Expr::div(x.clone(), x.clone());
    assert_eq!(optimize(&e6, &config), Expr::int(1));
}

#[test]
fn test_power_identities() {
    let config = SaturationConfig::default();
    let x = Expr::var("x");

    // x^1 -> x
    let e1 = Expr::pow(x.clone(), Expr::int(1));
    assert_eq!(optimize(&e1, &config), x);

    // x^0 -> 1
    let e2 = Expr::pow(x.clone(), Expr::int(0));
    assert_eq!(optimize(&e2, &config), Expr::int(1));

    // 1^x -> 1
    let e3 = Expr::pow(Expr::int(1), x.clone());
    assert_eq!(optimize(&e3, &config), Expr::int(1));

    // 0^x -> 0
    let e4 = Expr::pow(Expr::int(0), x.clone());
    assert_eq!(optimize(&e4, &config), Expr::int(0));
}

#[test]
fn test_constant_folding() {
    let config = SaturationConfig::default();

    // 2 + 3 -> 5
    let e1 = Expr::add(Expr::int(2), Expr::int(3));
    assert_eq!(optimize(&e1, &config), Expr::int(5));

    // 4 * 5 -> 20
    let e2 = Expr::mul(Expr::int(4), Expr::int(5));
    assert_eq!(optimize(&e2, &config), Expr::int(20));

    // 1/2 + 1/3 -> 5/6
    let e3 = Expr::add(Expr::frac(1, 2), Expr::frac(1, 3));
    assert_eq!(optimize(&e3, &config), Expr::frac(5, 6));

    // 2^3 -> 8
    let e4 = Expr::pow(Expr::int(2), Expr::int(3));
    assert_eq!(optimize(&e4, &config), Expr::int(8));
}
