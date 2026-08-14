//! 惰性 FTL 类型描述 —— 对应 Java `freemarker.core._DelayedFTLTypeDescription`
//! （_DelayedConversionToString 子类；doConversion = ClassUtil.getFTLTypeDescription(tm)；
//!  Rust 由 TModel.type_name 即时获取覆盖）

/// Java 类锚点：`_DelayedFTLTypeDescription`（Rust 由 TModel.type_name 即时获取覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedFTLTypeDescription;
