//! Bareiss fraction-free LU determinant and symbolic linear algebra (Phase 40).
//!
//! Implements fraction-free matrix elimination citing Erwin Bareiss (1968) and
//! Maxima's `newdet.lisp` (ADR-0010). Guarantees $O(n^3)$ polynomial preservation
//! without fractional expression explosion during intermediate elimination.

use std::fmt;
use std::ops::{Index, IndexMut};

use crate::expr::Expr;
use crate::rational::Rat;

/// Strongly typed domain errors for symbolic matrix operations.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum MatrixError {
    #[error("non-square matrix: {rows}x{cols}")]
    NonSquare { rows: usize, cols: usize },

    #[error("dimension mismatch: {r1}x{c1} vs {r2}x{c2}")]
    DimensionMismatch {
        r1: usize,
        c1: usize,
        r2: usize,
        c2: usize,
    },

    #[error("ragged matrix: row {row} has length {len}, expected {expected}")]
    RaggedMatrix {
        row: usize,
        len: usize,
        expected: usize,
    },

    #[error("singular matrix with determinant zero")]
    SingularMatrix,

    #[error("index out of bounds: ({row}, {col}) for matrix of size {rows}x{cols}")]
    OutOfBounds {
        row: usize,
        col: usize,
        rows: usize,
        cols: usize,
    },

    #[error("not a matrix expression: {0}")]
    NotAMatrix(String),
}

/// A 2D matrix of symbolic expressions [`Expr`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matrix {
    pub rows: usize,
    pub cols: usize,
    pub data: Vec<Vec<Expr>>,
}

impl Index<(usize, usize)> for Matrix {
    type Output = Expr;

    fn index(&self, (r, c): (usize, usize)) -> &Self::Output {
        &self.data[r][c]
    }
}

impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut Self::Output {
        &mut self.data[r][c]
    }
}

impl Matrix {
    /// Construct a new `Matrix` validating row uniformity.
    pub fn new(data: Vec<Vec<Expr>>) -> Result<Self, MatrixError> {
        let rows = data.len();
        if rows == 0 {
            return Ok(Self {
                rows: 0,
                cols: 0,
                data,
            });
        }
        let cols = data[0].len();
        for (i, row) in data.iter().enumerate() {
            if row.len() != cols {
                return Err(MatrixError::RaggedMatrix {
                    row: i,
                    len: row.len(),
                    expected: cols,
                });
            }
        }
        Ok(Self { rows, cols, data })
    }

    /// Construct from an `Expr::Matrix`.
    pub fn from_expr(expr: &Expr) -> Result<Self, MatrixError> {
        match expr {
            Expr::Matrix(rows) => Self::new(rows.clone()),
            other => Err(MatrixError::NotAMatrix(other.to_string())),
        }
    }

    /// Convert into `Expr::Matrix`.
    pub fn to_expr(&self) -> Expr {
        Expr::Matrix(self.data.clone())
    }

