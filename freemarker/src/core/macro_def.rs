//! 宏/函数定义 —— 对应 Java `freemarker.core.Macro`
//! （<#macro> / <#function> 定义；参数语义见 docs/04 §4.2）

use crate::core::Element;
use crate::core::Expr;
use crate::span::Span;

#[derive(Debug, Clone)]
pub struct MacroDef {
    pub name: String,
    pub is_function: bool,
    pub params: Vec<MacroParam>,
    pub body: Vec<Element>,
    /// 命名空间限定名（`<@ns.macro>` 中的 ns，None 表示当前命名空间）
    pub namespace: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MacroParam {
    pub name: String,
    pub default: Option<Expr>,
    pub optional: bool,
    pub catch_all: bool, // args...
}
