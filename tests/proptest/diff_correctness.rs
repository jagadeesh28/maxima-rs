//! Property-based testing: symbolic differentiation vs numerical central finite differences.

use maxima_rs::prelude::*;
use proptest::prelude::*;
use std::collections::HashMap;

proptest! {
    #[test]
    fn prop_diff_matches_finite_difference_polynomial(x_val in -10.0f64..10.0f64) {
        let x = Expr::var("x");
        // f(x) = x^3 - 3*x^2 + 2*x + 1
        let f = Expr::add(
            Expr::add(
                Expr::pow(x.clone(), Expr::int(3)),
                Expr::neg(Expr::mul(Expr::int(3), Expr::pow(x.clone(), Expr::int(2)))),
            ),
            Expr::add(
                Expr::mul(Expr::int(2), x.clone()),
                Expr::int(1),
            ),
        );

        let df = differentiate(&f, "x");

        let h = 1e-6;
        let mut env_plus = HashMap::new();
        env_plus.insert("x".to_string(), x_val + h);
        let mut env_minus = HashMap::new();
        env_minus.insert("x".to_string(), x_val - h);
        let mut env_at = HashMap::new();
        env_at.insert("x".to_string(), x_val);

        let f_plus = f.eval(&env_plus).expect("eval f_plus");
        let f_minus = f.eval(&env_minus).expect("eval f_minus");
        let num_df = (f_plus - f_minus) / (2.0 * h);

        let sym_df = df.eval(&env_at).expect("eval sym_df");

        let diff = (sym_df - num_df).abs();
        let rel_err = diff / (sym_df.abs().max(1.0));
        prop_assert!(rel_err < 1e-4, "x_val={x_val}: sym={sym_df}, num={num_df}, rel_err={rel_err}");
    }

    #[test]
    fn prop_diff_matches_finite_difference_trig(x_val in -3.0f64..3.0f64) {
        let x = Expr::var("x");
        // f(x) = sin(x) * cos(x)
        let f = Expr::mul(Expr::sin(x.clone()), Expr::cos(x.clone()));

        let df = differentiate(&f, "x");

        let h = 1e-6;
        let mut env_plus = HashMap::new();
        env_plus.insert("x".to_string(), x_val + h);
        let mut env_minus = HashMap::new();
        env_minus.insert("x".to_string(), x_val - h);
        let mut env_at = HashMap::new();
        env_at.insert("x".to_string(), x_val);

        let f_plus = f.eval(&env_plus).expect("eval f_plus");
        let f_minus = f.eval(&env_minus).expect("eval f_minus");
        let num_df = (f_plus - f_minus) / (2.0 * h);

        let sym_df = df.eval(&env_at).expect("eval sym_df");

        let diff = (sym_df - num_df).abs();
        let rel_err = diff / (sym_df.abs().max(1.0));
        prop_assert!(rel_err < 1e-4, "x_val={x_val}: sym={sym_df}, num={num_df}, rel_err={rel_err}");
    }
}
