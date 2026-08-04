//! 表达式 AST —— 对应 Java `freemarker.core.Expression` 家族
//! （产生式映射见 docs/03 §3；此文件是解析器与渲染引擎的共享契约 +
//! expression/ 子目录的聚合模块——各表达式类独立文件，一文件一 Java 对象）
//! 各 variant 对应 Java 类：Add→AddConcatExpression（expression/add_concat_expression.rs）、
//! And→AndExpression、Or→OrExpression、Sub/Mul/Div/Mod→ArithmeticExpression
//! （expression/arithmetic_expression.rs）、Eq/NotEq/Gt/Gte/Lt/Lte→ComparisonExpression
//! （expression/comparison_expression.rs）、Str/InterpStr→StringLiteral、
//! Num→NumberLiteral、Bool→BooleanLiteral、Ident→Identifier、ListLit→ListLiteral、
//! HashLit→HashLiteral（expression/hash_literal.rs）、Paren→ParentheticalExpression、
//! Range→Range、BuiltIn→BuiltIn、Lambda→LocalLambdaExpression、Dot→DotVariable、
//! DynKey→DynamicKey、Default→DefaultToExpression、Exists→ExistsExpression 等

mod add_concat_expression;
mod and_expression;
mod arithmetic_expression;
mod boolean_literal;
mod bounded_range_model;
mod builtin_variable;
mod comparison_expression;
mod default_to_expression;
mod dot;
mod dynamic_key_name;
mod exists_expression;
mod hash_literal;
mod identifier;
mod list_literal;
mod listable_right_unbounded_range_model;
mod local_lambda_expression;
mod method_call;
mod nonlistable_right_unbounded_range_model;
mod not_expression;
mod number_literal;
mod or_expression;
mod parenthetical_expression;
mod range;
mod string_literal;
mod unary_plus_minus_expression;

pub use add_concat_expression::AddConcatExpression;
pub use and_expression::AndExpression;
pub use arithmetic_expression::{ArithmeticExpression, NumOp};
pub use boolean_literal::BooleanLiteral;
pub(crate) use bounded_range_model::bounded_range_model;
pub use builtin_variable::BuiltinVariable;
pub(crate) use comparison_expression::compare_numbers;
pub use comparison_expression::{compare_models, CmpOp, ComparisonExpression};
pub use default_to_expression::DefaultToExpression;
pub(crate) use dot::dot_builtin_chain;
pub use dot::Dot;
pub use dynamic_key_name::DynamicKeyName;
pub use exists_expression::ExistsExpression;
pub use hash_literal::HashLiteral;
pub use identifier::Identifier;
pub use list_literal::ListLiteral;
pub(crate) use listable_right_unbounded_range_model::listable_right_unbounded_range_model;
pub use local_lambda_expression::LocalLambdaExpression;
pub use method_call::MethodCall;
pub(crate) use nonlistable_right_unbounded_range_model::nonlistable_right_unbounded_range_model;
pub use not_expression::NotExpression;
pub use number_literal::NumberLiteral;
pub use or_expression::OrExpression;
pub use parenthetical_expression::ParentheticalExpression;
pub use range::Range;
pub(crate) use string_literal::eval_interp_str;
pub use string_literal::StringLiteral;
pub use unary_plus_minus_expression::UnaryPlusMinusExpression;

