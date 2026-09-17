//! 指令栈快照描述 —— 元素描述、Java 快照过滤集、错误位置附加
//! （对应 Java TemplateElement.getDescription / isShownInStackTrace /
//! getInstructionStackSnapshot，docs/09 §2/§6.4）。

use super::environment_models::expr_desc;
use crate::core::{AssignOp, CallTarget, Element, ElementKind, Expr, MacroDef};
use crate::error::{ErrorCtx, TemplateError};
use crate::span::Span;

/// 元素是否在 Java 指令栈快照中显示 —— 对应各类 `isShownInStackTrace()` 覆盖
/// （jar 实测：BodyInstruction/Include/Interpolation/LibraryLoad/UnifiedCall/
/// TransformBlock/VisitNode/RecurseNode/FallbackInstruction 返回 true，其余 false；
/// 栈顶失败帧不受限——getInstructionStackSnapshot 对末帧无条件包含）
pub(crate) fn element_shown_in_stack_trace(el: &Element) -> bool {
    matches!(
        el.kind,
        ElementKind::Interpolation { .. }
            | ElementKind::Call { .. }
            | ElementKind::Include { .. }
            | ElementKind::Import { .. }
            | ElementKind::Nested { .. }
            | ElementKind::Transform { .. }
            | ElementKind::Visit { .. }
            | ElementKind::Recurse { .. }
            | ElementKind::Fallback
    )
}

/// 元素描述 —— 对应 Java `TemplateElement.getDescription()`（= `dump(false)`，元素
/// 源码形式的规范化文本；`_MessageUtil.shorten(desc, 40)` 截断，Environment.java:2622）。
/// 返回 None = 无描述（Java 快照过滤跳过 description==null 的帧；Text/Comment 等
/// 不可能失败的元素不压帧）。
pub(crate) fn describe_element(env: &super::Environment, el: &Element) -> Option<String> {
    use ElementKind as E;
    let d = match &el.kind {
        E::Text { .. }
        | E::NoParse { .. }
        | E::Comment { .. }
        | E::FtlHeader { .. }
        | E::TrimLineStart
        | E::NoTrimLineStart
        | E::TrimLineEnd
        | E::LeftTrimLine
        | E::RawText(_) => return None,
        E::Interpolation {
            expr,
            legacy_min_frac,
            ..
        } => {
            // Java DollarVariable.dump：`${expr}` / `#{expr}`；自动转义/`<#escape>` 内
            // 追加 " auto-escaped"（escapedExpression != expression）
            let s = if legacy_min_frac.is_some() {
                format!("#{{{}}}", expr_desc(expr))
            } else {
                format!("${{{}}}", expr_desc(expr))
            };
            if legacy_min_frac.is_none() && (!env.escapes.is_empty() || env.auto_escape) {
                format!("{s} auto-escaped")
            } else {
                s
            }
        }
        E::If { cond, .. } => format!("#if {}", expr_desc(cond)),
        E::List { seq, var, var2, .. } => {
            let mut s = format!("#list {}", expr_desc(seq));
            if !var.is_empty() {
                s.push_str(&format!(" as {var}"));
                if let Some(v2) = var2 {
                    s.push_str(&format!(", {v2}"));
                }
            }
            s
        }
        E::Items { var, var2, .. } => {
            let mut s = format!("#items as {var}");
            if let Some(v2) = var2 {
                s.push_str(&format!(", {v2}"));
            }
            s
        }
        E::Assign {
            target, expr, op, ..
        } => format!(
            "#assign {target} {} {}",
            assign_op_symbol(*op),
            expr_desc(expr)
        ),
        E::Global {
            target,
            expr: Some(e),
            ..
        } => format!("#global {target} = {}", expr_desc(e)),
        E::Local {
            target,
            expr: Some(e),
            ..
        } => format!("#local {target} = {}", expr_desc(e)),
        E::Macro { def } => macro_def_description(def),
        E::Call {
            callee,
            args,
            body_params,
            ..
        } => {
            // Java UnifiedCall.getDescription（dump）：`@name` + 位置参数 `1, 2`
            // （逗号空格）与命名参数 `b=1` 混合（位置在前）
            let mut s = String::from("@");
            match callee {
                CallTarget::Name(n) => s.push_str(n),
                CallTarget::Namespaced { ns, name } => {
                    s.push_str(ns);
                    s.push('.');
                    s.push_str(name);
                }
                CallTarget::Expr(e) => s.push_str(&expr_desc(e)),
            }
            let mut parts: Vec<String> = Vec::new();
            for (k, e) in args {
                if k.is_empty() {
                    parts.push(expr_desc(e));
                } else {
                    parts.push(format!("{k}={}", expr_desc(e)));
                }
            }
            if !parts.is_empty() {
                s.push(' ');
                s.push_str(&parts.join(", "));
            }
            // `; a, b` 体参数（Java `@m 1; a, b` 描述含体参数——场景未覆盖，按
            // 源码形式附加）
            if !body_params.is_empty() {
                s.push_str("; ");
                s.push_str(&body_params.join(", "));
            }
            s
        }
        E::Nested { .. } => "#nested".to_string(),
        E::Switch { expr, .. } => format!("#switch {}", expr_desc(expr)),
        E::Break => "#break".to_string(),
        E::Continue => "#continue".to_string(),
        E::Return { expr } => match expr {
            Some(e) => format!("#return {}", expr_desc(e)),
            None => "#return".to_string(),
        },
        E::Stop { msg } => match msg {
            Some(e) => format!("#stop {}", expr_desc(e)),
            None => "#stop".to_string(),
        },
        E::Flush => "#flush".to_string(),
        E::Include { path, .. } => format!("#include {}", expr_desc(path)),
        E::Import { path, ns } => format!("#import {} as {ns}", expr_desc(path)),
        E::Setting { key, value } => format!("#setting {key}={}", expr_desc(value)),
        E::Transform { expr, .. } => format!("#transform {}", expr_desc(expr)),
        E::Visit { expr, .. } => match expr {
            Some(e) => format!("#visit {}", expr_desc(e)),
            None => "#visit".to_string(),
        },
        E::Recurse { expr, .. } => match expr {
            Some(e) => format!("#recurse {}", expr_desc(e)),
            None => "#recurse".to_string(),
        },
        E::On { expr, .. } => format!("#on {}", expr_desc(expr)),
        E::Fallback => "#fallback".to_string(),
        // 容器类指令（Escape/NoEscape/AutoEsc/NoAutoEsc/OutputFormat/Compress/
        // Attempt/Trim/Assignments/BlockAssign/Sep）：Java 无描述或失败帧为内部
        // 元素——不压帧
        _ => return None,
    };
    Some(shorten_java(&d, 40))
}

