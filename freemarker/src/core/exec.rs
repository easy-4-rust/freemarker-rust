//! 指令执行 —— 对应 Java `freemarker.core.TemplateElement` 家族各子类的 `accept(Environment)` 方法
//! （集中式入口，docs/04 §4）。各 `ElementKind` variant → Java 类映射：
//! - Text → TextBlock.java:65；Interpolation → DollarVariable
//! - If → IfBlock.java:43 / ConditionalBlock；List → IteratorBlock.java（:98 acceptWithResult）
//! - Assign/Global/Local → Assignment.java:80 `accept` / AssignmentInstruction / BlockAssignment
//! - Macro → Macro.java:154 `accept`（→ Environment.visitMacroDef :1164）
//! - Call → UnifiedCall.java:66 `accept`；Nested → BodyInstruction.java:58
//! - Switch → SwitchBlock.java:36；Attempt → AttemptBlock（→ visitAttemptRecover :3542）
//! - Break/Continue → BreakInstruction/ContinueInstruction（BreakOrContinueException）
//! - Return → ReturnInstruction.java:35；Stop → StopInstruction（StopException）
//! - Include → Include.java:25；Import → LibraryLoad.java:26；Flush → FlushInstruction
//! - Trim → TrimInstruction；Compress → CompressedBlock（StandardCompress）
//! - Escape/NoEscape → EscapeBlock/NoEscapeBlock；AutoEsc/NoAutoEsc → AutoEscBlock/NoAutoEscBlock
//! - OutputFormat → OutputFormatBlock；Setting → PropertySetting.java:136
//! - Comment → Comment；FtlHeader/RawText/TrimLineStart/NoTrimLineStart → 解析期语义

use crate::core::environment::{expr_desc, model_to_string, RunSignal};
use crate::core::eval;
use crate::core::{Element, ElementKind, OutputFormatKind};
use crate::error::{FlowKind, Result, TemplateError};
use crate::template::TModel;

/// exec 结果 —— 对应 Java `TemplateElement.accept(Environment)` 的返回值
/// （Java 返回 TemplateElement[]，null 表示无后续；Rust 用变体表达流控信号，docs/04 §2）。
pub enum ExecOutcome {
    /// 待执行子元素（Java accept 返回 TemplateElement[]）
    Next(Vec<Element>),
    /// 替换栈顶（Java replaceTopElement :414 / replaceElementStackTop :2927；v1 供回插用）
    Replace(Element),
    /// break/continue 流控信号（Java BreakOrContinueException；由 `#list` 捕获，
    /// `#switch` 内捕获并当 break 处理；循环外 → "break is illegal outside a loop"）
    Flow(FlowKind),
    /// `<#return>` 返回值（Java ReturnInstruction.Return 异常；宏/函数帧捕获）
    ReturnValue(Option<TModel>),
    /// `<#stop>` 终止消息（Java StopException；渲染终止）
    Stop(Option<String>),
    /// 完成（Java accept 返回 null）
    Done,
}

