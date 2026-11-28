//! Initial and boundary condition solvers for Ordinary Differential Equations (`ic1`, `ic2`, `bc2`).
//!
//! Citations:
//! - Maxima 5.50: `src/ode2.lisp` (`$ic1`, `$ic2`, `$bc2`).
//! - ADR-0011: Special Functions, Operational Laplace Transforms, and Scoped Assumptions Architecture.

use crate::diff::differentiate;
use crate::expr::Expr;
use crate::poly::{expand, ratsimp};
use crate::solve::ode::OdeSolution;
use crate::transform::laplace::subst;

fn simplify_bvp_expr(e: &Expr) -> Expr {
    ratsimp(e).unwrap_or_else(|_| expand(e))
}

fn is_bvp_zero(e: &Expr) -> bool {
    e.is_zero() || simplify_bvp_expr(e).is_zero()
}

/// Solve 1st-order Initial Value Problem (IVP):
///
/// Given a general solution $y(x, C)$, finds $C$ such that $y(x_0) = y_0$.
///
/// Maxima `src/ode2.lisp` (`$ic1`).
pub fn ic1(sol: &Expr, x_var: &str, c_var: &str, x0: &Expr, y0: &Expr) -> Result<Expr, String> {
    // Evaluate y(x0, C)
    let y_at_x0 = subst(sol, x_var, x0);

    // Linear extraction in C: y(x0, C) = a*C + b
    let b = subst(&y_at_x0, c_var, &Expr::int(0));
    let a_plus_b = subst(&y_at_x0, c_var, &Expr::int(1));
    let a = Expr::sub(a_plus_b, b.clone());

    if is_bvp_zero(&a) {
        return Err("Constant of integration does not affect initial value".to_string());
    }

    // a*C + b = y0 => C = (y0 - b) / a
    let c_val = simplify_bvp_expr(&Expr::div(Expr::sub(y0.clone(), b), a));

    // Substitute C back into the general solution
    Ok(simplify_bvp_expr(&subst(sol, c_var, &c_val)))
}

/// Solve 1st-order IVP directly from an [`OdeSolution`].
pub fn ic1_ode(ode: &OdeSolution, x_var: &str, x0: &Expr, y0: &Expr) -> Result<Expr, String> {
    let c_var = ode
        .constants
        .first()
        .cloned()
        .unwrap_or_else(|| "_C1".to_string());
    ic1(&ode.solution, x_var, &c_var, x0, y0)
}

/// Solve 2nd-order Initial Value Problem (IVP):
///
/// Given a general solution $y(x, C_1, C_2)$, finds $C_1, C_2$ such that
/// $y(x_0) = y_0$ and $y'(x_0) = dy_0$.
///
/// Maxima `src/ode2.lisp` (`$ic2`).
pub fn ic2(
    sol: &Expr,
    x_var: &str,
    c1_var: &str,
    c2_var: &str,
    x0: &Expr,
    y0: &Expr,
    dy0: &Expr,
) -> Result<Expr, String> {
    let y_at_x0 = subst(sol, x_var, x0);
    let dy_expr = differentiate(sol, x_var);
    let dy_at_x0 = subst(&dy_expr, x_var, x0);

    // Extract 2x2 linear system for C1, C2:
    // Equation 1: A11*C1 + A12*C2 + B1 = y0
    let b1 = subst(
        &subst(&y_at_x0, c1_var, &Expr::int(0)),
        c2_var,
        &Expr::int(0),
    );
    let y_c1_1 = subst(
        &subst(&y_at_x0, c1_var, &Expr::int(1)),
        c2_var,
        &Expr::int(0),
    );
    let y_c2_1 = subst(
        &subst(&y_at_x0, c1_var, &Expr::int(0)),
        c2_var,
        &Expr::int(1),
    );
    let a11 = Expr::sub(y_c1_1, b1.clone());
    let a12 = Expr::sub(y_c2_1, b1.clone());

    // Equation 2: A21*C1 + A22*C2 + B2 = dy0
    let b2 = subst(
        &subst(&dy_at_x0, c1_var, &Expr::int(0)),
        c2_var,
        &Expr::int(0),
    );
    let dy_c1_1 = subst(
        &subst(&dy_at_x0, c1_var, &Expr::int(1)),
        c2_var,
        &Expr::int(0),
    );
    let dy_c2_1 = subst(
        &subst(&dy_at_x0, c1_var, &Expr::int(0)),
        c2_var,
        &Expr::int(1),
    );
    let a21 = Expr::sub(dy_c1_1, b2.clone());
    let a22 = Expr::sub(dy_c2_1, b2.clone());

    // Solve 2x2 system via Cramer's rule:
    let det = simplify_bvp_expr(&Expr::sub(
        Expr::mul(a11.clone(), a22.clone()),
        Expr::mul(a12.clone(), a21.clone()),
    ));
    if is_bvp_zero(&det) {
        return Err("Singular determinant in initial value system".to_string());
    }

    let rhs1 = Expr::sub(y0.clone(), b1);
    let rhs2 = Expr::sub(dy0.clone(), b2);

    let c1_num = Expr::sub(Expr::mul(rhs1.clone(), a22), Expr::mul(rhs2.clone(), a12));
    let c2_num = Expr::sub(Expr::mul(a11, rhs2), Expr::mul(a21, rhs1));

    let c1_val = simplify_bvp_expr(&Expr::div(c1_num, det.clone()));
    let c2_val = simplify_bvp_expr(&Expr::div(c2_num, det));

    let sol_c1 = subst(sol, c1_var, &c1_val);
    Ok(simplify_bvp_expr(&subst(&sol_c1, c2_var, &c2_val)))
}