    /// Create an `r x c` zero matrix.
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            data: vec![vec![Expr::int(0); cols]; rows],
        }
    }

    /// Create an `n x n` identity matrix.
    pub fn identity(n: usize) -> Self {
        let mut data = vec![vec![Expr::int(0); n]; n];
        for (i, row) in data.iter_mut().enumerate() {
            row[i] = Expr::int(1);
        }
        Self {
            rows: n,
            cols: n,
            data,
        }
    }

    /// Check if the matrix is square.
    pub fn is_square(&self) -> bool {
        self.rows == self.cols
    }

    /// Matrix addition `A + B`.
    pub fn add(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(MatrixError::DimensionMismatch {
                r1: self.rows,
                c1: self.cols,
                r2: other.rows,
                c2: other.cols,
            });
        }
        let mut res = vec![vec![Expr::int(0); self.cols]; self.rows];
        for (r_res, (r_self, r_other)) in
            res.iter_mut().zip(self.data.iter().zip(other.data.iter()))
        {
            for (cell, (c1, c2)) in r_res.iter_mut().zip(r_self.iter().zip(r_other.iter())) {
                *cell = crate::poly::expand(&Expr::add(c1.clone(), c2.clone()));
            }
        }
        Self::new(res)
    }

    /// Matrix subtraction `A - B`.
    pub fn sub(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.rows != other.rows || self.cols != other.cols {
            return Err(MatrixError::DimensionMismatch {
                r1: self.rows,
                c1: self.cols,
                r2: other.rows,
                c2: other.cols,
            });
        }
        let mut res = vec![vec![Expr::int(0); self.cols]; self.rows];
        for (r_res, (r_self, r_other)) in
            res.iter_mut().zip(self.data.iter().zip(other.data.iter()))
        {
            for (cell, (c1, c2)) in r_res.iter_mut().zip(r_self.iter().zip(r_other.iter())) {
                *cell = crate::poly::expand(&Expr::sub(c1.clone(), c2.clone()));
            }
        }
        Self::new(res)
    }

    /// Matrix multiplication `A * B`.
    pub fn mul(&self, other: &Self) -> Result<Self, MatrixError> {
        if self.cols != other.rows {
            return Err(MatrixError::DimensionMismatch {
                r1: self.rows,
                c1: self.cols,
                r2: other.rows,
                c2: other.cols,
            });
        }
        let mut res = vec![vec![Expr::int(0); other.cols]; self.rows];
        for (i, r_res) in res.iter_mut().enumerate() {
            for (j, cell) in r_res.iter_mut().enumerate() {
                let mut sum = Expr::int(0);
                for k in 0..self.cols {
                    let term = Expr::mul(self.data[i][k].clone(), other.data[k][j].clone());
                    sum = Expr::add(sum, term);
                }
                *cell = crate::poly::expand(&sum);
            }
        }
        Self::new(res)
    }

    /// Scalar multiplication `c * A`.
    pub fn scale(&self, scalar: &Expr) -> Self {
        let mut res = vec![vec![Expr::int(0); self.cols]; self.rows];
        for (r_res, r_self) in res.iter_mut().zip(self.data.iter()) {
            for (cell, elem) in r_res.iter_mut().zip(r_self.iter()) {
                *cell = crate::poly::expand(&Expr::mul(scalar.clone(), elem.clone()));
            }
        }
        Self {
            rows: self.rows,
            cols: self.cols,
            data: res,
        }
    }

    /// Transpose of the matrix $A^T$.
    pub fn transpose(&self) -> Self {
        let mut res = vec![vec![Expr::int(0); self.rows]; self.cols];
        for (i, row) in self.data.iter().enumerate() {
            for (j, elem) in row.iter().enumerate() {
                res[j][i] = elem.clone();
            }
        }
        Self {
            rows: self.cols,
            cols: self.rows,
            data: res,
        }
    }

    /// Submatrix obtained by removing row `skip_row` and column `skip_col`.
    pub fn submatrix_excluding(&self, skip_row: usize, skip_col: usize) -> Self {
        let mut sub = Vec::with_capacity(self.rows.saturating_sub(1));
        for (i, r) in self.data.iter().enumerate() {
            if i == skip_row {
                continue;
            }
            let mut row = Vec::with_capacity(self.cols.saturating_sub(1));
            for (j, elem) in r.iter().enumerate() {
                if j == skip_col {
                    continue;
                }
                row.push(elem.clone());
            }
            sub.push(row);
        }
        Self {
            rows: self.rows.saturating_sub(1),
            cols: self.cols.saturating_sub(1),
            data: sub,
        }
    }
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "matrix(")?;
        for (i, row) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "[")?;
            for (j, elem) in row.iter().enumerate() {
                if j > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{elem}")?;
            }
            write!(f, "]")?;
        }
        write!(f, ")")
    }
}

// ── Simplification & Kernel Mapping Helpers ────────────────────────

fn map_kernels(e: &Expr, kernels: &mut Vec<Expr>) -> Expr {
    match e {
        Expr::Add(a, b) => Expr::add(map_kernels(a, kernels), map_kernels(b, kernels)),
        Expr::Sub(a, b) => Expr::sub(map_kernels(a, kernels), map_kernels(b, kernels)),
        Expr::Mul(a, b) => Expr::mul(map_kernels(a, kernels), map_kernels(b, kernels)),
        Expr::Div(a, b) => Expr::div(map_kernels(a, kernels), map_kernels(b, kernels)),
        Expr::Neg(a) => Expr::neg(map_kernels(a, kernels)),
        Expr::Pow(b, exp) => {
            if let Expr::Const(r) = exp.as_ref()
                && crate::rational::Rat::is_integer(r)
                && *r >= num_rational::BigRational::from_integer(0.into())
            {
                return Expr::pow(map_kernels(b, kernels), (**exp).clone());
            }
            kernel_var(e, kernels)
        }
        Expr::Var(_) | Expr::Const(_) => e.clone(),
        _ => kernel_var(e, kernels),
    }
}

fn kernel_var(e: &Expr, kernels: &mut Vec<Expr>) -> Expr {
    if let Some(idx) = kernels.iter().position(|k| k == e) {
        return Expr::var(&format!("__mk_{idx}"));
    }
    let idx = kernels.len();
    kernels.push(e.clone());
    Expr::var(&format!("__mk_{idx}"))
}

