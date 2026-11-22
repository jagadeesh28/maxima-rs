# Symbolic Rust Optimizer (SymOpt)

A compile-time symbolic mathematics and code optimization engine in Rust, combining algebraic calculus, e-graph equality saturation, and procedural macro code generation.

## Language

**Symbolic Expression**:
An immutable abstract syntax tree representing real-valued mathematical operations, variables, and numerical constants.
_Avoid_: S-expression, Maxima form, math string

**E-Graph**:
A data structure encoding an exponential number of equivalent mathematical expressions simultaneously within equivalence classes.
_Avoid_: Expression tree, DAG

**Equality Saturation**:
A transformation process applying bidirectional algebraic rewrite rules non-destructively to an e-graph until reaching a fixed point or iteration threshold.
_Avoid_: Heuristic simplification, greedy rewriting, peephole pass

**Analytical Differentiation**:
Exact symbolic computation of partial derivatives, gradients, Jacobians, or Hessians from symbolic expressions.
_Avoid_: Numerical differentiation, automatic differentiation, autodiff

**Common Subexpression Elimination (CSE)**:
An extraction phase that identifies duplicate subcomputations across expressions and binds them to local variables to minimize redundant FLOPs.
_Avoid_: Factoring, deduplication

**Proc-Macro Codegen**:
The compile-time procedural macro phase transforming optimized expressions into zero-overhead native Rust function definitions.
_Avoid_: Transpilation, runtime evaluation, script generation

**Cost Model**:
A weighted assignment of empirical hardware cycle costs to mathematical operators used by the e-graph extractor to select the cheapest equivalent expression.
_Avoid_: Heuristic, AST size metric

**Fast-Math Semantics**:
The default algebraic mode treating floating-point values as real numbers, enabling full associative, commutative, and distributive rewrites regardless of IEEE-754 rounding edge cases. Can be disabled per-function for strict mode.
_Avoid_: Unsafe math, approximate mode

**sym_fn!**:
The item-level procedural macro declaring a symbolic function with named inputs, expression body, and requested derivative outputs (gradient, Jacobian, Hessian).
_Avoid_: derive macro, attribute macro

**sym_eval!**:
An inline expression macro returning an optimized closure or value directly within existing Rust code blocks.
_Avoid_: eval, compute macro

**Rewrite Rule**:
A bidirectional or directional algebraic identity encoded via `egg`'s `rewrite!` macro, hand-curated from Maxima's simplification logic and organized by operator category.
_Avoid_: Transformation, substitution, macro rule

**CSE Post-Pass**:
A separate pass after e-graph extraction that identifies shared subterms across a function's multiple outputs (value, gradient components) and emits sequential `let` bindings to eliminate redundant computation.
_Avoid_: Joint extraction, inline expansion

**sym-core**:
The foundational library crate containing the expression AST, e-graph rewrite rules, analytical differentiation, cost model, and CSE extraction. Usable standalone without macros.
_Avoid_: engine, backend

**sym-macros**:
The proc-macro crate providing `sym_fn!` and `sym_eval!`. Depends on sym-core.
_Avoid_: codegen crate, derive crate

**maxima-rs**:
The user-facing facade crate that re-exports sym-core utilities and sym-macros procedural macros as a single dependency. Named to acknowledge the project's Maxima heritage.
_Avoid_: max, sym-opt

**Saturation Limits**:
Fixed bounds on e-graph equality saturation (node limit, iteration limit, time limit) that prevent pathological compile-time blowups, with per-invocation overrides available via macro attributes.
_Avoid_: Timeout, budget

**Rational Constant**:
An arbitrary-precision rational number (`num::BigRational`) used to represent numeric constants inside the e-graph during simplification. Converted to a floating-point literal only during final codegen.
_Avoid_: Float constant, approximate constant

**Output Float Type**:
The concrete floating-point type emitted in generated code. Defaults to `f64`; configurable to `f32` via a per-invocation macro attribute for GPU or embedded targets.
_Avoid_: Generic float, trait-bounded type

**Rule Provenance**:
Structured metadata (`RuleProvenance` struct) tracing an `egg` rewrite rule back to its exact Maxima Common LISP source file and function, enabling 1:1 coverage comparison.
_Avoid_: Rule tag, rule annotation, rule metadata