/// 指令执行 —— 对应 Java `TemplateElement.accept(Environment)`（docs/04 §4 指令全清单）
pub fn exec(env: &mut crate::core::Environment, el: &Element) -> Result<ExecOutcome> {
    match &el.kind {
        ElementKind::Text {
            text,
            strip_before,
            strip_after,
            orig_end_line,
        } => crate::core::text_block::TextBlock::new(
            text.clone(),
            *strip_before,
            *strip_after,
            *orig_end_line,
            false,
        )
        .exec(env),
        ElementKind::NoParse {
            text,
            strip_before,
            strip_after,
            orig_end_line,
        } => crate::core::text_block::TextBlock::new(
            text.clone(),
            *strip_before,
            *strip_after,
            *orig_end_line,
            true,
        )
        .exec(env),
        ElementKind::Interpolation {
            expr,
            legacy_min_frac,
            legacy_max_frac,
        } => match (legacy_min_frac, legacy_max_frac) {
            (Some(min), Some(max)) => {
                crate::core::dollar_variable::NumericalOutput::new(expr.clone(), *min, *max)
                    .exec(env)
            }
            _ => crate::core::dollar_variable::DollarVariable::new(expr.clone()).exec(env),
        },
        ElementKind::If { cond, then, else_ } => {
            crate::core::if_block::IfBlock::new(cond.clone(), then.clone(), else_.clone(), el.span)
                .exec(env)
        }
        ElementKind::List {
            seq,
            var,
            var2,
            body,
            else_,
        } => crate::core::iterator_block::IteratorBlock::new(
            seq.clone(),
            var.clone(),
            var2.clone(),
            body.clone(),
            else_.clone(),
        )
        .exec(env),
        ElementKind::Items { var, var2, body } => {
            crate::core::items::Items::new(var.clone(), var2.clone(), body.clone()).exec(env)
        }
        ElementKind::Sep { body } => crate::core::sep::Sep::new(body.clone()).exec(env),
        ElementKind::Assignments(els) => {
            crate::core::assignment_instruction::AssignmentInstruction::new(els.clone()).exec(env)
        }
        ElementKind::Assign {
            target,
            expr,
            op,
            namespace,
        } => crate::core::assignment::Assignment::new(
            target.clone(),
            expr.clone(),
            *op,
            namespace.clone(),
        )
        .exec(env),
        ElementKind::BlockAssign {
            target,
            body,
            namespace,
            ..
        } => crate::core::block_assignment::BlockAssignment::new(
            target.clone(),
            body.clone(),
            namespace.clone(),
        )
        .exec(env),
        ElementKind::Global {
            target,
            expr,
            body,
            op,
        } => crate::core::global_assignment::GlobalAssignment::new(
            target.clone(),
            expr.clone(),
            body.clone(),
            *op,
        )
        .exec(env),
        ElementKind::Local {
            target,
            expr,
            body,
            op,
        } => crate::core::local_assignment::LocalAssignment::new(
            target.clone(),
            expr.clone(),
            body.clone(),
            *op,
        )
        .exec(env),
        ElementKind::Macro { def } => crate::core::r#macro::Macro::new(def.clone()).exec(env),
        ElementKind::Call {
            callee,
            args,
            body,
            body_params,
        } => crate::core::unified_call::UnifiedCall::new(
            callee.clone(),
            args.clone(),
            body.clone(),
            body_params.clone(),
            el.span,
        )
        .exec(env),
        ElementKind::Nested { args, body: _ } => {
            crate::core::body_instruction::BodyInstruction::new(args.clone()).exec(env)
        }
        ElementKind::Switch {
            expr,
            cases,
            default,
            default_pos,
        } => crate::core::switch_block::SwitchBlock::new(
            expr.clone(),
            cases.clone(),
            default.clone(),
            *default_pos,
        )
        .exec(env),
        ElementKind::Attempt { try_, recover } => {
            crate::core::attempt_block::AttemptBlock::new(try_.clone(), recover.clone()).exec(env)
        }
        ElementKind::Break => crate::core::break_instruction::BreakInstruction::new().exec(env),
        ElementKind::Continue => {
            crate::core::continue_instruction::ContinueInstruction::new().exec(env)
        }
        ElementKind::Return { expr } => {
            crate::core::return_instruction::ReturnInstruction::new(expr.clone()).exec(env)
        }
        ElementKind::Stop { msg } => {
            crate::core::stop_instruction::StopInstruction::new(msg.clone()).exec(env)
        }
        ElementKind::Flush => crate::core::flush_instruction::FlushInstruction::new().exec(env),
        ElementKind::Trim(body) => {
            crate::core::trim_instruction::TrimInstruction::new(body.clone()).exec(env)
        }
        ElementKind::Comment { text } => crate::core::comment::Comment::new(text.clone()).exec(env),
        ElementKind::Include { path, attrs } => {
            crate::core::include::Include::new(path.clone(), attrs.clone()).exec(env)
        }
        ElementKind::Import { path, ns } => {
            crate::core::library_load::LibraryLoad::new(path.clone(), ns.clone()).exec(env)
        }
        ElementKind::Escape { expr, body } => {
            crate::core::escape_block::EscapeBlock::new(expr.clone(), body.clone()).exec(env)
        }
        ElementKind::NoEscape(body) => {
            crate::core::no_escape_block::NoEscapeBlock::new(body.clone()).exec(env)
        }
        ElementKind::AutoEsc(body) => {
            crate::core::auto_esc_block::AutoEscBlock::new(body.clone()).exec(env)
        }
        ElementKind::NoAutoEsc(body) => {
            crate::core::no_auto_esc_block::NoAutoEscBlock::new(body.clone()).exec(env)
        }
        ElementKind::OutputFormat { name, body } => {
            crate::core::output_format_block::OutputFormatBlock::new(name.clone(), body.clone())
                .exec(env)
        }
        ElementKind::Compress(body) => {
            crate::core::compressed_block::CompressedBlock::new(body.clone()).exec(env)
        }
        ElementKind::Setting { key, value } => {
            crate::core::property_setting::Setting::new(key.clone(), value.clone()).exec(env)
        }
        ElementKind::FtlHeader { encoding } => {
            crate::core::ftl_header::FtlHeader::new(encoding.clone()).exec(env)
        }
        ElementKind::TrimLineStart
        | ElementKind::NoTrimLineStart
        | ElementKind::TrimLineEnd
        | ElementKind::LeftTrimLine => crate::core::trim_instruction::TrimMark::new().exec(env),
        ElementKind::RawText(t) => crate::core::text_block::RawText::new(t.clone()).exec(env),
        ElementKind::Transform { expr, body } => {
            crate::core::transform_block::TransformBlock::new(expr.clone(), body.clone()).exec(env)
        }
        ElementKind::Visit { expr, using } => {
            crate::core::visit_node::VisitNode::new(expr.clone(), using.clone()).exec(env)
        }
        ElementKind::Recurse { expr, using } => {
            crate::core::recurse_node::RecurseNode::new(expr.clone(), using.clone()).exec(env)
        }
        ElementKind::On { expr, body } => {
            crate::core::on::On::new(expr.clone(), body.clone()).exec(env)
        }
        ElementKind::Fallback => {
            crate::core::fallback_instruction::FallbackInstruction::new().exec(env)
        }
    }
}

