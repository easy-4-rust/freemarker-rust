//! Java 16 适配接口 —— 对应 Java `freemarker.core._Java16`
//! （JEP 238 Multi-Release JAR；isSupported/isRecord/getComponentAccessors；
//!  Rust 无 Java Record 类型 → PLATFORM_NA）

/// Java 类锚点：`_Java16`（PLATFORM_NA：Rust 无 Java Record 类型）
#[allow(dead_code)]
pub(crate) struct _Java16;