**Pipeline**:
A fluent builder API that composes parsing, differentiation, optimization, and code generation into a chainable multi-step workflow on symbolic expressions.
_Avoid_: Builder, chain, workflow object

**Input Format**:
A supported notation for parsing symbolic expressions into `Expr` ASTs: currently Maxima string notation and LaTeX formula notation, extensible to additional formats.
_Avoid_: Syntax mode, encoding, input language

**Maxima Source Citation**:
A doc comment and `RuleProvenance` entry tracing a rewrite rule to its exact Maxima Common LISP source file and function name, maintaining systematic 1:1 correspondence.
_Avoid_: Reference, attribution, credit

**Hyperbolic Function Node**:
An `Expr` variant (`Sinh`, `Cosh`, `Tanh`) representing hyperbolic sine/cosine/tangent, simplified by rules citing `sinh.lisp` (`simp-%sinh`, `simp-%cosh`, `simp-%tanh`).
_Avoid_: Trig function, transcendental

**Inverse Trigonometric Node**:
An `Expr` variant (`Asin`, `Acos`, `Atan`) representing principal-branch inverse trig functions, simplified by rules citing `trigi.lisp` (`simp-%asin`, `simp-%acos`, `simp-%atan`).
_Avoid_: Reciprocal trig, arc function

**Factorial Simplification**:
Rewriting and constant folding of `n!` using `sfacts.lisp` (`factcomb`) recurrences; exact integer arguments fold during e-graph analysis.
_Avoid_: Gamma expansion, product notation

**Rational Normalization**:
The `ratsimp`-style merging of quotients (common denominators, sign normalization, quotient-of-quotient) implemented by rules citing `nrat4.lisp` (`$ratsimp`, `$nrat`).
_Avoid_: Fraction reduction, division cleanup

**Named Constant**:
An irrational symbolic constant (`%pi`, `%e`) kept symbolic in the AST and encoded as `MathLang::NamedConst` in the e-graph; evaluated or emitted as `f64`/`f32` constants only at the leaves of the pipeline.
_Avoid_: Float literal, numeric constant

**Matrix Symbol Encoding**:
A lossless prefix-token serialization of `Expr::Matrix` into a single e-graph symbol, allowing matrices to survive saturation and extraction without an opaque sentinel.
_Avoid_: Matrix flattening, matrix placeholder

**Rule Coverage API**:
The `GET /api/rules` endpoint and WebUI "Maxima coverage" panel exposing the provenanced rule catalog and per-Lisp-file counts.
_Avoid_: Rule listing, provenance dump

**Procedural CAS Algorithm**:
A deterministic, directed symbolic manipulation routine (e.g., polynomial factoring, Risch integration, Gruntz limits, ODE solvers) operating directly on `Expr` ASTs outside the e-graph.
_Avoid_: E-graph rewrite, saturation rule, macro transform

**Sparse Distributed Multivariate Polynomial (Poly)**:
A canonical polynomial data structure storing sorted non-zero terms with packed monomial exponent vectors and exact rational coefficients.
_Avoid_: Recursive polynomial, sum-of-products AST, dense polynomial

**Ratsimp (Rational Simplification)**:
The canonical reduction of a rational expression to a coprime quotient of polynomials $P/Q$ via polynomial greatest common divisor (GCD).
_Avoid_: Fraction cancellation, division cleanup, rational normalization

**Hermite Reduction**:
An algebraic integration pre-pass decomposing a rational function into a purely rational derivative part and a square-free denominator integral without full factorization.
_Avoid_: Partial fraction integration, rational reduction

**Rothstein-Trager / Lazard-Rioboo-Trager (LRT)**:
An algebraic algorithm computing logarithmic and arctangent parts of rational integrals via resultant polynomials over square-free denominators.
_Avoid_: Complex log expansion, heuristic integration

**Gruntz Limit Algorithm**:
An asymptotic limit algorithm for exp-log functions that isolates Most Rapidly Varying (MRV) subexpression sets and expands generalized series around $\omega \to 0$.
_Avoid_: L'Hopital's rule, naive Taylor limit