/// `<#if>` 执行（elseif 链扁平化下钻；借用版：命中分支克隆返回）
/// `span`：当前 case 的源码位置（`<#elseif>` 下钻时更新为各 case 自身 span）
/// `<#if>` 条件类型错误 → Java `For "#if" condition: ... ==> {cond}` 形式
/// （NonBooleanException 的 blamer/blame/位置）
/// 所有权版指令执行 —— run_slice 的 mini 栈路径使用：命中分支/调用 body
/// 直接移动（零克隆）。非热路径 variant 委托 exec(&Element)（借用语义一致）。
pub(crate) fn exec_owned(env: &mut crate::core::Environment, el: Element) -> Result<ExecOutcome> {
    let span = el.span;
    match el.kind {
        // `<#if>`：分支 Vec 移动（零克隆）+ elseif 链下钻
        ElementKind::If { cond, then, else_ } => {
            let mut cur_span = span;
            let mut cond = cond;
            let mut then = then;
            let mut else_ = else_;
            loop {
                let cm = eval::eval(env, &cond).map_err(|e| {
                    crate::core::environment::attach_location(
                        e,
                        &env.current_template_name,
                        cur_span,
                    )
                })?;
                let b = eval::model_to_boolean(env, &cm).map_err(|e| {
                    crate::core::environment::attach_location(
                        e,
                        &env.current_template_name,
                        cur_span,
                    )
                })?;
                if b {
                    return Ok(ExecOutcome::Next(then));
                }
                match else_ {
                    Some(v) if v.len() == 1 => match v.into_iter().next().unwrap() {
                        Element {
                            kind:
                                ElementKind::If {
                                    cond: c2,
                                    then: t2,
                                    else_: e2,
                                },
                            span: s2,
                        } => {
                            cur_span = s2;
                            cond = c2;
                            then = t2;
                            else_ = e2;
                            continue;
                        }
                        e => return Ok(ExecOutcome::Next(vec![e])),
                    },
                    Some(v) => return Ok(ExecOutcome::Next(v)),
                    None => return Ok(ExecOutcome::Done),
                }
            }
        }
        // 多赋值：元素所有权逐个传递
        ElementKind::Assignments(els) => {
            for e in els {
                let outcome = exec_owned(env, e)?;
                if !matches!(outcome, ExecOutcome::Done) {
                    return Ok(outcome);
                }
            }
            Ok(ExecOutcome::Done)
        }
        // `<@...>` 调用：body/body_params 移动（避免每调用一次 to_vec 克隆）
        ElementKind::Call {
            callee,
            args,
            body,
            body_params,
        } => {
            crate::core::unified_call::exec_call_impl(env, &callee, &args, body, body_params, span)
        }
        other => exec(env, &Element { kind: other, span }),
    }
}

