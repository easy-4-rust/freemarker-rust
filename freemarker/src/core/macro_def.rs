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
    /// 宏定义所在模板的查找名（Java `Macro` 的 `template` 字段——`setLocation` 于
    /// 解析期绑定，TemplateObject.java:55-83；`.caller_template_name` 的调用点词法
    /// 模板判定依赖它，BuiltinVariable.java:264-267）
    pub template_name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MacroParam {
    pub name: String,
    pub default: Option<Expr>,
    pub optional: bool,
    pub catch_all: bool, // args...
}
