//! 组合标记输出格式 —— 对应 Java `freemarker.core.CombinedMarkupOutputFormat`
//! （外层格式套内层格式的组合：escape 逐层应用——最内层先转义，逐层向外；
//! name = `outer{inner}`；mimeType = 外层 MIME；@since 2.3.24）
//!
//! Java 只有 outer/inner 两个槽位（嵌套用递归构造），Rust 以 components 列表
//! 承载：components[0] = 最外层。转义顺序与 name 均与 Java 等价：
//! `escapePlainText` = outer.escape(inner.escape(...))（CombinedMarkupOutputFormat.java:78-80），
//! `getName` = outer.getName() + "{" + inner.getName() + "}"（:51-60）。

use crate::core::{escape_markup, mime_type, OutputFormatKind};

/// 组合标记输出格式（对应 CombinedMarkupOutputFormat.java；components[0] = 最外层）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedMarkupOutputFormat {
    pub components: Vec<OutputFormatKind>,
}

/// 组合标记输出模型（对应 Java `TemplateCombinedMarkupOutputModel`：
/// plainTextContent/markupContent 二选一；getPlainTextContent/getMarkupContent）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedMarkupOutputModel {
    pub plain_text: Option<String>,
    pub markup: Option<String>,
}

impl CombinedMarkupOutputFormat {
    /// 构造（Java `CombinedMarkupOutputFormat(MarkupOutputFormat outer,
    /// MarkupOutputFormat inner)`，:42-55；嵌套组合以平铺列表等价）
    pub fn new(components: Vec<OutputFormatKind>) -> Self {
        debug_assert!(!components.is_empty());
        CombinedMarkupOutputFormat { components }
    }

    /// 名称 —— 对应 Java `getName`（:57-60）：`outer.getName() + "{" +
    /// inner.getName() + "}"` 递归；`[HTML, RTF]` → "HTML{RTF}"，
    /// `[RTF, RTF, RTF]` → "RTF{RTF{RTF}}"
    pub fn name(&self) -> String {
        let mut it = self.components.iter();
        let outer = it.next().expect("components must not be empty");
        let mut s = outer.name().to_string();
        for inner in it {
            s.push('{');
            s.push_str(inner.name());
            s.push('}');
        }
        s
    }

