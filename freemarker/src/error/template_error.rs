//! 模板错误 —— 对应 Java `freemarker.template.TemplateException`
//! （错误层级与消息逐字对齐见 docs/09；消息结构经 freemarker-2.3.34 jar 实测）

use crate::error::error_ctx::render_ftl_stack_section;
use crate::error::{ErrorCtx, FlowKind};
use crate::span::Span;

pub type Result<T> = std::result::Result<T, TemplateError>;

/// 错误分类（对应 Java 异常层级；消息逐字对齐见 docs/09 §2）
#[derive(Debug)]
pub enum TemplateError {
    /// 变量缺失（Java InvalidReferenceException）：
    /// `The following has evaluated to null or missing:\n==> {name}  [in template ...]` + Tip 段
    InvalidReference { name: String, ctx: ErrorCtx },
    /// 类型不匹配（UnexpectedTypeException 族）：
    /// `[{For "{op}" {role}: }Expected {expected}, but this has evaluated to a {actual}{:|.}]`
    /// + `\n==> {blamed}  [in template ...]`（有 blame 时）
    TypeMismatch {
        expected: &'static str,
        actual: String,
        ctx: ErrorCtx,
    },
    /// 通用运行时错误（_MiscTemplateException）
    Misc { message: String },
    /// 解析错误（ParseException）：`Syntax error in template "{name}" in line L,
    /// column C:\n{details}`（Java 2.3.34 ParseException.getMessage 格式，jar 实测）
    Parse {
        template: String,
        line: u32,
        col: u32,
        message: String,
    },
    /// stop 指令（StopException；消息即 `<#stop "msg">` 的 msg，无位置）
    Stop { message: Option<String> },
    /// break/continue 流控信号（内部传播，不面向用户）
    Flow(FlowKind),
    /// 模板加载失败（TemplateNotFoundException）
    NotFound { name: String },
    /// I/O 错误
    Io(std::io::Error),
    /// 模板模型层错误（TemplateModelException）
    Model { message: String },
}

impl TemplateError {
    /// 附加指令栈（`----\nFTL stack trace ...` 段；渲染层 attach 时调用，
    /// 只附加一次——消息已含 "FTL stack trace" 则跳过）
    pub fn with_stack(mut self, stack: Vec<crate::error::StackFrame>) -> Self {
        let section = render_ftl_stack_section(&stack);
        let Some(section) = section else {
            return self;
        };
        match &mut self {
            TemplateError::InvalidReference { ctx, .. }
            | TemplateError::TypeMismatch { ctx, .. } => {
                if ctx.instruction_stack.is_empty() {
                    ctx.instruction_stack = stack;
                }
            }
            TemplateError::Misc { message }
            | TemplateError::Parse { message, .. }
            | TemplateError::Model { message } => {
                if !message.contains("FTL stack trace") {
                    message.push_str(&section);
                }
            }
            TemplateError::Stop { message } => {
                let msg = message.get_or_insert_with(String::new);
                if !msg.contains("FTL stack trace") {
                    // Java StopException 无消息 → 消息体 "[No error description was
                    // available.]"（含栈时同样前置该文本，jar 实测 stop_plain）
                    if msg.is_empty() {
                        msg.push_str("[No error description was available.]");
                    }
                    msg.push_str(&section);
                }
            }
            TemplateError::NotFound { .. } => {}
            TemplateError::Flow(_) | TemplateError::Io(_) => {}
        }
        self
    }

    pub fn invalid_reference(name: impl Into<String>) -> Self {
        TemplateError::InvalidReference {
            name: name.into(),
            ctx: ErrorCtx::default(),
        }
    }

    /// 带 blame 表达式位置的变量缺失（Java `InvalidReferenceException.getInstance(blamed, env)`；
    /// 渲染层未提供位置时以元素位置回退）
    pub fn invalid_reference_at(name: impl Into<String>, span: Span) -> Self {
        TemplateError::InvalidReference {
            name: name.into(),
            ctx: ErrorCtx {
                span,
                ..ErrorCtx::default()
            },
        }
    }

