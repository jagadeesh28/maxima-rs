# Special Functions, Operational Laplace Transforms, and Scoped Assumptions Architecture

**Status**: Accepted

Context: Expanding `maxima-rs` to 1:1 parity with Maxima 5.50 requires supporting dozens of special functions (`specfn.lisp`, `bessel.lisp`), symbolic forward/inverse Laplace transforms (`laplac.lisp`), and a mathematical deduction/assumptions engine (`compar.lisp`, `askp.lisp`). We decided to:
1. Represent non-elementary special functions via a single consolidated enum variant `Expr::SpecialFn(SpecialFnKind, Vec<Expr>)`. `SpecialFnKind` enumerates supported functions (Gamma, Erf, Bessel, Airy, Orthogonal Polynomials, etc.). Differentiation $\frac{d}{dx} f(u)$, asymptotic expansions, and numerical evaluation are dispatched via centralized table registries citing `specfn.lisp`, keeping the core AST compact and modular.
2. Implement forward `laplace(f, t, s)` and inverse `ilt(F, s, t)` using operational calculus rules (linearity, frequency shifts, derivative identities) coupled with `partfrac` pole decomposition and standard transform table matching citing `laplac.lisp`.
3. Model mathematical assumptions (`assume`, `forget`, `is`, `asksign`, `csign`) as an immutable, cloneable `Assumptions` context struct holding an inequality DAG and type property database. CAS algorithms take `Option<&Assumptions>`, eliminating global mutable locks while enabling safe concurrent execution across multiple web sessions and threads.