/// 宏调用执行（函数角色报错；宏体 run 经 invoke_macro）—— exec_call_impl 的
/// Name 快路径与常规 as_macro 路径共用
/// `<#visit>` 节点分派 —— 对应 Java `Environment.visit(TemplateNodeModel)`
/// （Environment.java:2885-2940）：按节点名查找同名宏（如 `<#macro book>` 处理
/// 元素 book），无 → `@default` 宏，再无可 → 默认行为（text/comment/PI/attr
/// 输出标量；element/document 递归 visit 子节点）。
/// 自定义指令 body 回插 —— 对应 Java `Environment.NestedElementTemplateDirectiveBody`
/// （Environment.java:3445-3475）：render(newOut) → visit(childBuffer)
/// `<#switch>` —— 对应 Java `SwitchBlock.accept`（SwitchBlock.java:36-115）：
/// 目标求值一次；逐个 case 以 `==` 语义比较（EvalUtil.compare，:66-71）；
/// 匹配后 fall-through 执行后续 case 与 default；未匹配 → default；
/// case 体内的 break/continue 被捕获并当作 break（:108-115 Java 注释确认的怪癖）。
/// `<#attempt>/<#recover>` —— 对应 Java `Environment.visitAttemptRecover`（:3542-3573）：
/// try 输出捕获；错误（非 Flow/Return——Java 中它们是 RuntimeException 不被捕获）→ 丢弃
/// 输出并执行 recover；attemptExceptionReporter v1 忽略。
/// `<#setting>` —— 对应 Java `PropertySetting.accept`（PropertySetting.java:136-155）+
/// `Configurable.setSetting`（未知键 → IllegalArgumentException → v1 报错）
/// 插值内容类型错误 → Java `For "${...}" content: ... ==> {expr}` 形式
/// （DollarVariable 的 coerceModelToStringOrMarkup blame；期望措辞含
/// `or "template output" ` 段——Java 消息中该段后紧跟逗号，jar 实测逐字）
pub(crate) fn exec_setting(
    env: &mut crate::core::Environment,
    key: &str,
    value: &crate::core::Expr,
) -> Result<ExecOutcome> {
    // Java PropertySetting.accept：标量原样、布尔 true/false、数字 toString（不经 boolean_format）
    let v = {
        let m = eval::eval(env, value)?;
        if m.is_nothing() {
            return Err(TemplateError::invalid_reference(expr_desc(value)));
        }
        if let Some(b) = &m.boolean {
            if b.as_boolean()? {
                "true".to_string()
            } else {
                "false".to_string()
            }
        } else if let Some(s) = &m.scalar {
            s.as_string()?
        } else if let Some(n) = &m.number {
            n.as_number()?.to_plain_string()
        } else {
            model_to_string(env, &m)?
        }
    };
    match key {
        "locale" => env.settings.to_mut().locale = v,
        "number_format" => env.settings.to_mut().number_format = v,
        "boolean_format" => {
            // Java Configurable.setBooleanFormat：必须含逗号或为 "c"（否则 IllegalArgumentException）
            if v != "c" && !v.contains(',') {
                return Err(TemplateError::misc(format!(
                    "Setting value must be a string that contains two comma-separated values for true and false, or it must be \"c\", but it was {v:?}."
                )));
            }
            env.settings.to_mut().boolean_format = v;
        }
        "date_format" => env.settings.to_mut().date_format = v,
        "time_format" => env.settings.to_mut().time_format = v,
        // Java 设置键为 "datetime_format"（Configurable.DATETIME_FORMAT_KEY）
        "datetime_format" => env.settings.to_mut().date_time_format = v,
        "output_encoding" => env.settings.to_mut().output_encoding = v,
        "url_escaping_charset" => env.settings.to_mut().url_escaping_charset = v,
        "time_zone" => {
            // P4：`default` → 恢复配置级时区（Java PropertySetting：null → 配置默认）；
            // GMT±HH[:mm]/IANA 名经 TzSetting::from_str（对应 Java TimeZone.getTimeZone）
            env.settings.to_mut().time_zone = if v == "default" {
                env.base_time_zone
            } else {
                v.parse()
                    .map_err(|_| TemplateError::misc(format!("Unknown time zone: {v}")))?
            };
            // Java TimeZone.getID()（`.time_zone` 读数；GMT 名归一化为 GMT±HH:MM）
            env.settings.to_mut().time_zone_id = if v == "default" {
                env.base_time_zone_id.clone()
            } else {
                crate::core::configurable::java_time_zone_id(&v)
            };
        }
        "sql_date_and_time_time_zone" => {
            // Java PropertySetting 支持（影响 SQL 日期格式化，v1 忽略 —— 文档化偏差）
        }
        "classic_compatible" => {
            env.settings.to_mut().classic_compatible = parse_bool_setting(&v)?;
        }
        "whitespace_stripping" => {
            env.settings.to_mut().whitespace_stripping = parse_bool_setting(&v)?
        }
        "strict_syntax" => env.settings.to_mut().strict_syntax = parse_bool_setting(&v)?,
        "output_format" => {
            env.settings.to_mut().output_format = OutputFormatKind::parse(&v)
                .ok_or_else(|| TemplateError::misc(format!("Unknown output format: {v}")))?;
        }
        "c_format" => {
            // Java Configurable.C_FORMAT_KEY（c_format 设置；StandardCFormats 注册名）
            env.settings.to_mut().c_format = crate::builtins::format::CFormatKind::parse(&v)
                .ok_or_else(|| TemplateError::misc(format!("Unknown c_format: {v}")))?;
        }
        "auto_escaping" => {
            env.settings.to_mut().auto_escaping = match v.as_str() {
                "on" => crate::core::AutoEscaping::On,
                "off" => crate::core::AutoEscaping::Off,
                "default" => crate::core::AutoEscaping::Default,
                other => {
                    return Err(TemplateError::misc(format!(
                        "Invalid auto_escaping value: {other}"
                    )))
                }
            };
        }
        "template_exception_handler" => {
            // Java 允许的 4 种处理器名（TemplateExceptionHandler.getDefault / setSetting 的
            // 字符串形式；jar 实测 PropertySetting 解析期即拒绝该键——v1 文档化偏差：允许
            // 模板内设置，取值受限为 Java 的 4 个内置处理器）
            env.settings.to_mut().template_exception_handler = match v.as_str() {
                "rethrow" | "debug" | "html_debug" | "ignore" => v,
                other => {
                    return Err(TemplateError::misc(format!(
                        "Invalid template_exception_handler value: {other}. It must be one of: rethrow, debug, html_debug, ignore"
                    )))
                }
            };
        }
        other => {
            // Java Configurable.setSetting 未知键：IllegalArgumentException
            return Err(TemplateError::misc(format!("Unsupported setting: {other}")));
        }
    }
    Ok(ExecOutcome::Done)
}

