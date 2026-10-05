//! Property-based testing: numerical equivalence between original and e-graph-optimized expressions.

use maxima_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashMap;
use sym_core::saturate::SaturationConfig;

proptest! {
    #[test]
    fn prop_pythagorean_identity_numeric_equiv(x_val in -100.0f64..100.0f64) {
        let x = Expr::var("x");
        let orig = Expr::add(
            Expr::pow(Expr::sin(x.clone()), Expr::int(2)),
            Expr::pow(Expr::cos(x.clone()), Expr::int(2)),
        );

        let opt = optimize(&orig, &SaturationConfig::default());

        let mut env = HashMap::new();
        env.insert("x".to_string(), x_val);

        let val_orig = orig.eval(&env).expect("eval orig");
        let val_opt = opt.eval(&env).expect("eval opt");

        prop_assert!((val_orig - val_opt).abs() < 1e-10, "val_orig = {val_orig}, val_opt = {val_opt}");
        prop_assert!((val_opt - 1.0).abs() < 1e-10, "opt is not 1.0: {val_opt}");
    }

    #[test]
    fn prop_arithmetic_linear_numeric_equiv(
        x_val in -50.0f64..50.0f64,
        y_val in -50.0f64..50.0f64,
        z_val in -50.0f64..50.0f64
    ) {
        let x = Expr::var("x");
        let y = Expr::var("y");
        let z = Expr::var("z");

        // ((x + y) * z) / z -> x + y (when z != 0)
        prop_assume!(z_val.abs() > 0.01);

        let orig = Expr::div(
            Expr::mul(Expr::add(x.clone(), y.clone()), z.clone()),
            z.clone(),
        );

        let opt = optimize(&orig, &SaturationConfig::default());

        let mut env = HashMap::new();
        env.insert("x".to_string(), x_val);
        env.insert("y".to_string(), y_val);
        env.insert("z".to_string(), z_val);

        let val_orig = orig.eval(&env).expect("eval orig");
        let val_opt = opt.eval(&env).expect("eval opt");

        prop_assert!((val_orig - val_opt).abs() < 1e-7, "val_orig = {val_orig}, val_opt = {val_opt}");
    }
}