fn unmap_kernels(e: &Expr, kernels: &[Expr]) -> Expr {
    match e {
        Expr::Var(n) if n.starts_with("__mk_") => {
            if let Ok(idx) = n["__mk_".len()..].parse::<usize>()
                && let Some(orig) = kernels.get(idx)
            {
                return orig.clone();
            }
            e.clone()
        }
        Expr::Add(a, b) => Expr::add(unmap_kernels(a, kernels), unmap_kernels(b, kernels)),
        Expr::Sub(a, b) => Expr::sub(unmap_kernels(a, kernels), unmap_kernels(b, kernels)),
        Expr::Mul(a, b) => Expr::mul(unmap_kernels(a, kernels), unmap_kernels(b, kernels)),
        Expr::Div(a, b) => Expr::div(unmap_kernels(a, kernels), unmap_kernels(b, kernels)),
        Expr::Pow(a, b) => Expr::pow(unmap_kernels(a, kernels), unmap_kernels(b, kernels)),
        Expr::Neg(a) => Expr::neg(unmap_kernels(a, kernels)),
        _ => e.clone(),
    }
}

fn is_expr_zero(e: &Expr) -> bool {
    if e.is_zero() {
        return true;
    }
    let exp = crate::poly::expand(e);
    if exp.is_zero() {
        return true;
    }
    if let Ok(sim) = crate::poly::ratsimp(e)
        && sim.is_zero()
    {
        return true;
    }
    let mut kernels = Vec::new();
    let mapped = map_kernels(e, &mut kernels);
    if let Ok(p) = crate::poly::Poly::from_expr(&mapped)
        && p.is_zero()
    {
        return true;
    }
    false
}

fn simplify_div(num: &Expr, den: &Expr) -> Expr {
    if den.is_one() {
        return num.clone();
    }
    if num.is_zero() {
        return Expr::int(0);
    }
    if let (Expr::Const(r1), Expr::Const(r2)) = (num, den)
        && let Some(res) = Rat::div(r1, r2)
    {
        return Expr::Const(res);
    }
    let div_expr = Expr::div(num.clone(), den.clone());
    if let Ok(sim) = crate::poly::ratsimp(&div_expr) {
        sim
    } else {
        crate::poly::expand(&div_expr)
    }
}

/// Try converting all matrix entries into `Poly` over a shared `VarContext` (with kernel mapping).
fn matrix_to_polys(
    m: &Matrix,
) -> Option<(
    Vec<Vec<crate::poly::Poly>>,
    crate::poly::VarContext,
    Vec<Expr>,
)> {
    use crate::poly::{Poly, VarContext};
    let mut kernels = Vec::new();
    let mut mapped_rows = Vec::with_capacity(m.rows);
    let mut all_vars = Vec::new();
    for row in &m.data {
        let mut mr = Vec::with_capacity(m.cols);
        for cell in row {
            let me = map_kernels(cell, &mut kernels);
            for v in me.variables() {
                if !all_vars.contains(&v) {
                    all_vars.push(v);
                }
            }
            mr.push(me);
        }
        mapped_rows.push(mr);
    }
    all_vars.sort();
    let mut ctx = VarContext::new();
    for v in &all_vars {
        ctx.add(v);
    }
    let mut poly_rows = Vec::with_capacity(m.rows);
    for mr in &mapped_rows {
        let mut pr = Vec::with_capacity(m.cols);
        for cell in mr {
            pr.push(Poly::from_expr_with_ctx(cell, &ctx).ok()?);
        }
        poly_rows.push(pr);
    }
    Some((poly_rows, ctx, kernels))
}

fn bareiss_poly_det(
    mut a: Vec<Vec<crate::poly::Poly>>,
    ctx: &crate::poly::VarContext,
    kernels: &[Expr],
) -> Option<Expr> {
    use crate::poly::Poly;
    use crate::poly::gcd::poly_div_exact;
    use num_rational::BigRational;
    use num_traits::One;

    let n = a.len();
    let mut prev_pivot = Poly::constant(BigRational::one(), ctx.clone());
    let mut sign = 1i32;

    for k in 0..n - 1 {
        if a[k][k].is_zero() {
            let swap_row = (k + 1..n).find(|&p| !a[p][k].is_zero());
            match swap_row {
                Some(p) => {
                    a.swap(k, p);
                    sign = -sign;
                }
                None => return Some(Expr::int(0)),
            }
        }
        let p_kk = a[k][k].clone();
        for i in (k + 1)..n {
            let p_ik = a[i][k].clone();
            for j in (k + 1)..n {
                let num = p_kk.mul(&a[i][j]).sub(&p_ik.mul(&a[k][j]));
                a[i][j] = poly_div_exact(&num, &prev_pivot)?;
            }
        }
        prev_pivot = p_kk;
    }
    let det_poly = if sign == 1 {
        a[n - 1][n - 1].clone()
    } else {
        a[n - 1][n - 1].neg()
    };
    Some(unmap_kernels(&det_poly.to_expr(), kernels))
}

