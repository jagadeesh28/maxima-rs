//! E-Graph language definition and constant folding analysis.
//!
//! Maps [`Expr`](crate::expr::Expr) into [`egg`]'s e-graph intermediate representation
//! using [`MathLang`]. Exact constants are represented as [`BigRational`].

use egg::{Analysis, Id, RecExpr, Symbol, define_language};
use num_rational::BigRational;

use crate::expr::Expr;
use crate::rational::Rat;

define_language! {
    /// egg Language enum representing algebraic expressions.
    pub enum MathLang {
        // Binary operations
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "/" = Div([Id; 2]),
        "^" = Pow([Id; 2]),

        // Unary operations
        "neg" = Neg([Id; 1]),
        "sin" = Sin([Id; 1]),
        "cos" = Cos([Id; 1]),
        "tan" = Tan([Id; 1]),
        "exp" = Exp([Id; 1]),
        "ln" = Ln([Id; 1]),
        "sqrt" = Sqrt([Id; 1]),

        // Leaf nodes
        Const(BigRational),
        Var(Symbol),
        Tensor(Symbol),

        // Calculus
        "d" = Derivative([Id; 2]),
        "int" = Integral([Id; 2]),
        "sum" = Sum([Id; 4]),

        // Matrix (Serialized as a flat symbol for now)
        Matrix(Symbol),
    }
}

/// Constant analysis data for e-classes.
///
/// If an e-class is known to be a constant rational, it stores `Some(BigRational)`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MathAnalysis;

impl Analysis<MathLang> for MathAnalysis {
    type Data = Option<BigRational>;

    fn merge(&mut self, to: &mut Self::Data, from: Self::Data) -> egg::DidMerge {
        egg::merge_max(to, from)
    }

    fn make(egraph: &egg::EGraph<MathLang, Self>, enode: &MathLang) -> Self::Data {
        let get_const = |id: &Id| egraph[*id].data.clone();

        match enode {
            MathLang::Const(c) => Some(c.clone()),
            MathLang::Var(_) | MathLang::Tensor(_) => None,
            MathLang::Add([a, b]) => {
                let ca = get_const(a)?;
                let cb = get_const(b)?;
                Some(Rat::add(&ca, &cb))
            }
            MathLang::Sub([a, b]) => {
                let ca = get_const(a)?;
                let cb = get_const(b)?;
                Some(Rat::sub(&ca, &cb))
            }
            MathLang::Mul([a, b]) => {
                let ca = get_const(a)?;
                let cb = get_const(b)?;
                Some(Rat::mul(&ca, &cb))
            }
            MathLang::Div([a, b]) => {
                let ca = get_const(a)?;
                let cb = get_const(b)?;
                Rat::div(&ca, &cb)
            }
            MathLang::Pow([a, b]) => {
                let ca = get_const(a)?;
                let cb = get_const(b)?;
                Rat::pow_int(&ca, &cb)
            }
            MathLang::Neg([a]) => {
                let ca = get_const(a)?;
                Some(Rat::negate(&ca))
            }
            MathLang::Sin(_)
            | MathLang::Cos(_)
            | MathLang::Tan(_)
            | MathLang::Exp(_)
            | MathLang::Ln(_)
            | MathLang::Sqrt(_)
            | MathLang::Derivative(_)
            | MathLang::Integral(_)
            | MathLang::Sum(_)
            | MathLang::Matrix(_) => None,
        }
    }

    fn modify(egraph: &mut egg::EGraph<MathLang, Self>, id: Id) {
        if let Some(c) = egraph[id].data.clone() {
            let added = egraph.add(MathLang::Const(c));
            egraph.union(id, added);
        }
    }
}

/// Conversion between [`Expr`] and [`egg::RecExpr<MathLang>`].
impl Expr {
    /// Convert an [`Expr`] into an [`egg::RecExpr<MathLang>`].
    pub fn to_recexpr(&self) -> RecExpr<MathLang> {
        let mut recexpr = RecExpr::default();
        self.build_recexpr(&mut recexpr);
        recexpr
    }