fn parse_bool_setting(v: &str) -> Result<bool> {
    match v {
        "true" | "yes" | "y" => Ok(true),
        "false" | "no" | "n" => Ok(false),
        other => Err(TemplateError::misc(format!(
            "Invalid boolean value: {other}"
        ))),
    }
}

/// run 结果 → ExecOutcome（宏体/捕获块等内部 run 的返回值上传）
pub(crate) fn outcome_from_run(r: Result<RunSignal>) -> Result<ExecOutcome> {
    match r {
        Ok(RunSignal::Completed) => Ok(ExecOutcome::Done),
        Ok(RunSignal::Returned(v)) => Ok(ExecOutcome::ReturnValue(v)),
        Err(e) => Err(e),
    }
}

/// 首个换行（含）之后的起始下标（Java TextBlock.openingCharsToStrip 的裁剪量）
fn first_newline_end(s: &str) -> usize {
    match s.find('\n') {
        Some(i) => i + 1,
        None => s.len(),
    }
}

/// 最后一个换行之后的起始下标（Java TextBlock.trailingCharsToStrip 的保留起点）
fn last_newline_start(s: &str) -> usize {
    match s.rfind('\n') {
        Some(i) => i + 1,
        None => s.len(),
    }
}

/// 文本裁剪（Java TextBlock.postParseCleanup 的渲染期等价；Text/NoParse 共用）。
/// 注：strip_after 的标记在**裁剪前**的文本上计算（含换行），deliberate rt/t 消费后
/// 文本可能已无换行（如 "\n  " → "  "）——无换行时整段剥除
/// （Java trailingCharsToStrip：lastNewlineIndex==-1 && beginColumn==1 → 整段，
///  TextBlock.java:294-297；全空白文本才可能带此标记）。
pub(crate) fn strip_text<'a>(
    text: &'a str,
    strip_before: bool,
    strip_after: bool,
    env: &crate::core::Environment,
) -> &'a str {
    if !env.settings.whitespace_stripping || (!strip_before && !strip_after) {
        return text;
    }
    let begin = if strip_before {
        first_newline_end(text)
    } else {
        0
    };
    let end = if strip_after {
        if text.contains('\n') {
            last_newline_start(text)
        } else {
            0 // 无换行 → 整段剥
        }
    } else {
        text.len()
    };
    &text[begin.min(end)..end]
}

