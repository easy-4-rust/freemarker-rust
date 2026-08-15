//! Java 9 适配实现 —— 对应 Java `freemarker.core._Java9Impl`
//! （默认实现：isSupported=false；isAccessibleAccordingToModuleExports 抛异常；
//!  Java 9+ 版本由 Multi-Release JAR 替换；Rust 无对应 → PLATFORM_NA）

/// Java 类锚点：`_Java9Impl`（PLATFORM_NA：Rust 无 Java 模块系统）
#[allow(dead_code)]
pub(crate) struct _Java9Impl;
