//! 类型不匹配异常 —— 对应 Java `freemarker.core.UnexpectedTypeException` 族
//! （`[{For "{op}" {role}: }Expected {expected}, but this has evaluated to a
//! {actual}{:|.}]` + `\n==> {blamed}  [in template ...]`；NonXxx 子类只设 expected
//! 描述，见各 non_*_exception.rs）

use crate::error::{ErrorCtx, TemplateError};
use crate::span::Span;

/// Java `UnexpectedTypeException(String expectedTypesDesc, ...)` 的 Rust 入口
pub(crate) fn new_type_mismatch(
    expected: &'static str,
    actual: impl Into<String>,
) -> TemplateError {
    TemplateError::TypeMismatch {
        expected,
        actual: actual.into(),
        ctx: Box::new(ErrorCtx::default()),
    }
}

/// 带 blame 表达式位置的类型不匹配
pub(crate) fn new_type_mismatch_at(
    expected: &'static str,
    actual: impl Into<String>,
    span: Span,
) -> TemplateError {
    TemplateError::TypeMismatch {
        expected,
        actual: actual.into(),
        ctx: Box::new(ErrorCtx {
            span,
            ..ErrorCtx::default()
        }),
    }
}

/// 附加 blamer 前缀与 blame 表达式（Java `_ErrorDescriptionBuilder.blame(blamed)`
/// + `showBlamer(true)` 的 `For "{nodeTypeSymbol}" {role}: ` 段与 `==> {expr}` 行）
pub(crate) fn with_blame(
    mut e: TemplateError,
    node_type_symbol: &str,
    role: &str,
    blamed_expr: &str,
) -> TemplateError {
    if let TemplateError::TypeMismatch { ctx, .. } = &mut e {
        ctx.blamer = Some(format!("For \"{node_type_symbol}\" {role}: "));
        ctx.blamed_expr = Some(blamed_expr.to_string());
    }
    e
}

/// 附加 blamer + blame 表达式 + 位置（Java blame(blamed) 的 blamed.getStartLocation；
/// eval 各操作数/内建错误构造用——`==> {blamed}  [in template ... at line L, column C]`）
pub(crate) fn with_blame_at(
    mut e: TemplateError,
    node_type_symbol: &str,
    role: &str,
    blamed_expr: &str,
    template_name: &str,
    span: Span,
) -> TemplateError {
    if let TemplateError::TypeMismatch { ctx, .. } = &mut e {
        ctx.blamer = Some(format!("For \"{node_type_symbol}\" {role}: "));
        ctx.blamed_expr = Some(blamed_expr.to_string());
        ctx.span = span;
        ctx.template_name = Some(template_name.to_string());
    }
    e
}

/// 附加赋值目标变量（Java `UnexpectedTypeException(blamedAssignmentTargetVarName, ...)`；
/// 与 blame 表达式互斥——消息以 "assignment target variable \"x\"" 代替 "this"，
/// 且结尾用 `.` 而非 `:`）
pub(crate) fn with_assignment_target(mut e: TemplateError, target: &str) -> TemplateError {
    if let TemplateError::TypeMismatch { ctx, .. } = &mut e {
        ctx.assignment_target = Some(format!("\"{target}\""));
    }
    e
}

/// 附加 Tip（Java `_ErrorDescriptionBuilder.tip(...)`；TypeMismatch 消息的
/// `\n\n----\nTip: ...\n----` 段——数字键哈希目标 / 集合目标等场景）
pub(crate) fn with_tip(mut e: TemplateError, tip: &str) -> TemplateError {
    if let TemplateError::TypeMismatch { ctx, .. } = &mut e {
        ctx.extra_tip = Some(tip.to_string());
    }
    e
}

/// 覆盖期望类型描述（Java `unexpectedTypeErrorDescription` 的 expectedTypesDesc 的
/// a/an 形式；默认按 expected 键映射，需要特定措辞的调用点覆盖）
pub(crate) fn with_expected_phrase(mut e: TemplateError, phrase: &str) -> TemplateError {
    if let TemplateError::TypeMismatch { ctx, .. } = &mut e {
        ctx.expected_phrase = Some(phrase.to_string());
    }
    e
}

/// 期望类型描述的 a/an 形式（Java `unexpectedTypeErrorDescription` 的 expectedTypesDesc；
/// 各调用点措辞见 UnexpectedTypeException 子类与 EvalUtil）
pub(crate) fn expected_phrase_for(expected: &'static str) -> String {
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
pub(crate) fn a_or_an(type_name: &str) -> String {
    let first = type_name.chars().next().unwrap_or('x');
    if "aeiouAEIOU".contains(first) {
        format!("an {type_name}")
    } else {
        format!("a {type_name}")
    }
}