    fn build_recexpr(&self, recexpr: &mut RecExpr<MathLang>) -> Id {
        match self {
            Expr::Var(name) => recexpr.add(MathLang::Var(Symbol::from(name.as_str()))),
            Expr::Const(c) => recexpr.add(MathLang::Const(c.clone())),
            Expr::Add(a, b) => {
                let id_a = a.build_recexpr(recexpr);
                let id_b = b.build_recexpr(recexpr);
                recexpr.add(MathLang::Add([id_a, id_b]))
            }
            Expr::Sub(a, b) => {
                let id_a = a.build_recexpr(recexpr);
                let id_b = b.build_recexpr(recexpr);
                recexpr.add(MathLang::Sub([id_a, id_b]))
            }
            Expr::Mul(a, b) => {
                let id_a = a.build_recexpr(recexpr);
                let id_b = b.build_recexpr(recexpr);
                recexpr.add(MathLang::Mul([id_a, id_b]))
            }
            Expr::Div(a, b) => {
                let id_a = a.build_recexpr(recexpr);
                let id_b = b.build_recexpr(recexpr);
                recexpr.add(MathLang::Div([id_a, id_b]))
            }
            Expr::Pow(a, b) => {
                let id_a = a.build_recexpr(recexpr);
                let id_b = b.build_recexpr(recexpr);
                recexpr.add(MathLang::Pow([id_a, id_b]))
            }
            Expr::Neg(a) => {
                let id_a = a.build_recexpr(recexpr);
                recexpr.add(MathLang::Neg([id_a]))
            }
            Expr::Sin(a) => {
                let id_a = a.build_recexpr(recexpr);
                recexpr.add(MathLang::Sin([id_a]))
            }
            Expr::Cos(a) => {
                let id_a = a.build_recexpr(recexpr);
                recexpr.add(MathLang::Cos([id_a]))
            }
            Expr::Tan(a) => {
                let id_a = a.build_recexpr(recexpr);
                recexpr.add(MathLang::Tan([id_a]))
            }
            Expr::Exp(a) => {
                let id_a = a.build_recexpr(recexpr);
                recexpr.add(MathLang::Exp([id_a]))
            }
            Expr::Ln(a) => {
                let id_a = a.build_recexpr(recexpr);
                recexpr.add(MathLang::Ln([id_a]))
            }
            Expr::Sqrt(a) => {
                let id_a = a.build_recexpr(recexpr);
                recexpr.add(MathLang::Sqrt([id_a]))
            }
            Expr::Tensor(name, indices) => {
                let encoded = crate::tensor::encode_tensor_symbol(name, indices);
                recexpr.add(MathLang::Tensor(Symbol::from(encoded.as_str())))
            }
            Expr::Derivative(a, b) => {
                let id_a = a.build_recexpr(recexpr);
                let id_b = b.build_recexpr(recexpr);
                recexpr.add(MathLang::Derivative([id_a, id_b]))
            }
            Expr::Integral(a, b) => {
                let id_a = a.build_recexpr(recexpr);
                let id_b = b.build_recexpr(recexpr);
                recexpr.add(MathLang::Integral([id_a, id_b]))
            }
            Expr::Sum(a, b, lower, upper) => {
                let id_a = a.build_recexpr(recexpr);
                let id_b = b.build_recexpr(recexpr);
                let id_l = lower.build_recexpr(recexpr);
                let id_u = upper.build_recexpr(recexpr);
                recexpr.add(MathLang::Sum([id_a, id_b, id_l, id_u]))
            }
            Expr::Matrix(_mat) => {
                // Matrices are flattened to an opaque symbol to avoid egg Language complexity for now
                recexpr.add(MathLang::Matrix(Symbol::from(
                    "matrix_unsupported_in_egraph",
                )))
            }
        }
    }

    /// Construct an [`Expr`] from an [`egg::RecExpr<MathLang>`].
    pub fn from_recexpr(recexpr: &RecExpr<MathLang>) -> Self {
        let nodes = recexpr.as_ref();
        if nodes.is_empty() {
            return Expr::int(0);
        }
        Self::from_recexpr_id(nodes, Id::from(nodes.len() - 1))
    }