use crate::span::Span;
use crate::value::TNumber;

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self {
        Expr { kind, span }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// 字符串字面量（无插值）
    Str(String),
    /// 字符串字面量（含 ${} 插值片段）
    InterpStr(Vec<StrPart>),
    /// 数值字面量
    Num(TNumber),
    /// 布尔字面量
    Bool(bool),
    /// 标识符（变量引用；对应 Identifier）
    Ident(String),
    /// 点访问 obj.name（对应 DotVariable）
    Dot {
        target: Box<Expr>,
        name: String,
    },
    /// 动态键 `obj[key]`（对应 DynamicKey）
    DynKey {
        target: Box<Expr>,
        key: Box<Expr>,
    },
    /// 方法调用 expr(args)（对应 MethodCall）
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    /// 一元负号（对应 UnaryPlusMinusExpression）
    UnaryMinus(Box<Expr>),
    /// 逻辑非（对应 NotExpression）
    Not(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    NotEq(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Gte(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Lte(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    /// 范围表达式（对应 Range；END_SIZE_LIMITED 为 ..*）
    Range {
        start: Box<Expr>,
        end: Option<Box<Expr>>,
        kind: RangeKind,
    },
    /// 缺失默认 expr!default（对应 DefaultToExpression；default=None 为 expr!）
    Default {
        target: Box<Expr>,
        default: Option<Box<Expr>>,
    },
    /// 存在性 expr??（对应 ExistsExpression）
    Exists(Box<Expr>),
    /// 内建函数 expr?name 或 expr?name(args)（对应 BuiltIn）
    BuiltIn {
        target: Box<Expr>,
        name: String,
        args: Option<Vec<Expr>>,
    },
    /// 列表字面量（对应 ListLiteral）
    ListLit(Vec<Expr>),
    /// 哈希字面量（对应 HashLiteral）
    HashLit(Vec<(Expr, Expr)>),
    /// lambda 表达式 x -> expr / (x, y) -> expr（对应 LocalLambdaExpression；
    /// Java LambdaParameterList 支持多参数，params 为参数名列表）
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    /// 括号（对应 ParentheticalExpression）
    Paren(Box<Expr>),
    /// 内置变量（对应 BuiltinVariable：true/false/now 等）
    BuiltinVar(BuiltinVar),
}

#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    Text(String),
    Interp(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeKind {
    /// .. 含端
    Inclusive,
    /// ..< 排端
    Exclusive,
    /// ..* 无界
    SizeLimited,
}

/// 内置变量 —— 对应 Java `BuiltinVariable.java:43-82` 的 SPEC_VAR_NAMES 全清单
/// （`._eval` :186-300 语义；评估见 eval.rs eval_builtin_var）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinVar {
    True,
    False,
    Now,
    /// `.namespace`（当前命名空间，Java getCurrentNamespace）
    Namespace,
    /// `.main`（主命名空间，Java getMainNamespace）
    Main,
    /// `.globals`（全局命名空间，Java getGlobalVariables）
    Globals,
    /// `.locals`（当前宏帧局部变量哈希；宏外为 null —— Java BuiltinVariable.java:193-196）
    Locals,
    /// `.data_model` / `.dataModel`（根数据模型，Java getDataModel）
    DataModel,
    /// `.vars`（变量查找哈希，Java VarsHash :330-337 —— get 走完整变量解析链）
    Vars,
    /// `.lang`（locale 语言，Java getLocale().getLanguage()）
    Lang,
    /// `.locale`（locale 字符串，Java getLocale().toString()）
    Locale,
    /// `.locale_object` / `.localeObject`（Java 特有能力 —— ObjectWrapper.wrap(Locale)）
    LocaleObject,
    /// `.time_zone` / `.timeZone`（时区 ID，Java getTimeZone().getID()）
    TimeZone,
    /// `.template_name` / `.templateName`（主模板名，Java getTemplate230().getName()）
    TemplateName,
    /// `.main_template_name` / `.mainTemplateName`
    MainTemplateName,
    /// `.current_template_name` / `.currentTemplateName`（当前执行模板名）
    CurrentTemplateName,
    /// `.node` / `.current_node` / `.currentNode`（XML 节点 —— Java 特有，v1 无节点模型）
    Node,
    /// `.error`（最近一次 attempt/recover 捕获的错误消息，Java getCurrentRecoveredErrorMessage）
    Error,
    /// `.output_encoding` / `.outputEncoding`（输出编码；未设置时为 null）
    OutputEncoding,
    /// `.output_format` / `.outputFormat`（输出格式名，Java OutputFormat.getName()）
    OutputFormat,
    /// `.auto_esc` / `.autoEsc`（自动转义开关）
    AutoEsc,
    /// `.url_escaping_charset` / `.urlEscapingCharset`
    UrlEscapingCharset,
    /// `.version`（引擎版本号，Java Configuration.getVersionNumber()）
    Version,
    /// `.incompatible_improvements` / `.incompatibleImprovements`
    IncompatibleImprovements,
    /// `.args`（宏/函数参数哈希，仅宏内合法）
    Args,
    /// `.get_optional_template` / `.getOptionalTemplate`（Java GetOptionalTemplateMethod：
    /// 方法模型，调用返回 {exists/include/import} 哈希；Java 为两个独立名称
    /// （BuiltinVariable.java:258-262），错误消息用各自方法名——Rust 侧拆两个变体）
    GetOptionalTemplate,
    /// `.getOptionalTemplate`（camelCase 别名；错误消息用 ".getOptionalTemplate"）
    GetOptionalTemplateCc,
}

// ---------------------------------------------------------------------------
// TemplateElement
// ---------------------------------------------------------------------------
