//! Attribute and configuration parser for symbolic macros.

use sym_core::codegen::FloatType;
use sym_core::saturate::SaturationConfig;

/// Configuration options extracted from macro attributes or arguments.
#[derive(Debug, Clone)]
pub struct MacroConfig {
    /// Target floating-point type (default: f64).
    pub float_type: FloatType,
    /// Fast-math enablement.
    pub fast_math: bool,
    /// Saturation limits and rule settings.
    pub saturation: SaturationConfig,
}

impl Default for MacroConfig {
    fn default() -> Self {
        Self {
            float_type: FloatType::F64,
            fast_math: true,
            saturation: SaturationConfig::default(),
        }
    }
}
