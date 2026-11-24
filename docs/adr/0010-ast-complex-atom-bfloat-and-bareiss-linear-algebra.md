# AST Complex Atom, Bfloat Node, and Bareiss Linear Algebra Architecture

**Status**: Accepted

Context: Implementing Maxima 5.50 parity for symbolic linear algebra (`matrix.lisp`, `newdet.lisp`), complex analysis (`rpart.lisp`), and arbitrary-precision floating-point numbers requires choosing exact representation models in `Expr` and foundational numerical/symbolic algorithms. We decided to:
1. Add `Expr::I` as a first-class, zero-allocation atom in the `Expr` AST enum representing the imaginary unit $i = \sqrt{-1}$. This provides clean pattern-matching, avoids string comparison overhead of `%i` variables, and integrates directly into `egg` rewrite rules ($I^2 \to -1$, Euler's formula) and procedural `sym-core/src/complex/` routines (`rectform`, `polarform`, `carg`, `conjugate`, `residue`).
2. Integrate `Expr::Bfloat(dashu::float::DBig)` into the AST enum for arbitrary-precision floating-point arithmetic with ambient precision (`fpprec`) managed via pipeline and API configuration.
3. Implement symbolic matrix determinant (`determinant`), adjoint inverse (`invert`), and characteristic polynomial (`charpoly`) using the **Bareiss fraction-free LU algorithm** (`newdet.lisp`). This guarantees $O(n^3)$ polynomial preservation without fractional expression explosion during intermediate elimination steps.