/// Solve 2nd-order IVP directly from an [`OdeSolution`].
pub fn ic2_ode(
    ode: &OdeSolution,
    x_var: &str,
    x0: &Expr,
    y0: &Expr,
    dy0: &Expr,
) -> Result<Expr, String> {
    let c1_var = ode
        .constants
        .first()
        .cloned()
        .unwrap_or_else(|| "_C1".to_string());
    let c2_var = ode
        .constants
        .get(1)
        .cloned()
        .unwrap_or_else(|| "_C2".to_string());
    ic2(&ode.solution, x_var, &c1_var, &c2_var, x0, y0, dy0)
}

/// Solve 2nd-order Boundary Value Problem (BVP):
///
/// Given a general solution $y(x, C_1, C_2)$, finds $C_1, C_2$ such that
/// $y(x_0) = y_0$ and $y(x_1) = y_1$.
///
/// Maxima `src/ode2.lisp` (`$bc2`).
pub fn bc2(
    sol: &Expr,
    x_var: &str,
    c1_var: &str,
    c2_var: &str,
    bc0: (&Expr, &Expr),
    bc1: (&Expr, &Expr),
) -> Result<Expr, String> {
    let (x0, y0) = bc0;
    let (x1, y1) = bc1;
    let y_at_x0 = subst(sol, x_var, x0);
    let y_at_x1 = subst(sol, x_var, x1);

    // Equation 1: A11*C1 + A12*C2 + B1 = y0
    let b1 = subst(
        &subst(&y_at_x0, c1_var, &Expr::int(0)),
        c2_var,
        &Expr::int(0),
    );
    let y_c1_1 = subst(
        &subst(&y_at_x0, c1_var, &Expr::int(1)),
        c2_var,
        &Expr::int(0),
    );
    let y_c2_1 = subst(
        &subst(&y_at_x0, c1_var, &Expr::int(0)),
        c2_var,
        &Expr::int(1),
    );
    let a11 = Expr::sub(y_c1_1, b1.clone());
    let a12 = Expr::sub(y_c2_1, b1.clone());

    // Equation 2: A21*C1 + A22*C2 + B2 = y1
    let b2 = subst(
        &subst(&y_at_x1, c1_var, &Expr::int(0)),
        c2_var,
        &Expr::int(0),
    );
    let y1_c1_1 = subst(
        &subst(&y_at_x1, c1_var, &Expr::int(1)),
        c2_var,
        &Expr::int(0),
    );
    let y1_c2_1 = subst(
        &subst(&y_at_x1, c1_var, &Expr::int(0)),
        c2_var,
        &Expr::int(1),
    );
    let a21 = Expr::sub(y1_c1_1, b2.clone());
    let a22 = Expr::sub(y1_c2_1, b2.clone());

    let det = simplify_bvp_expr(&Expr::sub(
        Expr::mul(a11.clone(), a22.clone()),
        Expr::mul(a12.clone(), a21.clone()),
    ));
    if is_bvp_zero(&det) {
        return Err("Singular determinant in boundary value system".to_string());
    }

    let rhs1 = Expr::sub(y0.clone(), b1);
    let rhs2 = Expr::sub(y1.clone(), b2);

    let c1_num = Expr::sub(Expr::mul(rhs1.clone(), a22), Expr::mul(rhs2.clone(), a12));
    let c2_num = Expr::sub(Expr::mul(a11, rhs2), Expr::mul(a21, rhs1));

    let c1_val = simplify_bvp_expr(&Expr::div(c1_num, det.clone()));
    let c2_val = simplify_bvp_expr(&Expr::div(c2_num, det));

    let sol_c1 = subst(sol, c1_var, &c1_val);
    Ok(simplify_bvp_expr(&subst(&sol_c1, c2_var, &c2_val)))
}

/// Solve 2nd-order BVP directly from an [`OdeSolution`].
pub fn bc2_ode(
    ode: &OdeSolution,
    x_var: &str,
    bc0: (&Expr, &Expr),
    bc1: (&Expr, &Expr),
) -> Result<Expr, String> {
    let c1_var = ode
        .constants
        .first()
        .cloned()
        .unwrap_or_else(|| "_C1".to_string());
    let c2_var = ode
        .constants
        .get(1)
        .cloned()
        .unwrap_or_else(|| "_C2".to_string());
    bc2(&ode.solution, x_var, &c1_var, &c2_var, bc0, bc1)
}