/// 赋值操作符符号（Java Assignment.dump 的标签文本）
fn assign_op_symbol(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Equals => "=",
        AssignOp::PlusEq => "+=",
        AssignOp::MinusEq => "-=",
        AssignOp::TimesEq => "*=",
        AssignOp::DivideEq => "/=",
        AssignOp::ModuloEq => "%=",
        AssignOp::PlusPlus => "++",
        AssignOp::MinusMinus => "--",
    }
}

/// 宏/函数定义描述 —— 对应 Java `Macro.getDescription()`（dump）：
/// `#macro {name} {param}[={default}]...` / `#function ...`（参数空格分隔，
/// 默认值按表达式描述）
pub(crate) fn macro_def_description(def: &MacroDef) -> String {
    let mut s = format!(
        "{} {name}",
        if def.is_function {
            "#function"
        } else {
            "#macro"
        },
        name = def.name
    );
    for p in &def.params {
        if p.catch_all {
            s.push_str(&format!(
                " {}{}",
                p.name,
                if p.default.is_some() { "..." } else { "" }
            ));
            continue;
        }
        match &p.default {
            Some(e) => s.push_str(&format!(" {}={}", p.name, expr_desc(e))),
            None => s.push_str(&format!(" {}", p.name)),
        }
    }
    s
}

/// Java `_MessageUtil.shorten(s, maxLen)`：超长截断为 `前 maxLen-3 字符 + "..."`
fn shorten_java(s: &str, max_len: usize) -> String {
    let count = s.chars().count();
    if count <= max_len {
        return s.to_string();
    }
    let cut = max_len.saturating_sub(3);
    let head: String = s.chars().take(cut).collect();
    format!("{head}...")
}

/// 收集表达式中的标识符名（`<#escape x as x?html>` 的占位符识别；v1 近似
/// Java 解析期替换，见 apply_escape 注释）
pub(crate) fn collect_ident_names(e: &Expr) -> Vec<String> {
    let mut out = Vec::new();
    collect_ident_names_into(e, &mut out);
    out
}