/// 空白压缩 —— 对应 Java `<#compress>`（CompressedBlock.accept :40-44 →
/// StandardCompress.INSTANCE 变换）：Java 逐字符状态机（utility_transforms.rs）
pub(crate) fn get_yes_no(_exp: &crate::core::Expr, s: &str) -> Result<bool> {
    let s2 = if s.starts_with('"') && s.len() >= 2 {
        &s[1..s.len() - 1]
    } else {
        s
    };
    let lower = s2.to_ascii_lowercase();
    match lower.as_str() {
        "n" | "no" | "f" | "false" => Ok(false),
        "y" | "yes" | "t" | "true" => Ok(true),
        _ => Err(TemplateError::misc(format!(
            "Value must be boolean (or one of these strings: \"n\", \"no\", \"f\", \"false\", \"y\", \"yes\", \"t\", \"true\"), but it was \"{s}\"."
        ))),
    }
}

/// 表达式求值 → 字符串（`<#include>`/`<#import>`/`<#stop msg>`/`<#setting>` 值）
pub(crate) fn eval_to_string(
    env: &mut crate::core::Environment,
    e: &crate::core::Expr,
) -> Result<String> {
    crate::core::environment::eval_to_string(env, e)
}

#[cfg(test)]
#[path = "exec_tests.rs"]
mod exec_tests;

// ------------------------------------------------------------------
// 黄金断言：与 Java templatesuite expected/ 输出逐字节对照（docs/11 §3；
// 路径为 Java 仓库 templatesuite，expected 文件开头为 /* ... */ 许可证注释，先剥除）
// ------------------------------------------------------------------
