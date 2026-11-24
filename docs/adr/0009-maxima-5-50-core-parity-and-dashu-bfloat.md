# Maxima 5.50 Core Mathematical Parity and Pure-Rust Bfloat Strategy

**Status**: Accepted

Context: Maxima 5.50 is a multi-decade Common Lisp Computer Algebra System combining an interactive dynamic LISP runtime (`meval`, dynamic property lists, dynamic scoping) with extensive mathematical transformation libraries across core `src/` and over 100 domain-specific `share/` packages. Achieving 1:1 parity in `maxima-rs` requires deciding the architectural boundary of parity, the numerical floating-point model, and the implementation sequence. We decided to:
1. Scope 1:1 parity strictly to **Core Mathematical Parity**: porting the algorithms, transformations, special functions, and solvers of Maxima core `src/` into deterministic Rust modules and `egg` rewrite rules under the hybrid architecture (ADR-0006), rather than emulating the Common Lisp interpreter runtime or dynamic scoping.
2. Bound the target codebase to Maxima core `src/`, treating contributed `share/` packages as out of scope for the core parity definition.
3. Implement arbitrary-precision floating-point numbers (`bfloat`) matching Maxima's `bfloat` and `fpprec` using the pure-Rust `dashu-float` crate. This avoids C-linking dependencies (such as GNU MPFR / `rug`), ensuring that `maxima-rs`, `sym-core`, and `sym-macros` remain 100% memory-safe, cross-platform, compile-time usable in procedural macros, and deployable to WebAssembly targets.
4. Sequence the remaining implementation phases in dependency order: Linear Algebra (`matrix.lisp`) → Complex Analysis (`rpart.lisp`) → Special Functions (`specfn.lisp`) → Advanced Solvers & Transforms (`laplac.lisp`, `ode2.lisp`) → Context Database & Deduction Engine (`compar.lisp`, `askp.lisp`).
