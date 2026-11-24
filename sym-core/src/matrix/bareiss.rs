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

// ── Simplification Helper ───────────────────────────────────────────

fn is_expr_zero(e: &Expr) -> bool {
    if e.is_zero() {
        return true;
    }
    let exp = crate::poly::expand(e);
    if exp.is_zero() {
        return true;
    }
    if let Ok(sim) = crate::poly::ratsimp(e) {
        return sim.is_zero();
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

    let mut a = m.data.clone();
    let mut prev_pivot = Expr::int(1);
    let mut sign = 1i32;

    for k in 0..n - 1 {
        // Pivot selection: check if a[k][k] is zero
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

/// Compute the classical adjoint (adjugate) matrix: $\text{adj}(M)_{i,j} = (-1)^{i+j} \det(M_{\setminus j, \setminus i})$.
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