// ── Bareiss Algorithm ───────────────────────────────────────────────

/// Compute the determinant of a square matrix using the Bareiss $O(n^3)$ fraction-free algorithm.
///
/// Mirrors Maxima 5.50's `newdet.lisp` (ADR-0010). Performs exact division by the previous pivot
/// at each elimination step, avoiding exponential expression blowup.
pub fn determinant(m: &Matrix) -> Result<Expr, MatrixError> {
    if !m.is_square() {
        return Err(MatrixError::NonSquare {
            rows: m.rows,
            cols: m.cols,
        });
    }
    let n = m.rows;
    if n == 0 {
        return Ok(Expr::int(1));
    }
    if n == 1 {
        return Ok(m.data[0][0].clone());
    }
    if n == 2 {
        let p1 = Expr::mul(m.data[0][0].clone(), m.data[1][1].clone());
        let p2 = Expr::mul(m.data[0][1].clone(), m.data[1][0].clone());
        return Ok(crate::poly::expand(&Expr::sub(p1, p2)));
    }

    if let Some((polys, ctx, kernels)) = matrix_to_polys(m)
        && let Some(det) = bareiss_poly_det(polys, &ctx, &kernels)
    {
        return Ok(det);
    }

    let mut a = m.data.clone();
    let mut prev_pivot = Expr::int(1);
    let mut sign = 1i32;

    for k in 0..n - 1 {
        if is_expr_zero(&a[k][k]) {
            let mut swap_row = None;
            for (p, row) in a.iter().enumerate().skip(k + 1) {
                if !is_expr_zero(&row[k]) {
                    swap_row = Some(p);
                    break;
                }
            }
            match swap_row {
                Some(p) => {
                    a.swap(k, p);
                    sign = -sign;
                }
                None => return Ok(Expr::int(0)),
            }
        }

        let p_kk = a[k][k].clone();
        for i in (k + 1)..n {
            let p_ik = a[i][k].clone();
            for j in (k + 1)..n {
                let p_kj = a[k][j].clone();
                let a_ij = a[i][j].clone();

                let cross1 = Expr::mul(p_kk.clone(), a_ij);
                let cross2 = Expr::mul(p_ik.clone(), p_kj);
                let num = crate::poly::expand(&Expr::sub(cross1, cross2));
                a[i][j] = simplify_div(&num, &prev_pivot);
            }
        }
        prev_pivot = p_kk;
    }

    let last = a[n - 1][n - 1].clone();
    let det = if sign == 1 {
        crate::poly::expand(&last)
    } else {
        crate::poly::expand(&Expr::neg(last))
    };
    Ok(det)
}

/// Compute the trace of a square matrix: $\text{tr}(M) = \sum_{i=1}^n M_{i,i}$.
pub fn trace(m: &Matrix) -> Result<Expr, MatrixError> {
    if !m.is_square() {
        return Err(MatrixError::NonSquare {
            rows: m.rows,
            cols: m.cols,
        });
    }
    let mut sum = Expr::int(0);
    for (i, row) in m.data.iter().enumerate() {
        sum = Expr::add(sum, row[i].clone());
    }
    Ok(crate::poly::expand(&sum))
}

/// Compute the characteristic polynomial of a square matrix: $P(\lambda) = \det(M - \lambda I)$.
///
/// Mirrors Maxima 5.50's `charpoly(M, var)` in `newdet.lisp`.
pub fn charpoly(m: &Matrix, var: &str) -> Result<Expr, MatrixError> {
    if !m.is_square() {
        return Err(MatrixError::NonSquare {
            rows: m.rows,
            cols: m.cols,
        });
    }
    let lambda = Expr::var(var);
    let mut data = m.data.clone();
    for (i, row) in data.iter_mut().enumerate() {
        row[i] = Expr::sub(row[i].clone(), lambda.clone());
    }
    let char_mat = Matrix::new(data)?;
    determinant(&char_mat)
}

