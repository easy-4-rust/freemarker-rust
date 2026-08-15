//! 参数角色 —— 对应 Java `freemarker.core.ParameterRole`
//! （常量枚举类：LEFT_HAND_OPERAND/RIGHT_HAND_OPERAND/ENCLOSED_OPERAND/ITEM_VALUE/
//!  ITEM_KEY/ASSIGNMENT_TARGET/ASSIGNMENT_OPERATOR/ASSIGNMENT_SOURCE/VARIABLE_SCOPE/
//!  NAMESPACE/ERROR_HANDLER 等；TemplateObject.getParameterValue/getParameterRole 使用；
//!  AST 调试/dump 用途；当前 Rust 无公开 AST API → 锚点）

/// 对应 Java `ParameterRole`（AST 调试用途；当前 Rust 无公开 AST API）
#[allow(dead_code)]
pub(crate) struct ParameterRole;

impl ParameterRole {
    /// 未知角色
    #[allow(dead_code)]
    pub(crate) const UNKNOWN: &'static str = "[unknown role]";
    /// 左操作数
    #[allow(dead_code)]
    pub(crate) const LEFT_HAND_OPERAND: &'static str = "left-hand operand";
    /// 右操作数
    #[allow(dead_code)]
    pub(crate) const RIGHT_HAND_OPERAND: &'static str = "right-hand operand";
    /// 包围操作数
    #[allow(dead_code)]
    pub(crate) const ENCLOSED_OPERAND: &'static str = "enclosed operand";
    /// 项值
    #[allow(dead_code)]
    pub(crate) const ITEM_VALUE: &'static str = "item value";
    /// 项键
    #[allow(dead_code)]
    pub(crate) const ITEM_KEY: &'static str = "item key";
    /// 赋值目标
    #[allow(dead_code)]
    pub(crate) const ASSIGNMENT_TARGET: &'static str = "assignment target";
    /// 赋值操作符
    #[allow(dead_code)]
    pub(crate) const ASSIGNMENT_OPERATOR: &'static str = "assignment operator";
    /// 赋值源
    #[allow(dead_code)]
    pub(crate) const ASSIGNMENT_SOURCE: &'static str = "assignment source";
    /// 变量作用域
    #[allow(dead_code)]
    pub(crate) const VARIABLE_SCOPE: &'static str = "variable scope";
    /// 命名空间
    #[allow(dead_code)]
    pub(crate) const NAMESPACE: &'static str = "namespace";
    /// 错误处理器
    #[allow(dead_code)]
    pub(crate) const ERROR_HANDLER: &'static str = "error handler";
}
