//! 对象构建器设置求值器 —— 对应 Java `freemarker.core._ObjectBuilderSettingEvaluator`
//! （解析设置值字符串为 Java 对象实例；INSTANCE 字段/build 方法/Builder 后缀约定；
//!  shorthand 映射；Rust 由 NewBuiltinClassResolver::parse 承载部分类似语义）

/// Java 类锚点：`_ObjectBuilderSettingEvaluator`
/// （Rust 由 NewBuiltinClassResolver::parse 与 Settings 直接求值承载）
#[allow(dead_code)]
pub(crate) struct _ObjectBuilderSettingEvaluator;
