//! Golden integration tests for Common Subexpression Elimination (CSE).

use maxima_rs::prelude::*;
use sym_core::cse::eliminate_common_subexprs;

#[test]
fn test_cse_shared_transcendental() {
    let x = Expr::var("x");

    // e1 = sin(x) + 1
    // e2 = sin(x) * 2
    let e1 = Expr::add(Expr::sin(x.clone()), Expr::int(1));
    let e2 = Expr::mul(Expr::sin(x.clone()), Expr::int(2));

    let cse = eliminate_common_subexprs(&[e1, e2]);

    // sin(x) should be extracted into a let binding
    assert!(
        !cse.bindings.is_empty(),
        "Expected at least one CSE binding"
    );
    let bound_expr = &cse.bindings[0].expr;
    assert_eq!(*bound_expr, Expr::sin(x));

    // Both outputs should now reference the bound variable
    let b_name = &cse.bindings[0].name;
    assert_eq!(cse.outputs.len(), 2);
    assert_eq!(cse.outputs[0], Expr::add(Expr::var(b_name), Expr::int(1)));
    assert_eq!(cse.outputs[1], Expr::mul(Expr::var(b_name), Expr::int(2)));
}

#[test]
fn test_cse_no_duplicates_no_bindings() {
    let x = Expr::var("x");
    let y = Expr::var("y");

    let e1 = Expr::add(x, Expr::int(1));
    let e2 = Expr::mul(y, Expr::int(2));

    let cse = eliminate_common_subexprs(&[e1.clone(), e2.clone()]);
    assert!(cse.bindings.is_empty());
    assert_eq!(cse.outputs, vec![e1, e2]);
}
