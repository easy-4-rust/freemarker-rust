//! 自定义属性 —— 对应 Java `freemarker.core.CustomAttribute`
//! （SCOPE_ENVIRONMENT=0 / SCOPE_TEMPLATE=1 / SCOPE_CONFIGURATION=2；
//!  ThreadLocal/Template/Configuration 作用域的用户自定义数据；
//!  get()/set() 为 final；Rust 无直接对应 → 锚点）

/// 对应 Java `CustomAttribute`（Rust 无直接对应；用户扩展点）
#[allow(dead_code)]
pub(crate) struct CustomAttribute;

impl CustomAttribute {
    /// Environment 作用域（Java SCOPE_ENVIRONMENT = 0）
    #[allow(dead_code)]
    pub(crate) const SCOPE_ENVIRONMENT: i32 = 0;
    /// Template 作用域（Java SCOPE_TEMPLATE = 1）
    #[allow(dead_code)]
    pub(crate) const SCOPE_TEMPLATE: i32 = 1;
    /// Configuration 作用域（Java SCOPE_CONFIGURATION = 2）
    #[allow(dead_code)]
    pub(crate) const SCOPE_CONFIGURATION: i32 = 2;
}