    /// 附加点链缺失 Tip（Java Dot._eval 的 `newInvalidReferenceException`：
    /// "It's the step after the last dot that caused this error, not those before it."）
    pub fn with_dot_tip(mut self) -> Self {
        if let TemplateError::InvalidReference { ctx, .. } = &mut self {
            ctx.extra_tip = Some(
                "It's the step after the last dot that caused this error, not those before it."
                    .to_string(),
            );
        }
        self
    }

    pub fn type_mismatch(expected: &'static str, actual: impl Into<String>) -> Self {
        TemplateError::TypeMismatch {
            expected,
            actual: actual.into(),
            ctx: ErrorCtx::default(),
        }
    }

    /// 带 blame 表达式位置的类型不匹配
    pub fn type_mismatch_at(
        expected: &'static str,
        actual: impl Into<String>,
        span: Span,
    ) -> Self {
        TemplateError::TypeMismatch {
            expected,
            actual: actual.into(),
            ctx: ErrorCtx {
                span,
                ..ErrorCtx::default()
            },
        }
    }

    /// 附加 blamer 前缀与 blame 表达式（Java `_ErrorDescriptionBuilder.blame(blamed)` 
    /// + `showBlamer(true)` 的 `For "{nodeTypeSymbol}" {role}: ` 段与 `==> {expr}` 行）
    pub fn with_blame(mut self, node_type_symbol: &str, role: &str, blamed_expr: &str) -> Self {
        if let TemplateError::TypeMismatch { ctx, .. } = &mut self {
            ctx.blamer = Some(format!("For \"{node_type_symbol}\" {role}: "));
            ctx.blamed_expr = Some(blamed_expr.to_string());
        }
        self
    }

    /// 附加 blamer + blame 表达式 + 位置（Java blame(blamed) 的 blamed.getStartLocation；
    /// eval 各操作数/内建错误构造用——`==> {blamed}  [in template ... at line L, column C]`）
    pub fn with_blame_at(
        mut self,
        node_type_symbol: &str,
        role: &str,
        blamed_expr: &str,
        template_name: &str,
        span: Span,
    ) -> Self {
        if let TemplateError::TypeMismatch { ctx, .. } = &mut self {
            ctx.blamer = Some(format!("For \"{node_type_symbol}\" {role}: "));
            ctx.blamed_expr = Some(blamed_expr.to_string());
            ctx.span = span;
            ctx.template_name = Some(template_name.to_string());
        }
        self
    }

    /// 附加赋值目标变量（Java `UnexpectedTypeException(blamedAssignmentTargetVarName, ...)`；
    /// 与 blame 表达式互斥——消息以 "assignment target variable \"x\"" 代替 "this"，
    /// 且结尾用 `.` 而非 `:`）
    pub fn with_assignment_target(mut self, target: &str) -> Self {
        if let TemplateError::TypeMismatch { ctx, .. } = &mut self {
            ctx.assignment_target = Some(format!("\"{target}\""));
        }
        self
    }

    /// 附加 blame 位置（模板名 + 表达式起始行列；Java `Expression.getStartLocation` +
    /// `_MessageUtil.formatLocationForEvaluationError`。渲染层 eval 包装/元素级
    /// attach_location 调用；已带位置（行 > 0）或模板名时不覆盖——内层失败表达式
    /// 的位置优先（如 `user.name` 中 `user` 的位置））
    pub fn with_location(mut self, template_name: &str, span: Span) -> Self {
        match &mut self {
            TemplateError::InvalidReference { ctx, .. }
            | TemplateError::TypeMismatch { ctx, .. } => {
                if ctx.span.line == 0 {
                    ctx.span = span;
                }
                if ctx.template_name.is_none() {
                    ctx.template_name = Some(template_name.to_string());
                }
            }
            _ => {}
        }
        self
    }

    /// 是否已带 blame 位置（Java 异常构造时 blamed 表达式的开始位置；渲染层
    /// attach_location 检测到后不再重复附加）
    pub fn has_location(&self) -> bool {
        match self {
            TemplateError::InvalidReference { ctx, .. }
            | TemplateError::TypeMismatch { ctx, .. } => {
                ctx.span.line > 0 || ctx.template_name.is_some()
            }
            _ => false,
        }
    }