/// Compute the classical adjoint (adjugate) matrix using $O(n^3)$ Bareiss-Jordan
/// fraction-free elimination for non-singular matrices, falling back to cofactors if singular.
pub fn adjoint(m: &Matrix) -> Result<Matrix, MatrixError> {
    if !m.is_square() {
        return Err(MatrixError::NonSquare {
            rows: m.rows,
            cols: m.cols,
        });
    }
    let n = m.rows;
    if n == 0 {
        return Ok(Matrix::zeros(0, 0));
    }
    if n == 1 {
        return Matrix::new(vec![vec![Expr::int(1)]]);
    }

    if let Some((polys, ctx, kernels)) = matrix_to_polys(m)
        && let Some(adj) = bareiss_jordan_adjoint_poly(polys, &ctx, &kernels)
    {
        return Matrix::new(adj);
    }

    let mut adj_data = vec![vec![Expr::int(0); n]; n];
    for (i, row) in adj_data.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            let sub = m.submatrix_excluding(j, i);
            let det = determinant(&sub)?;
            let sign = if (i + j) % 2 == 0 { 1 } else { -1 };
            let cofactor = if sign == 1 {
                det
            } else {
                crate::poly::expand(&Expr::neg(det))
            };
            *cell = cofactor;
        }
    }
    Matrix::new(adj_data)
}

fn bareiss_jordan_adjoint_poly(
    mut a: Vec<Vec<crate::poly::Poly>>,
    ctx: &crate::poly::VarContext,
    kernels: &[Expr],
) -> Option<Vec<Vec<Expr>>> {
    use crate::poly::Poly;
    use crate::poly::gcd::poly_div_exact;
    use num_rational::BigRational;
    use num_traits::One;

    let n = a.len();
    let one = Poly::constant(BigRational::one(), ctx.clone());
    let zero = Poly::zero(ctx.clone());
    let mut rhs = vec![vec![zero; n]; n];
    for (i, row) in rhs.iter_mut().enumerate() {
        row[i] = one.clone();
    }
    let mut prev_pivot = one;
    let mut sign = 1i32;

    for k in 0..n {
        if a[k][k].is_zero() {
            let p = (k + 1..n).find(|&r| !a[r][k].is_zero())?;
            a.swap(k, p);
            rhs.swap(k, p);
            sign = -sign;
        }
        let p_kk = a[k][k].clone();
        for i in 0..n {
            if i == k {
                continue;
            }
            let p_ik = a[i][k].clone();
            for j in (k + 1)..n {
                let num = p_kk.mul(&a[i][j]).sub(&p_ik.mul(&a[k][j]));
                a[i][j] = poly_div_exact(&num, &prev_pivot)?;
            }
            for j in 0..n {
                let num = p_kk.mul(&rhs[i][j]).sub(&p_ik.mul(&rhs[k][j]));
                rhs[i][j] = poly_div_exact(&num, &prev_pivot)?;
            }
            a[i][k] = Poly::zero(ctx.clone());
        }
        prev_pivot = p_kk;
    }

    let mut out = vec![vec![Expr::int(0); n]; n];
    for (i, row) in rhs.into_iter().enumerate() {
        for (j, poly) in row.into_iter().enumerate() {
            let signed = if sign == 1 { poly } else { poly.neg() };
            out[i][j] = unmap_kernels(&signed.to_expr(), kernels);
        }
    }
    Some(out)
}

/// Compute the inverse matrix $M^{-1} = \frac{1}{\det(M)} \text{adj}(M)$.
///
/// Returns `Err(MatrixError::SingularMatrix)` if $\det(M) = 0$.
pub fn invert(m: &Matrix) -> Result<Matrix, MatrixError> {
    let det = determinant(m)?;
    if is_expr_zero(&det) {
        return Err(MatrixError::SingularMatrix);
    }
    let adj = adjoint(m)?;
    let mut inv_data = vec![vec![Expr::int(0); m.cols]; m.rows];
    for (r_inv, r_adj) in inv_data.iter_mut().zip(adj.data.iter()) {
        for (cell, adj_elem) in r_inv.iter_mut().zip(r_adj.iter()) {
            *cell = simplify_div(adj_elem, &det);
        }
    }
    Matrix::new(inv_data)
}

/// Compute the eigenvalues of a square matrix by finding exact roots of $\text{charpoly}(M, \lambda) = 0$.
pub fn eigenvalues(m: &Matrix) -> Result<Vec<Expr>, MatrixError> {
    let lambda_var = "%lambda";
    let cp = charpoly(m, lambda_var)?;
    let sols = crate::solve::solve(&cp, lambda_var);
    Ok(sols.into_vec())
}
