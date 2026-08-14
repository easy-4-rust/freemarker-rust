//! Java 16 适配实现 —— 对应 Java `freemarker.core._Java16Impl`
//! （默认实现：isSupported=false；isRecord/getComponentAccessors 抛异常；
//!  Java 16+ 版本由 Multi-Release JAR 替换；Rust 无对应 → PLATFORM_NA）

/// Java 类锚点：`_Java16Impl`（PLATFORM_NA：Rust 无 Java Record 类型）
#[allow(dead_code)]
pub(crate) struct _Java16Impl;
