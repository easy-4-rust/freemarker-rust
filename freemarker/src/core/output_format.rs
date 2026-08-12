//! 输出格式 —— 对应 Java `freemarker.core.OutputFormat` 家族
//! （完整转义语义见 docs/08 §1；v1 为枚举子集，E 智能体扩展转义规则）

use crate::template::utility::{html_escape, xml_escape};

/// 自动转义模式（对应 autoEscaping 设置：on/off/default）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoEscaping {
    On,
    Off,
    /// 默认行为（随 outputFormat 与 incompatibleImprovements）
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormatKind {
    PlainText,
    Html,
    Xml,
    XHtml,
    JavaScript,
    Json,
    Css,
    Rtf,
}

impl OutputFormatKind {
    pub fn name(&self) -> &'static str {
        match self {
            OutputFormatKind::PlainText => "plainText",
            OutputFormatKind::Html => "HTML",
            OutputFormatKind::Xml => "XML",
            OutputFormatKind::XHtml => "XHTML",
            OutputFormatKind::JavaScript => "JavaScript",
            OutputFormatKind::Json => "JSON",
            OutputFormatKind::Css => "CSS",
            OutputFormatKind::Rtf => "RTF",
        }
    }

    pub fn parse(name: &str) -> Option<OutputFormatKind> {
        match name {
            "plainText" | "plaintext" => Some(OutputFormatKind::PlainText),
            "HTML" | "html" => Some(OutputFormatKind::Html),
            "XML" | "xml" => Some(OutputFormatKind::Xml),
            "XHTML" | "xhtml" => Some(OutputFormatKind::XHtml),
            "JavaScript" | "javascript" | "JS" => Some(OutputFormatKind::JavaScript),
            "JSON" | "json" => Some(OutputFormatKind::Json),
            "CSS" | "css" => Some(OutputFormatKind::Css),
            "RTF" | "rtf" => Some(OutputFormatKind::Rtf),
            _ => None,
        }
    }

    pub fn is_markup(&self) -> bool {
        !matches!(
            self,
            OutputFormatKind::PlainText | OutputFormatKind::Json | OutputFormatKind::JavaScript
        )
    }
}

/// 按输出格式转义纯文本 —— 对应 Java 各 `MarkupOutputFormat.escapePlainText`
/// （CommonMarkupOutputFormat.java:110-118；HTML/XHTML → `StringUtil.HTMLEnc`，
/// XML → `StringUtil.XMLEnc`，RTF → `StringUtil.RTFEnc`；CSS/JS/JSON/plainText
/// 无转义）。供 `CombinedMarkupOutputFormat.escapePlainText` 逐层应用
/// （CombinedMarkupOutputFormat.java:78-80）与自动转义共用。
pub fn escape_markup(kind: OutputFormatKind, s: &str) -> String {
    match kind {
        OutputFormatKind::Html | OutputFormatKind::XHtml => html_escape(s),
        OutputFormatKind::Xml => xml_escape(s),
        OutputFormatKind::Rtf => rtf_escape(s),
        _ => s.to_string(),
    }
}

/// RTF 转义 —— 对应 Java `StringUtil.RTFEnc`（StringUtil.java:276-314：
/// 仅转义 `\`、`{`、`}`，各前置一个 `\`；`RTFEnc("{a\b&}")` → `\{a\\b&\}`）。
/// jar 2.3.34 实测对齐（三层 RTF 组合：`RTF^3("b{}")` = 7×`\`+`{`+7×`\`+`}`）。
pub fn rtf_escape(s: &str) -> String {
    if !s.contains(['\\', '{', '}']) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if c == '\\' || c == '{' || c == '}' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// MIME 类型 —— 对应 Java 各 `OutputFormat.getMimeType`（HTML "text/html"、
/// XHTML "application/xhtml+xml"、XML "application/xml"、RTF "application/rtf"、
/// CSS "text/css"、JavaScript "application/javascript"、JSON "application/json"、
/// plainText "text/plain"；javadoc 行号见各 OutputFormat 头注释）。
/// 组合格式取最外层（CombinedMarkupOutputFormat.java:63-65）。
pub fn mime_type(kind: OutputFormatKind) -> &'static str {
    match kind {
        OutputFormatKind::PlainText => "text/plain",
        OutputFormatKind::Html => "text/html",
        OutputFormatKind::XHtml => "application/xhtml+xml",
        OutputFormatKind::Xml => "application/xml",
        OutputFormatKind::Rtf => "application/rtf",
        OutputFormatKind::Css => "text/css",
        OutputFormatKind::JavaScript => "application/javascript",
        OutputFormatKind::Json => "application/json",
    }
}

/// 组合标记格式名解析 —— 对应 Java `Configuration.getOutputFormat(String)`
/// （Configuration.java:2351-2398）：`outer{inner}` 递归语法（如 `HTML{RTF}`、
/// `XML{HTML{RTF}}`），`{`/`}` 语法错误或非标记格式成员 → None。
/// 返回组件列表（components[0] = 最外层）。
///
/// 注：任务描述中的 "A+B" 写法在 Java 2.3.34 源码中不存在（`+` 仅用于
/// `setRegisteredCustomOutputFormats` 的名称禁用检查，Configuration.java:2459）；
/// 按 Java 原语义实现 `{`-based 形式。
pub fn parse_combined_markup_format(name: &str) -> Option<Vec<OutputFormatKind>> {
    if name.is_empty() {
        return None;
    }
    if name.ends_with('}') {
        // Java :2355-2366：以 '}' 结尾 → 组合；缺 '{' → 语法错误
        let open = name.find('{')?;
        let outer = parse_combined_markup_format(&name[..open])?;
        let inner = parse_combined_markup_format(&name[open + 1..name.len() - 1])?;
        // Java getMarkupOutputFormatForCombined（:2400-2409）：成员必须为标记格式
        if outer.first().is_some_and(|k| k.is_markup())
            && inner.last().is_some_and(|k| k.is_markup())
        {
            let mut v = outer;
            v.extend(inner);
            Some(v)
        } else {
            None
        }
    } else {
        OutputFormatKind::parse(name).map(|k| vec![k])
    }
}