fn collect_ident_names_into(e: &Expr, out: &mut Vec<String>) {
    use crate::core::ExprKind as K;
    match &e.kind {
        K::Ident(n) => out.push(n.clone()),
        K::InterpStr(parts) => {
            for p in parts {
                if let crate::core::StrPart::Interp(inner) = p {
                    collect_ident_names_into(inner, out);
                }
            }
        }
        K::Dot { target, .. }
        | K::UnaryMinus(target)
        | K::Not(target)
        | K::Exists(target)
        | K::Paren(target) => collect_ident_names_into(target, out),
        K::DynKey { target, key } => {
            collect_ident_names_into(target, out);
            collect_ident_names_into(key, out);
        }
        K::Default { target, default } => {
            collect_ident_names_into(target, out);
            if let Some(d) = default {
                collect_ident_names_into(d, out);
            }
        }
        K::Add(a, b)
        | K::Sub(a, b)
        | K::Mul(a, b)
        | K::Div(a, b)
        | K::Mod(a, b)
        | K::Eq(a, b)
        | K::NotEq(a, b)
        | K::Gt(a, b)
        | K::Gte(a, b)
        | K::Lt(a, b)
        | K::Lte(a, b)
        | K::And(a, b)
        | K::Or(a, b) => {
            collect_ident_names_into(a, out);
            collect_ident_names_into(b, out);
        }
        K::Range { start, end, .. } => {
            collect_ident_names_into(start, out);
            if let Some(end) = end {
                collect_ident_names_into(end, out);
            }
        }
        K::BuiltIn { target, args, .. } => {
            collect_ident_names_into(target, out);
            if let Some(args) = args {
                for a in args {
                    collect_ident_names_into(a, out);
                }
            }
        }
        K::Call { callee, args } => {
            collect_ident_names_into(callee, out);
            for a in args {
                collect_ident_names_into(a, out);
            }
        }
        K::ListLit(items) => {
            for i in items {
                collect_ident_names_into(i, out);
            }
        }
        K::HashLit(pairs) => {
            for (k, v) in pairs {
                collect_ident_names_into(k, out);
                collect_ident_names_into(v, out);
            }
        }
        K::Lambda { body, .. } => collect_ident_names_into(body, out),
        _ => {}
    }
}

/// 错误附加源码位置 —— `[in template "name" at line L, column C]`（docs/09 §2 消息模板）。
/// 只附加一次（消息已含 "[in template" 则跳过）；Flow/Stop/Parse/Io 不附加
/// （Flow 是流控信号；Stop 是 Java StopException 语义，自带消息）。
pub(crate) fn attach_location(
    err: TemplateError,
    template_name: &str,
    span: Span,
) -> TemplateError {
    // 错误已带位置（eval 包装按失败表达式位置附加）或消息已含位置 → 不重复附加
    if err.has_location() {
        return err;
    }
    match err {
        TemplateError::InvalidReference { name, ctx } => {
            if name.contains("[in template") {
                TemplateError::InvalidReference { name, ctx }
            } else {
                TemplateError::InvalidReference {
                    name,
                    ctx: Box::new(ErrorCtx {
                        span,
                        template_name: Some(template_name.to_string()),
                        ..*ctx
                    }),
                }
            }
        }
        TemplateError::TypeMismatch {
            expected,
            actual,
            ctx,
        } => {
            if actual.contains("[in template") || ctx.assignment_target.is_some() {
                // 赋值目标错误（Java UnexpectedTypeException(blamedAssignmentTargetVarName)）
                // 无 blame 表达式 → 消息不含位置（位置仅出现在 FTL stack 段）
                TemplateError::TypeMismatch {
                    expected,
                    actual,
                    ctx,
                }
            } else {
                TemplateError::TypeMismatch {
                    expected,
                    actual,
                    ctx: Box::new(ErrorCtx {
                        span,
                        template_name: Some(template_name.to_string()),
                        ..*ctx
                    }),
                }
            }
        }
        // Java _MiscTemplateException / TemplateModelException 消息不含位置
        // （位置仅由 FTL stack trace 段承载，jar 实测 div_by_zero/宏参数错误等）
        other => other,
    }
}
