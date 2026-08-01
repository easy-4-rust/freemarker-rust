//! 输出格式 —— 对应 Java `freemarker.core.OutputFormat` 家族
//! （完整转义语义见 docs/08 §1；v1 为枚举子集，E 智能体扩展转义规则）

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