    /// MIME 类型 —— 对应 Java `getMimeType`（:63-65）：取最外层格式的 MIME
    pub fn mime_type(&self) -> &'static str {
        mime_type(self.components[0])
    }

    /// 纯文本转义 —— 对应 Java `escapePlainText`（:78-80）：
    /// `outer.escapePlainText(inner.escapePlainText(...))`——最内层先转义、逐层向外
    pub fn escape_plain_text(&self, s: &str) -> String {
        self.components
            .iter()
            .rev()
            .fold(s.to_string(), |acc, kind| escape_markup(*kind, &acc))
    }

    /// 输出纯文本 —— 对应 Java `output(String, Writer)`（:68-70）：
    /// `outer.output(inner.escapePlainText(textToEsc), out)`，即先经
    /// escapePlainText 转义（最外层再按自身 escape 一次）
    pub fn output(&self, s: &str) -> String {
        self.escape_plain_text(s)
    }

    /// 标记为"已转义 markup" —— 对应 Java `fromMarkup`（CommonMarkupOutputFormat
    /// :38-41：newTemplateMarkupOutputModel(null, markupText)；测试断言
    /// getMarkupContent 原样、getPlainTextContent 为 null）
    pub fn from_markup(&self, markup: String) -> CombinedMarkupOutputModel {
        CombinedMarkupOutputModel {
            plain_text: None,
            markup: Some(markup),
        }
    }

    /// 标记为"来自纯文本（未计算转义结果）" —— 对应 Java
    /// `fromPlainTextByEscaping`（CommonMarkupOutputFormat :34-37：模型只存
    /// plainTextContent，markupContent 为 null——"Not the MO's duty to calculate it!"）
    pub fn from_plain_text_by_escaping(&self, text: String) -> CombinedMarkupOutputModel {
        CombinedMarkupOutputModel {
            plain_text: Some(text),
            markup: None,
        }
    }

    /// markup 字符串 —— 对应 Java `getMarkupString`（CommonMarkupOutputFormat
    /// :47-58）：markupContent 非 null 原样返回，否则 escapePlainText 后返回
    pub fn get_markup_string(&self, mo: &CombinedMarkupOutputModel) -> String {
        if let Some(mc) = &mo.markup {
            return mc.clone();
        }
        self.escape_plain_text(mo.plain_text.as_deref().unwrap_or(""))
    }

    /// 输出模型 —— 对应 Java `output(MO, Writer)`（CommonMarkupOutputFormat
    /// :43-48）：markupContent 非 null 直接写出，否则 output(plainTextContent)
    pub fn output_model(&self, mo: &CombinedMarkupOutputModel) -> String {
        match &mo.markup {
            Some(mc) => mc.clone(),
            None => self.output(mo.plain_text.as_deref().unwrap_or("")),
        }
    }

    /// 模型拼接 —— 对应 Java `concat`（CommonMarkupOutputFormat :60-75）：
    /// 双 plain → plain 拼接；双 markup → markup 拼接；混合 → 纯文本侧经
    /// getMarkupString 转义后与 markup 侧拼接
    pub fn concat(
        &self,
        mo1: &CombinedMarkupOutputModel,
        mo2: &CombinedMarkupOutputModel,
    ) -> CombinedMarkupOutputModel {
        let pc1 = mo1.plain_text.clone();
        let mc1 = mo1.markup.clone();
        let pc2 = mo2.plain_text.clone();
        let mc2 = mo2.markup.clone();

        let pc3 = match (&pc1, &pc2) {
            (Some(a), Some(b)) => Some(format!("{a}{b}")),
            _ => None,
        };
        let mc3 = match (&mc1, &mc2) {
            (Some(a), Some(b)) => Some(format!("{a}{b}")),
            _ => None,
        };
        if pc3.is_some() || mc3.is_some() {
            return CombinedMarkupOutputModel {
                plain_text: pc3,
                markup: mc3,
            };
        }
        if pc1.is_some() {
            // Java：newTemplateMarkupOutputModel(null, getMarkupString(mo1) + mc2)
            CombinedMarkupOutputModel {
                plain_text: None,
                markup: Some(format!(
                    "{}{}",
                    self.get_markup_string(mo1),
                    mc2.expect("pc1/mc1 and pc2/mc2 can't both be null here")
                )),
            }
        } else {
            CombinedMarkupOutputModel {
                plain_text: None,
                markup: Some(format!(
                    "{}{}",
                    mc1.expect("pc1/mc1 and pc2/mc2 can't both be null here"),
                    self.get_markup_string(mo2)
                )),
            }
        }
    }

    /// 是否标记格式 —— 组合格式恒为标记格式（Java `isOutputFormatMixingAllowed`
    /// :93-95 委托 outer，:88-90 isAutoEscapedByDefault 委托 outer）
    pub fn is_markup(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::parse_combined_markup_format;

    #[test]
    fn parse_combined_names() {
        // Java Configuration.getOutputFormat(String)（Configuration.java:2351-2398）：
        // "HTML{RTF}" → [HTML, RTF]；递归 "XML{HTML{RTF}}" → [XML, HTML, RTF]
        assert_eq!(
            parse_combined_markup_format("HTML{RTF}"),
            Some(vec![OutputFormatKind::Html, OutputFormatKind::Rtf])
        );
        assert_eq!(
            parse_combined_markup_format("XML{HTML{RTF}}"),
            Some(vec![
                OutputFormatKind::Xml,
                OutputFormatKind::Html,
                OutputFormatKind::Rtf
            ])
        );
        // 简单名不受影响
        assert_eq!(
            parse_combined_markup_format("HTML"),
            Some(vec![OutputFormatKind::Html])
        );
        // 语法错误 / 非标记格式成员 → None（Java 抛 IllegalArgumentException /
        // "can't be used in ...{...} expression"）
        assert_eq!(parse_combined_markup_format(""), None);
        assert_eq!(parse_combined_markup_format("HTML{RTF"), None);
        assert_eq!(parse_combined_markup_format("{RTF}"), None);
        assert_eq!(parse_combined_markup_format("plainText{RTF}"), None);
    }

    #[test]
    fn combined_names_and_mime() {
        let html_rtf =
            CombinedMarkupOutputFormat::new(vec![OutputFormatKind::Html, OutputFormatKind::Rtf]);
        let xml_xml =
            CombinedMarkupOutputFormat::new(vec![OutputFormatKind::Xml, OutputFormatKind::Xml]);
        assert_eq!(html_rtf.name(), "HTML{RTF}");
        assert_eq!(xml_xml.name(), "XML{XML}");
        assert_eq!(html_rtf.mime_type(), "text/html");
        assert_eq!(xml_xml.mime_type(), "application/xml");
        assert!(html_rtf.is_markup());
    }
}
