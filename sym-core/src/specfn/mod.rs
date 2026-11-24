//! Special Functions Engine & Centralized Table Dispatch.
//!
//! Citations:
//! - Maxima 5.50: `src/specfn.lisp`, `src/bessel.lisp`, `src/airy.lisp`, `share/orthopoly/orthopoly.lisp`.
//! - ADR-0011: Special Functions, Operational Laplace Transforms, and Scoped Assumptions Architecture.

pub mod airy;
pub mod bessel;
pub mod gamma;
pub mod orthopoly;

use crate::expr::Expr;

/// Enumeration of non-elementary special functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SpecialFnKind {
    /// $\Gamma(z)$ — Euler Gamma function
    Gamma,
    /// $\beta(a, b)$ — Euler Beta function
    Beta,
    /// $\psi(n, z)$ — Polygamma function (or digamma if $n=0$)
    Psi,
    /// $\text{erf}(z)$ — Gauss Error function
    Erf,
    /// $\text{erfc}(z)$ — Complementary Error function
    Erfc,
    /// $J_\nu(z)$ — Bessel function of the first kind
    BesselJ,
    /// $Y_\nu(z)$ — Bessel function of the second kind
    BesselY,
    /// $I_\nu(z)$ — Modified Bessel function of the first kind
    BesselI,
    /// $K_\nu(z)$ — Modified Bessel function of the second kind
    BesselK,
    /// $\text{Ai}(z)$ — Airy function of the first kind
    AiryAi,
    /// $\text{Ai}'(z)$ — Derivative of the Airy Ai function
    AiryAiPrime,
    /// $\text{Bi}(z)$ — Airy function of the second kind
    AiryBi,
    /// $\text{Bi}'(z)$ — Derivative of the Airy Bi function
    AiryBiPrime,
    /// $P_n(x)$ — Legendre polynomial of degree $n$
    LegendreP,
    /// $T_n(x)$ — Chebyshev polynomial of the first kind
    ChebyshevT,
    /// $U_n(x)$ — Chebyshev polynomial of the second kind
    ChebyshevU,
    /// $H_n(x)$ — Hermite polynomial (physicist's convention)
    HermiteH,
    /// $L_n(x)$ — Laguerre polynomial
    LaguerreL,
}

