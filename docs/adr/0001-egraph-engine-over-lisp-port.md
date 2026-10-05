# E-Graph Engine Over Direct Maxima Lisp Port

Context: We need to bring Maxima's algebraic manipulation and symbolic optimization into a high-performance Rust ecosystem to generate optimized mathematical code. We decided to implement an e-graph equality saturation engine (using `egg`) and extract algebraic rules rather than translating legacy Maxima Common Lisp source code line-by-line. This avoids importing decades of mutable Lisp s-expression idioms and dynamic global state into Rust, while gaining provable rule-based equality saturation and optimal expression cost extraction.
