# WebUI CAS Typed Dispatch, Precedence LaTeX, and Offline-First Architecture

**Status**: Accepted

Context: The maxima-web service previously only exposed basic e-graph simplification and calculus operations, lacked procedural CAS operations (Phases 22–31), loaded frontend scripts and styles from external CDNs, and formatted LaTeX with naive wrapping and syntax bugs. Furthermore, users require air-gapped offline operation without external network dependencies, while the workspace enforces strict explicit configuration without environment variables. We decided to:
1. Route all CAS and algebraic computations through structured, typed requests (`POST /api/compute` with operation name and typed parameter schemas) that invoke `sym-core` procedural algorithms directly, avoiding premature mutation of the core `Expr` AST for command string parsing.
2. Relocate and reimplement the LaTeX formatter inside `sym-core/src/latex.rs` with an operator-precedence model, making clean LaTeX formatting a core library capability covered by workspace test suites.
3. Adopt an offline-first, zero-Node architecture for `maxima-web`: vendor KaTeX (including WOFF2 fonts) and pre-compiled CSS into `static/vendor/`, serve vanilla ES modules, and embed all static assets into the release binary via `rust-embed`.
4. Enforce explicit configuration via CLI arguments (`clap`) and optional TOML configuration files, providing a standalone offline executable while supporting containerized AWS Lambda packaging as a secondary target with command-line parameter passing.