impl SpecialFnKind {
    /// Canonical Maxima function name.
    pub fn as_str(&self) -> &'static str {
        match self {
            SpecialFnKind::Gamma => "gamma",
            SpecialFnKind::Beta => "beta",
            SpecialFnKind::Psi => "psi",
            SpecialFnKind::Erf => "erf",
            SpecialFnKind::Erfc => "erfc",
            SpecialFnKind::BesselJ => "bessel_j",
            SpecialFnKind::BesselY => "bessel_y",
            SpecialFnKind::BesselI => "bessel_i",
            SpecialFnKind::BesselK => "bessel_k",
            SpecialFnKind::AiryAi => "airy_ai",
            SpecialFnKind::AiryAiPrime => "airy_dai",
            SpecialFnKind::AiryBi => "airy_bi",
            SpecialFnKind::AiryBiPrime => "airy_dbi",
            SpecialFnKind::LegendreP => "legendre_p",
            SpecialFnKind::ChebyshevT => "chebyshev_t",
            SpecialFnKind::ChebyshevU => "chebyshev_u",
            SpecialFnKind::HermiteH => "hermite",
            SpecialFnKind::LaguerreL => "laguerre",
        }
    }

    /// Parse from Maxima identifier string.
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "gamma" | "%gamma" => Some(SpecialFnKind::Gamma),
            "beta" | "%beta" => Some(SpecialFnKind::Beta),
            "psi" | "%psi" => Some(SpecialFnKind::Psi),
            "erf" | "%erf" => Some(SpecialFnKind::Erf),
            "erfc" | "%erfc" => Some(SpecialFnKind::Erfc),
            "bessel_j" | "%bessel_j" => Some(SpecialFnKind::BesselJ),
            "bessel_y" | "%bessel_y" => Some(SpecialFnKind::BesselY),
            "bessel_i" | "%bessel_i" => Some(SpecialFnKind::BesselI),
            "bessel_k" | "%bessel_k" => Some(SpecialFnKind::BesselK),
            "airy_ai" | "%airy_ai" => Some(SpecialFnKind::AiryAi),
            "airy_dai" | "%airy_dai" => Some(SpecialFnKind::AiryAiPrime),
            "airy_bi" | "%airy_bi" => Some(SpecialFnKind::AiryBi),
            "airy_dbi" | "%airy_dbi" => Some(SpecialFnKind::AiryBiPrime),
            "legendre_p" | "$legendre_p" => Some(SpecialFnKind::LegendreP),
            "chebyshev_t" | "$chebyshev_t" => Some(SpecialFnKind::ChebyshevT),
            "chebyshev_u" | "$chebyshev_u" => Some(SpecialFnKind::ChebyshevU),
            "hermite" | "$hermite" => Some(SpecialFnKind::HermiteH),
            "laguerre" | "$laguerre" => Some(SpecialFnKind::LaguerreL),
            _ => None,
        }
    }

    /// Expected argument arity.
    pub fn expected_arity(&self) -> usize {
        match self {
            SpecialFnKind::Gamma
            | SpecialFnKind::Erf
            | SpecialFnKind::Erfc
            | SpecialFnKind::AiryAi
            | SpecialFnKind::AiryAiPrime
            | SpecialFnKind::AiryBi
            | SpecialFnKind::AiryBiPrime => 1,
            SpecialFnKind::Beta
            | SpecialFnKind::BesselJ
            | SpecialFnKind::BesselY
            | SpecialFnKind::BesselI
            | SpecialFnKind::BesselK
            | SpecialFnKind::LegendreP
            | SpecialFnKind::ChebyshevT
            | SpecialFnKind::ChebyshevU
            | SpecialFnKind::HermiteH
            | SpecialFnKind::LaguerreL => 2,
            SpecialFnKind::Psi => 1, // Can be 1 (digamma) or 2 (polygamma)
        }
    }

    /// Evaluate known exact values.
    pub fn eval_exact(&self, args: &[Expr]) -> Option<Expr> {
        match self {
            SpecialFnKind::Gamma if !args.is_empty() => gamma::gamma_eval(&args[0]),
            SpecialFnKind::Beta if args.len() >= 2 => gamma::beta_eval(&args[0], &args[1]),
            SpecialFnKind::Erf if !args.is_empty() => gamma::erf_eval(&args[0]),
            SpecialFnKind::Erfc if !args.is_empty() => gamma::erfc_eval(&args[0]),
            SpecialFnKind::BesselJ if args.len() >= 2 => bessel::bessel_j_eval(&args[0], &args[1]),
            SpecialFnKind::BesselI if args.len() >= 2 => bessel::bessel_i_eval(&args[0], &args[1]),
            _ => None,
        }
    }

    /// Render to LaTeX format.
    pub fn to_latex(&self, args: &[Expr]) -> String {
        let fmt_arg = |i: usize| -> String {
            if i < args.len() {
                crate::latex::to_latex(&args[i])
            } else {
                String::new()
            }
        };
        match self {
            SpecialFnKind::Gamma => format!(r"\Gamma\left({}\right)", fmt_arg(0)),
            SpecialFnKind::Beta => {
                format!(r"\mathrm{{B}}\left({}, {}\right)", fmt_arg(0), fmt_arg(1))
            }
            SpecialFnKind::Psi => {
                if args.len() <= 1 {
                    format!(r"\psi\left({}\right)", fmt_arg(0))
                } else {
                    format!(r"\psi^{{({})}}\left({}\right)", fmt_arg(0), fmt_arg(1))
                }
            }
            SpecialFnKind::Erf => format!(r"\mathrm{{erf}}\left({}\right)", fmt_arg(0)),
            SpecialFnKind::Erfc => format!(r"\mathrm{{erfc}}\left({}\right)", fmt_arg(0)),
            SpecialFnKind::BesselJ => format!(r"J_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1)),
            SpecialFnKind::BesselY => format!(r"Y_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1)),
            SpecialFnKind::BesselI => format!(r"I_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1)),
            SpecialFnKind::BesselK => format!(r"K_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1)),
            SpecialFnKind::AiryAi => format!(r"\mathrm{{Ai}}\left({}\right)", fmt_arg(0)),
            SpecialFnKind::AiryAiPrime => format!(r"\mathrm{{Ai}}'\left({}\right)", fmt_arg(0)),
            SpecialFnKind::AiryBi => format!(r"\mathrm{{Bi}}\left({}\right)", fmt_arg(0)),
            SpecialFnKind::AiryBiPrime => format!(r"\mathrm{{Bi}}'\left({}\right)", fmt_arg(0)),
            SpecialFnKind::LegendreP => format!(r"P_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1)),
            SpecialFnKind::ChebyshevT => {
                format!(r"T_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1))
            }
            SpecialFnKind::ChebyshevU => {
                format!(r"U_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1))
            }
            SpecialFnKind::HermiteH => format!(r"H_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1)),
            SpecialFnKind::LaguerreL => format!(r"L_{{{}}}\left({}\right)", fmt_arg(0), fmt_arg(1)),
        }
    }
}

impl std::str::FromStr for SpecialFnKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_name(s).ok_or(())
    }
}

/// Table-driven differentiation for special functions:
///
/// Dispatches to the corresponding module using the chain rule.
pub fn diff_special_fn(kind: SpecialFnKind, args: &[Expr], var: &str) -> Expr {
    match kind {
        SpecialFnKind::Gamma => gamma::diff_gamma(args, var),
        SpecialFnKind::Beta => gamma::diff_beta(args, var),
        SpecialFnKind::Psi => gamma::diff_psi(args, var),
        SpecialFnKind::Erf => gamma::diff_erf(args, var),
        SpecialFnKind::Erfc => gamma::diff_erfc(args, var),
        SpecialFnKind::BesselJ => bessel::diff_bessel_j(args, var),
        SpecialFnKind::BesselY => bessel::diff_bessel_y(args, var),
        SpecialFnKind::BesselI => bessel::diff_bessel_i(args, var),
        SpecialFnKind::BesselK => bessel::diff_bessel_k(args, var),
        SpecialFnKind::AiryAi => airy::diff_airy_ai(args, var),
        SpecialFnKind::AiryAiPrime => airy::diff_airy_ai_prime(args, var),
        SpecialFnKind::AiryBi => airy::diff_airy_bi(args, var),
        SpecialFnKind::AiryBiPrime => airy::diff_airy_bi_prime(args, var),
        SpecialFnKind::LegendreP => orthopoly::diff_legendre_p(args, var),
        SpecialFnKind::ChebyshevT => orthopoly::diff_chebyshev_t(args, var),
        SpecialFnKind::ChebyshevU => orthopoly::diff_chebyshev_u(args, var),
        SpecialFnKind::HermiteH => orthopoly::diff_hermite_h(args, var),
        SpecialFnKind::LaguerreL => orthopoly::diff_laguerre_l(args, var),
    }
}

/// Decode a serialized special function symbol.
pub fn decode_special_fn(s: &str) -> Option<Expr> {
    let mut parts = s.splitn(2, ':');
    let kind_str = parts.next()?;
    let kind = SpecialFnKind::from_name(kind_str)?;
    let args_str = parts.next().unwrap_or("");
    if args_str.is_empty() {
        return Some(Expr::special_fn(kind, Vec::new()));
    }
    let arg_tokens: Vec<&str> = args_str.split(',').collect();
    let mut args = Vec::with_capacity(arg_tokens.len());
    for tok in arg_tokens {
        args.push(Expr::var(tok.trim()));
    }
    Some(Expr::special_fn(kind, args))
}
