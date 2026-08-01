//! 模板元素 AST —— 对应 Java `freemarker.core.TemplateElement` 家族
//! （指令产生式映射见 docs/03 §4；各 variant 对应 Java 类注释内联标注）

use crate::core::Expr;
use crate::core::MacroDef;
use crate::span::Span;

#[derive(Debug, Clone)]
pub struct Element {
    pub kind: ElementKind,
    pub span: Span,
}

impl Element {
    pub fn new(kind: ElementKind, span: Span) -> Self {
        Element { kind, span }
    }
}

/// 赋值操作符（对应 AssignmentInstruction 的 8 种形式）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Equals,
    PlusEq,
    MinusEq,
    TimesEq,
    DivideEq,
    ModuloEq,
    PlusPlus,
    MinusMinus,
}

#[derive(Debug, Clone)]
pub enum ElementKind {
    /// 模板文本（whitespace stripping 标记：解析期决定渲染期是否裁剪）
    Text {
        text: String,
        strip_before: bool,
        strip_after: bool,
        /// 原始结束行（token 行号；Java TextBlock 的 endLine 在空白剥离时**不更新**，
        /// 而内容裁剪会改变换行数 —— prev/next 链的行号判定须用原始值）
        orig_end_line: u32,
    },
    /// ${expr} 插值
    Interpolation(Expr),
    /// <#if>（elseif 已扁平化为嵌套 If 的 else 分支）
    If {
        cond: Expr,
        then: Vec<Element>,
        else_: Option<Vec<Element>>,
    },
    /// <#list>（items/sep 为就地元素，Java IteratorBlock + Items/Sep 模型；
    /// var2 = `as k, v` 的键循环变量 —— hashListing，IteratorBlock.java:221-223）
    List {
        seq: Expr,
        var: String,
        var2: Option<String>,
        body: Vec<Element>,
        else_: Option<Vec<Element>>,
    },
    /// <#items as x[, y]>（就地元素 —— 对应 Java `Items.java:29`；render 时由
    /// 最近的 #list 迭代上下文驱动 body 逐项执行，loopForItemsElement 语义）
    Items {
        var: String,
        var2: Option<String>,
        body: Vec<Element>,
    },
    /// <#sep>（就地元素 —— 对应 Java `Sep.java:29`；当前迭代 hasNext 时渲染 body）
    Sep {
        body: Vec<Element>,
    },
    /// <#assign a=1, b=2> 多赋值（对应 Java `AssignmentInstruction`）
    Assignments(Vec<Element>),
    /// <#assign name = expr>（含 += 等操作符）
    Assign {
        target: String,
        expr: Expr,
        op: AssignOp,
        namespace: Option<String>,
    },
    /// <#assign name>body</#assign> 块捕获
    BlockAssign {
        target: String,
        body: Vec<Element>,
        op: AssignOp,
        namespace: Option<String>,
    },
    /// <#global>
    Global {
        target: String,
        expr: Option<Expr>,
        body: Option<Vec<Element>>,
        op: AssignOp,
    },
    /// <#local>
    Local {
        target: String,
        expr: Option<Expr>,
        body: Option<Vec<Element>>,
        op: AssignOp,
    },
    /// <#macro> / <#function>
    Macro {
        def: MacroDef,
    },
    /// <@callee args>body</@callee>
    Call {
        callee: CallTarget,
        args: Vec<(String, Expr)>,
        body: Option<Vec<Element>>,
        /// body 参数名（<@m ; a, b>；对应 Java UnifiedCall.bodyParameters 列表）
        body_params: Vec<String>,
    },
    /// <#nested>（宏体回插）
    Nested {
        args: Vec<Expr>,
        body: Option<Vec<Element>>,
    },
    /// <#switch>
    Switch {
        expr: Expr,
        cases: Vec<CaseDef>,
        default: Option<Vec<Element>>,
        /// default 在源码序列中的位置（0 起始；Java SwitchBlock 子块按源码序，
        /// legacy 怪癖：default 可不在末尾且不后落到后续 case —— switch.ftl 用例）
        default_pos: Option<usize>,
    },
    /// <#attempt>
    Attempt {
        try_: Vec<Element>,
        recover: Vec<Element>,
    },
    Break,
    Continue,
    /// <#return> / <#return expr>
    Return {
        expr: Option<Expr>,
    },
    /// <#stop> / <#stop "msg">
    Stop {
        msg: Option<Expr>,
    },
    Flush,
    /// <#trim>
    Trim(Vec<Element>),
    /// <#-- comment -->
    Comment {
        text: String,
    },
    /// <#include path args...>
    Include {
        path: Expr,
        attrs: Vec<(String, Expr)>,
    },
    /// <#import path as ns>
    Import {
        path: Expr,
        ns: String,
    },
    /// <#escape>
    Escape {
        expr: Expr,
        body: Vec<Element>,
    },
    NoEscape(Vec<Element>),
    AutoEsc(Vec<Element>),
    NoAutoEsc(Vec<Element>),
    /// <#outputformat "HTML">
    OutputFormat {
        name: Expr,
        body: Vec<Element>,
    },
    /// <#compress>
    Compress(Vec<Element>),
    /// <#setting key=value>
    Setting {
        key: String,
        value: Expr,
    },
    /// `[<#ftl encoding="...">]`
    FtlHeader {
        encoding: Option<String>,
    },
    /// <#t>（行首裁剪）
    TrimLineStart,
    /// <#nt>（行首不裁剪）
    NoTrimLineStart,
    /// <#rt>（行尾空白裁剪 —— 对应 Java TrimInstruction(false,true)，RTRIM）
    TrimLineEnd,
    /// <#lt>（行首空白裁剪 —— 对应 Java TrimInstruction(true,false)，LTRIM；
    /// 注：Java 中 `<#lt>` 是裁剪指令而非字面 "<"，v1 曾映射为 RawText 属文档化偏差）
    LeftTrimLine,
    /// <#transform expr>body</#transform>（对应 Java `TransformBlock.java`：
    /// 旧式 TemplateTransformModel 指令；`?interpret` 产物为变换模型）
    Transform {
        expr: Expr,
        body: Vec<Element>,
    },
    /// <#visit expr>（对应 Java `VisitNode.java`：XML 节点访问）
    Visit {
        expr: Expr,
    },
    /// <#recurse expr>（对应 Java `RecurseNode.java`：递归访问子节点）
    Recurse {
        expr: Expr,
    },
    /// <#on name>body</#on>（对应 Java `On.java`：节点名分派）
    On {
        expr: Expr,
        body: Vec<Element>,
    },
    /// <#fallback>（对应 Java `FallbackInstruction.java`：回退到默认节点模板）
    Fallback,
    /// 特殊文本 `<#gt>`（Java 无 `<#gt>` 指令，v1 契约映射为字面 ">"）
    RawText(String),
    /// <#noparse> 原样文本（Java TextBlock(unparsed=true)：与普通文本一样参与空白剥离）
    NoParse {
        text: String,
        strip_before: bool,
        strip_after: bool,
        /// 同 Text.orig_end_line（Java endLine 裁剪时不更新）
        orig_end_line: u32,
    },
}

/// 宏/函数定义（对应 Java `Macro`）
#[derive(Debug, Clone, PartialEq)]
pub enum CallTarget {
    /// 简单名（当前命名空间宏/变量）
    Name(String),
    /// ns.name（命名空间限定）
    Namespaced { ns: String, name: String },
    /// 动态表达式
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct CaseDef {
    pub value: Expr,
    pub body: Vec<Element>,
}
