//! 核心字符串工具 —— 对应 Java `freemarker.core._CoreStringUtils`
//! （FTL 标识符转义：toFTLIdentifierReferenceAfterDot/toFTLTopLevelIdentifierReference；
//!  backslashEscapeIdentifier；Rust 解析器内联处理标识符转义）

/// Java 类锚点：`_CoreStringUtils` 的 Rust 语义由解析器标识符处理承载
#[allow(dead_code)]
pub(crate) struct _CoreStringUtils;
