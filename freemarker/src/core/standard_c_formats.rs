//! 标准 C 格式注册表 —— 对应 Java `freemarker.core.StandardCFormats`
//! （管理 CFormat 实例的注册表；Rust 侧由 `CFormatKind` 枚举 + `CFormatKind::parse` 承载）

/// Java 类锚点：`StandardCFormats`（Rust 侧由 `builtins::format::CFormatKind` 枚举承载）
///
/// Java `StandardCFormats` 维护一个 `Map<String, CFormat>` 注册表，
/// Rust 用 `CFormatKind::parse(name)` 枚举解析实现等价功能。
#[allow(dead_code)]
pub(crate) struct StandardCFormats;
