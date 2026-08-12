//! 插值 —— 对应 Java `freemarker.core.DollarVariable` / `NumericalOutput`
//! （DollarVariable.accept：求值 → 转义 → 输出；NumericalOutput.accept：
//! 旧式 `#{...}` 数值插值——NumberFormat(min, max) 输出，不经过 `<#escape>` 栈）

use crate::core::eval::eval;
use crate::core::exec::ExecOutcome;
use crate::core::{Expr, OutputFormatKind};
use crate::error::{Result, TemplateError};
use crate::value::TNumber;

/// `${expr}` 插值（对应 DollarVariable.java；解析器经
/// `ElementKind::Interpolation` 的 `${...}` 分支承载）
pub struct DollarVariable {
    pub expr: Expr,
}

impl DollarVariable {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(expr: Expr) -> Self {
        DollarVariable { expr }
    }

    /// 执行（Java accept → DollarVariable.calculateInterpolatedStringOrMarkup）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        // Java DollarVariable：求值 → 转义 → 输出字符串 → 写出
        // （P4：转义表达式接收插值模型而非字符串——嵌套 escape 组合与
        // `<#escape x as h[x]>` 数字索引需要原值，见 environment.rs apply_escape）
        let m = eval(env, &self.expr)?;
        if m.is_nothing() {
            // Java DollarVariable.accept → EvalUtil.coerceModelToTextualCommon →
            // coerceModelToTextualCommon（EvalUtil.java:486-489）：classic 兼容
            // 模式回退空串；strict → InvalidReferenceException.getInstance(blamed,
            // env)——blamed = 整个插值表达式（位置 = 表达式起始），且 blamed 为
            // Dot/DynamicKeyName 时附 "It's the step after the last dot..." Tip
            // （InvalidReferenceException.java:110-158，jar 实测 missing_var_nested）
            if env.settings.classic_compatible {
                return Ok(ExecOutcome::Done);
            }
            let mut e = TemplateError::invalid_reference_at(
                crate::core::environment::expr_desc(&self.expr),
                self.expr.span,
            );
            if let TemplateError::InvalidReference { ctx, .. } = &mut e {
                if ctx.template_name.is_none() {
                    ctx.template_name = Some(env.current_template_name.clone());
                }
            }
            if matches!(
                self.expr.kind,
                crate::core::ExprKind::Dot { .. } | crate::core::ExprKind::DynKey { .. }
            ) {
                e = e.with_dot_tip();
            }
            return Err(e);
        }
        // Java DollarVariable.calculateInterpolatedStringOrMarkup：
        // 内容类型错误 blame 插值表达式——`For "${...}" content: Expected a
        // string or something automatically convertible to string (number, date
        // or boolean), or "template output" , but this has evaluated to a {type}:
        // ==> {expr}`（位置 = 表达式起始）
        let s = env
            .apply_escape(&m)
            .map_err(|e| blame_interpolation_content(e, env, &self.expr))?;
        env.emit(&s)?;
        Ok(ExecOutcome::Done)
    }
}

/// `#{expr[ ; mNMN]}` 旧式数值插值（对应 NumericalOutput.java；解析器经
/// `ElementKind::Interpolation` 的 legacy 分支承载）
pub struct NumericalOutput {
    pub expr: Expr,
    pub min_frac: u32,
    pub max_frac: u32,
}

impl NumericalOutput {
    /// 构造（Java 构造器；Rust 侧由解析器产生；无格式串 → (0, 50)）
    pub fn new(expr: Expr, min_frac: u32, max_frac: u32) -> Self {
        NumericalOutput {
            expr,
            min_frac,
            max_frac,
        }
    }

    /// 执行（Java accept → NumberFormat(min, max, grouping=false) 输出；
    /// 不经过 `<#escape>` 栈，仅 autoesc/outputFormat 转义）
    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        let n = eval(env, &self.expr)?.get_number()?;
        let s = legacy_number_format(&n, self.min_frac, self.max_frac, &env.settings.locale);
        env.emit(&legacy_auto_escaped(env, &s))?;
        Ok(ExecOutcome::Done)
    }
}

fn legacy_number_format(n: &TNumber, min_frac: u32, max_frac: u32, locale: &str) -> String {
    use bigdecimal::RoundingMode;
    match n {
        TNumber::Float(f) if f.is_nan() || f.is_infinite() => {
            // Java NumberFormat.format：NaN → "NaN"，±∞ → "∞"
            return if f.is_nan() {
                "NaN".to_string()
            } else {
                "\u{221E}".to_string()
            };
        }
        TNumber::Double(d) if d.is_nan() || d.is_infinite() => {
            return if d.is_nan() {
                "NaN".to_string()
            } else {
                "\u{221E}".to_string()
            };
        }
        _ => {}
    }
    let rounded = n
        .as_big_decimal()
        .with_scale_round(max_frac as i64, RoundingMode::HalfEven);
    let mut s = rounded.to_plain_string();
    // 剥除超出 min_frac 的尾部零（Java NumberFormat 不输出多余尾零；
    // with_scale_round 已保证小数位 = max_frac ≥ min_frac，只需剥零）
    if let Some(dot) = s.find('.') {
        let mut frac_end = s.len();
        while frac_end - dot - 1 > min_frac as usize && s.as_bytes()[frac_end - 1] == b'0' {
            frac_end -= 1;
        }
        if frac_end - dot - 1 == 0 {
            s.truncate(dot); // 小数全零 → 去掉小数点（"2.00" → "2"）
        } else {
            s.truncate(frac_end);
        }
    }
    // Locale 感知的小数点替换：fr_FR 等欧洲 locale 用 ',' 作小数点
    // （Java NumberFormat.getNumberInstance(locale) 的行为；
    //  与 format.rs decimal_separator/group_separator 一致）
    let dec_sep = match locale.split('_').next().unwrap_or("en") {
        "fr" | "de" | "es" | "tr" | "it" | "pt" | "nl" | "sv" | "cs" | "pl" | "hu" | "ro"
        | "ru" | "uk" | "bg" | "el" | "fi" | "da" | "no" | "sk" | "sl" | "hr" | "lt" | "lv"
        | "et" | "id" | "vi" | "th" => ',',
        _ => '.',
    };
    if dec_sep != '.' {
        s = s.replace('.', &dec_sep.to_string());
    }
    s
}

fn legacy_auto_escaped(env: &crate::core::Environment, s: &str) -> String {
    if !env.is_auto_escape() {
        return s.to_string();
    }
    match env.settings.output_format {
        OutputFormatKind::Html | OutputFormatKind::XHtml => {
            crate::template::utility::html_escape(s)
        }
        OutputFormatKind::Xml => crate::template::utility::xml_escape(s),
        _ => s.to_string(),
    }
}

fn blame_interpolation_content(
    e: TemplateError,
    env: &mut crate::core::Environment,
    expr: &crate::core::Expr,
) -> TemplateError {
    if let TemplateError::TypeMismatch { ctx, .. } = &e {
        if ctx.blamer.is_none() {
            return e
                .with_expected_phrase(
                    "a string or something automatically convertible to string (number, date or boolean), or \"template output\" ",
                )
                .with_blame_at(
                    "${...}",
                    "content",
                    &crate::core::environment::expr_desc(expr),
                    &env.current_template_name,
                    expr.span,
                );
        }
    }
    crate::core::environment::attach_location(e, &env.current_template_name, expr.span)
}
