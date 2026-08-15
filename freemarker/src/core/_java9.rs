//! Java 9 适配接口 —— 对应 Java `freemarker.core._Java9`
//! （JEP 238 Multi-Release JAR；isSupported/isAccessibleAccordingToModuleExports；
//!  Rust 无 Java 模块系统 → PLATFORM_NA）

/// Java 类锚点：`_Java9`（PLATFORM_NA：Rust 无 Java 模块系统）
#[allow(dead_code)]
pub(crate) struct _Java9;
