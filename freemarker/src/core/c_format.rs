//! C 格式抽象基类 —— 对应 Java `freemarker.core.CFormat`
//! （定义 ?c/?cn 的格式化接口；Rust 侧由 `CFormatKind` 枚举承载语义，
//!  实际格式化逻辑在 `builtins/format.rs`）

/// Java 抽象类锚点：`CFormat`（Rust 侧由 `builtins::format::CFormatKind` 枚举承载）
///
/// Java `CFormat` 定义了 `formatNumber`/`formatBoolean`/`formatString` 等抽象方法，
/// Rust 用 `CFormatKind` 枚举 + `format_c_string` 等函数实现等价分派。
#[allow(dead_code)]
pub(crate) struct CFormat;

impl CFormat {
    /// Java `CFormat.NUMBER_FORMAT_FLAG`（数字格式化标志）
    #[allow(dead_code)]
    pub(crate) const NUMBER_FORMAT_FLAG: i32 = 1;
    /// Java `CFormat.BOOLEAN_FORMAT_FLAG`（布尔格式化标志）
    #[allow(dead_code)]
    pub(crate) const BOOLEAN_FORMAT_FLAG: i32 = 2;
    /// Java `CFormat.STRING_FORMAT_FLAG`（字符串格式化标志）
    #[allow(dead_code)]
    pub(crate) const STRING_FORMAT_FLAG: i32 = 4;
}
