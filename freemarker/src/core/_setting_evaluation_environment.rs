//! 设置求值环境 —— 对应 Java `freemarker.core._SettingEvaluationEnvironment`
//! （ThreadLocal 作用域；BeansWrapper 引用；startScope/getCurrent/endScope；
//!  配置设置求值时的对象包装上下文；Rust 无 BeansWrapper → 锚点）

/// Java 类锚点：`_SettingEvaluationEnvironment` 的 Rust 语义由 Settings 直接求值承载
#[allow(dead_code)]
pub(crate) struct _SettingEvaluationEnvironment;