    fn from_recexpr_id(nodes: &[MathLang], id: Id) -> Self {
        let node = &nodes[usize::from(id)];
        match node {
            MathLang::Var(s) => Expr::Var(s.to_string()),
            MathLang::Const(c) => Expr::Const(c.clone()),
            MathLang::Add([a, b]) => Expr::add(
                Self::from_recexpr_id(nodes, *a),
                Self::from_recexpr_id(nodes, *b),
            ),
            MathLang::Sub([a, b]) => Expr::sub(
                Self::from_recexpr_id(nodes, *a),
                Self::from_recexpr_id(nodes, *b),
            ),
            MathLang::Mul([a, b]) => Expr::mul(
                Self::from_recexpr_id(nodes, *a),
                Self::from_recexpr_id(nodes, *b),
            ),
            MathLang::Div([a, b]) => Expr::div(
                Self::from_recexpr_id(nodes, *a),
                Self::from_recexpr_id(nodes, *b),
            ),
            MathLang::Pow([a, b]) => Expr::pow(
                Self::from_recexpr_id(nodes, *a),
                Self::from_recexpr_id(nodes, *b),
            ),
            MathLang::Neg([a]) => Expr::neg(Self::from_recexpr_id(nodes, *a)),
            MathLang::Sin([a]) => Expr::sin(Self::from_recexpr_id(nodes, *a)),
            MathLang::Cos([a]) => Expr::cos(Self::from_recexpr_id(nodes, *a)),
            MathLang::Tan([a]) => Expr::tan(Self::from_recexpr_id(nodes, *a)),
            MathLang::Exp([a]) => Expr::exp(Self::from_recexpr_id(nodes, *a)),
            MathLang::Ln([a]) => Expr::ln(Self::from_recexpr_id(nodes, *a)),
            MathLang::Sqrt([a]) => Expr::sqrt(Self::from_recexpr_id(nodes, *a)),
            MathLang::Tensor(s) => {
                if let Some((name, indices)) = crate::tensor::decode_tensor_symbol(s.as_str()) {
                    Expr::Tensor(name, indices)
                } else {
                    // Fallback to avoid panics if someone constructed a weird symbol
                    Expr::var(s.as_str())
                }
            }
            MathLang::Derivative([a, b]) => Expr::derivative(
                Self::from_recexpr_id(nodes, *a),
                Self::from_recexpr_id(nodes, *b),
            ),
            MathLang::Integral([a, b]) => Expr::integral(
                Self::from_recexpr_id(nodes, *a),
                Self::from_recexpr_id(nodes, *b),
            ),
            MathLang::Sum([a, b, l, u]) => Expr::sum(
                Self::from_recexpr_id(nodes, *a),
                Self::from_recexpr_id(nodes, *b),
                Self::from_recexpr_id(nodes, *l),
                Self::from_recexpr_id(nodes, *u),
            ),
            MathLang::Matrix(_) => {
                // Return an empty matrix to prevent panic, since we flattened them out
                Expr::Matrix(vec![])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_conversion() {
        let x = Expr::var("x");
        let expr = Expr::add(
            Expr::mul(Expr::int(2), x.clone()),
            Expr::sin(Expr::pow(x.clone(), Expr::int(3))),
        );
        let recexpr = expr.to_recexpr();
        let recovered = Expr::from_recexpr(&recexpr);
        assert_eq!(expr, recovered);
    }

    #[test]
    fn test_const_analysis_add() {
        let mut egraph = egg::EGraph::<MathLang, MathAnalysis>::default();
        let c1 = egraph.add(MathLang::Const(Rat::from_int(2)));
        let c2 = egraph.add(MathLang::Const(Rat::from_int(3)));
        let add = egraph.add(MathLang::Add([c1, c2]));
        egraph.rebuild();
        assert_eq!(egraph[add].data, Some(Rat::from_int(5)));
    }
}