    /// 覆盖期望类型描述（Java `unexpectedTypeErrorDescription` 的 expectedTypesDesc 的
    /// a/an 形式；默认按 expected 键映射，需要特定措辞的调用点覆盖）
    pub fn with_expected_phrase(mut self, phrase: &str) -> Self {
        if let TemplateError::TypeMismatch { ctx, .. } = &mut self {
            ctx.expected_phrase = Some(phrase.to_string());
        }
        self
    }

    pub fn misc(message: impl Into<String>) -> Self {
        TemplateError::Misc {
            message: message.into(),
        }
    }

    /// 生成与 Java 版对齐的错误消息文本。
    /// 结构（TemplateException.getMessage()，jar 实测）：
    /// `{description}{tips}{FTL stack trace 段}`；
    /// description 含 blame 表达式与其位置（如 `==> missing  [in template ...]`）。
    pub fn to_user_message(&self) -> String {
        match self {
            TemplateError::InvalidReference { name, ctx } => {
                let mut s = format!(
                    "The following has evaluated to null or missing:\n==> {name}{}",
                    ctx.blamed_location()
                );
                s.push_str(&render_tips(
                    ctx.extra_tip
                        .iter()
                        .map(|t| t.as_str())
                        .chain([INVALID_REFERENCE_TIP]),
                ));
                if let Some(sec) = render_ftl_stack_section(&ctx.instruction_stack) {
                    s.push_str(&sec);
                }
                s
            }
            TemplateError::TypeMismatch {
                expected,
                actual,
                ctx,
            } => {
                let mut s = String::new();
                if let Some(b) = &ctx.blamer {
                    s.push_str(b);
                }
                let expected_phrase = ctx
                    .expected_phrase
                    .clone()
                    .unwrap_or_else(|| expected_phrase_for(expected));
                s.push_str(&format!("Expected {expected_phrase}, but "));
                match &ctx.assignment_target {
                    Some(t) => s.push_str(&format!("assignment target variable {t} has evaluated to ")),
                    None => s.push_str("this has evaluated to "),
                }
                s.push_str(&a_or_an(actual));
                if ctx.blamed_expr.is_some() {
                    s.push(':');
                } else {
                    s.push('.');
                }
                if let Some(blamed) = &ctx.blamed_expr {
                    s.push_str(&format!("\n==> {blamed}{}", ctx.blamed_location()));
                }
                if let Some(sec) = render_ftl_stack_section(&ctx.instruction_stack) {
                    s.push_str(&sec);
                }
                s
            }
            TemplateError::Misc { message } => message.clone(),
            // Java ParseException.getMessage()：`Syntax error in template "{t}" in line
            // L, column C:\n{details}`（jar 实测全部 parse_* 基线）
            TemplateError::Parse {
                template,
                line,
                col,
                message,
            } => format!(
                "Syntax error in template \"{template}\" in line {line}, column {col}:\n{message}"
            ),
            TemplateError::Stop { message } => match message {
                Some(m) => m.clone(),
                // Java StopException 无消息 → "[No error description was available.]"
                None => "[No error description was available.]".to_string(),
            },
            TemplateError::Flow(kind) => match kind {
                FlowKind::Break => "break is illegal outside a loop".to_string(),
                FlowKind::Continue => "continue is illegal outside a loop".to_string(),
            },
            TemplateError::NotFound { name } => {
                format!("Template not found for name \"{name}\".")
            }
            TemplateError::Io(e) => e.to_string(),
            TemplateError::Model { message } => message.clone(),
        }
    }
}

/// Java InvalidReferenceException 的提示段（InvalidReferenceException.java，
/// `Tip:` 字面，jar 实测逐字）
pub(crate) const INVALID_REFERENCE_TIP: &str = "If the failing expression is known to legally refer to something that's sometimes null or missing, either specify a default value like myOptionalVar!myDefault, or use <#if myOptionalVar??>when-present<#else>when-missing</#if>. (These only cover the last step of the expression; to cover the whole expression, use parenthesis: (myOptionalVar.foo)!myDefault, (myOptionalVar.foo)??";

