//! 惰性获取规范形式 —— 对应 Java `freemarker.core._DelayedGetCanonicalForm`
//! （_DelayedConversionToString 子类；doConversion = TemplateObject.getCanonicalForm()；
//!  Rust 由 Element/Expr 的 Display 或 dump 方法覆盖）

/// Java 类锚点：`_DelayedGetCanonicalForm`（Rust 由 Element/Expr dump 覆盖）
#[allow(dead_code)]
pub(crate) struct _DelayedGetCanonicalForm;
