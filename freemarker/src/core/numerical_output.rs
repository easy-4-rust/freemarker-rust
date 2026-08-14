//! 数值输出插值 —— 对应 Java `freemarker.core.NumericalOutput`
//! （Interpolation 子类；旧式 `#{expr; mNMN}` 语法；minFracDigits/maxFracDigits
//!  控制小数位；hasFormat=false 时 (0, 50) 默认）

/// 对应 Java `NumericalOutput`（ElementKind::Interpolation 的 legacy_min/max_frac 承载）
#[allow(dead_code)]
pub(crate) struct NumericalOutput;