/// 期望类型描述的 a/an 形式（Java `unexpectedTypeErrorDescription` 的 expectedTypesDesc；
/// 各调用点措辞见 UnexpectedTypeException 子类与 EvalUtil）
fn expected_phrase_for(expected: &'static str) -> String {
    match expected {
        "number" => "a number".to_string(),
        "boolean" => "a boolean".to_string(),
        "hash" => "a hash".to_string(),
        "sequence" => "a sequence".to_string(),
        "string" => "a string".to_string(),
        // Java EvalUtil.coerceModelToStringOrMarkup 的 expectedTypesDesc
        // （upper_case 等字符串类内建；插值内容在 exec.rs 覆盖为含 "template output" 变体）
        "string-like value" => {
            "a string or something automatically convertible to string (number, date or boolean)"
                .to_string()
        }
        "method" => "a method".to_string(),
        "transform" => "a transform".to_string(),
        other => format!("a {other}"),
    }
}

/// `_DelayedAOrAn`：按首字母元音判定 a/an
fn a_or_an(type_name: &str) -> String {
    let first = type_name.chars().next().unwrap_or('x');
    if "aeiouAEIOU".contains(first) {
        format!("an {type_name}")
    } else {
        format!("a {type_name}")
    }
}

/// Tips 段（Java `_ErrorDescriptionBuilder.toString` :134-164）：
/// `\n\n` + 每个 Tip：`----\nTip: {tip}`，Tip 间换行，末尾 `\n----`
fn render_tips<'a>(tips: impl IntoIterator<Item = &'a str>) -> String {
    let mut s = String::new();
    let mut first = true;
    for tip in tips {
        if first {
            s.push_str("\n\n");
            first = false;
        } else {
            s.push('\n');
        }
        s.push_str("----\nTip: ");
        s.push_str(tip);
    }
    if !first {
        s.push_str("\n----");
    }
    s
}

impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_user_message())
    }
}

impl std::error::Error for TemplateError {}

impl From<std::io::Error> for TemplateError {
    fn from(e: std::io::Error) -> Self {
        TemplateError::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_reference_message_matches_java() {
        // 对照 Java InvalidReferenceException 消息格式（描述 + Tip 段，jar 实测）
        let mut e = TemplateError::invalid_reference_at(
            "user.name",
            Span::new(1, 3),
        );
        if let TemplateError::InvalidReference { ctx, .. } = &mut e {
            ctx.template_name = Some("t.ftl".to_string());
        }
        let msg = e.to_user_message();
        assert!(
            msg.starts_with("The following has evaluated to null or missing:\n==> user.name  [in template"),
            "{msg}"
        );
        assert!(
            msg.contains("\n\n----\nTip: If the failing expression is known to legally refer"),
            "{msg}"
        );
        assert!(
            msg.ends_with("(myOptionalVar.foo)!myDefault, (myOptionalVar.foo)??\n----"),
            "{msg}"
        );
    }

    #[test]
    fn type_mismatch_blame_matches_java() {
        // jar 实测 `${n - s}`（s 为字符串）：
        // For "-" right-hand operand: Expected a number, but this has evaluated to a string:
        // ==> s  [in template "t.ftl" at line 1, column 7]
        let mut e = TemplateError::type_mismatch_at("number", "string", Span::new(1, 7))
            .with_blame("-", "right-hand operand", "s");
        if let TemplateError::TypeMismatch { ctx, .. } = &mut e {
            ctx.template_name = Some("t.ftl".to_string());
        }
        assert_eq!(
            e.to_user_message(),
            "For \"-\" right-hand operand: Expected a number, but this has evaluated to a string:\n==> s  [in template \"t.ftl\" at line 1, column 7]"
        );
    }

    #[test]
    fn type_mismatch_assignment_target() {
        let e = TemplateError::type_mismatch("number", "string").with_assignment_target("x");
        // 无 blame → 无位置
        assert!(matches!(&e, TemplateError::TypeMismatch { ctx, .. } if ctx.blamed_expr.is_none()));
        assert_eq!(
            e.to_user_message(),
            "Expected a number, but assignment target variable \"x\" has evaluated to a string."
        );
    }

    #[test]
    fn flow_kind_display() {
        assert_eq!(
            TemplateError::Flow(FlowKind::Break).to_user_message(),
            "break is illegal outside a loop"
        );
        assert_eq!(
            TemplateError::Flow(FlowKind::Continue).to_user_message(),
            "continue is illegal outside a loop"
        );
    }
}
