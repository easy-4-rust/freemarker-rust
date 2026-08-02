//! Java `freemarker.core.RTFOutputFormatTest` 的 Rust 1:1 实现
//! （对应 Java: RTFOutputFormatTest —— RTF 转义规则：`\`→`\\`、`{`→`\{`、`}`→`\}`；
//!   该类只测内建 RTF 输出格式的纯字符串规则，v1 无 RTF 格式化器 —— 用本地
//!   rtf_escape 复刻 Java RTFOutputFormat.escapePlainText 逐字符逻辑）

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java RTFOutputFormat.escapePlainText（逐字符：`\`→`\\`、`{`→`\{`、`}`→`\}`）
fn escape_plain_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            _ => out.push(c),
        }
    }
    out
}

/// Java RTFOutputFormat.getMimeType
fn get_mime_type() -> &'static str {
    "application/rtf"
}

/// 标记输出模型镜像（对应 TemplateRTFOutputModel）
struct RtfMo {
    plain: Option<String>,
    markup: Option<String>,
}

impl RtfMo {
    fn from_plain_text_by_escaping(s: &str) -> RtfMo {
        RtfMo {
            plain: Some(s.to_string()),
            markup: None,
        }
    }
    fn from_markup(s: &str) -> RtfMo {
        RtfMo {
            plain: None,
            markup: Some(s.to_string()),
        }
    }
    fn get_plain_text_content(&self) -> Option<&str> {
        self.plain.as_deref()
    }
    fn get_markup_content(&self) -> Option<&str> {
        self.markup.as_deref()
    }
}

/// Java CommonMarkupOutputFormat.output(MO, out)：mc 直写；否则 escapePlainText(pc)
fn output(mo: &RtfMo, out: &mut String) {
    match mo.get_markup_content() {
        Some(mc) => out.push_str(mc),
        None => out.push_str(&escape_plain_text(
            mo.get_plain_text_content().unwrap_or(""),
        )),
    }
}

/// Java CommonMarkupOutputFormat.getMarkupString
fn get_markup_string(mo: &RtfMo) -> String {
    match mo.get_markup_content() {
        Some(mc) => mc.to_string(),
        None => escape_plain_text(mo.get_plain_text_content().unwrap_or("")),
    }
}

/// Java CommonMarkupOutputFormat.concat
fn concat(mo1: &RtfMo, mo2: &RtfMo) -> RtfMo {
    let pc1 = mo1.get_plain_text_content();
    let mc1 = mo1.get_markup_content();
    let pc2 = mo2.get_plain_text_content();
    let mc2 = mo2.get_markup_content();
    match (pc1, pc2) {
        (Some(a), Some(b)) => RtfMo::from_plain_text_by_escaping(&(a.to_string() + b)),
        _ => match (mc1, mc2) {
            (Some(a), Some(b)) => RtfMo {
                plain: None,
                markup: Some(a.to_string() + b),
            },
            _ => {
                let m = if pc1.is_some() {
                    format!("{}{}", get_markup_string(mo1), mc2.unwrap_or(""))
                } else {
                    format!("{}{}", mc1.unwrap_or(""), get_markup_string(mo2))
                };
                RtfMo {
                    plain: None,
                    markup: Some(m),
                }
            }
        },
    }
}

/// Java testOutputMO：fromMarkup 原样、fromPlainTextByEscaping 转义
#[test]
fn test_output_mo() {
    let mut out = String::new();
    output(&RtfMo::from_markup("\\par Test "), &mut out);
    output(
        &RtfMo::from_plain_text_by_escaping("foo { bar } \\ "),
        &mut out,
    );
    output(&RtfMo::from_plain_text_by_escaping("baaz "), &mut out);
    output(
        &RtfMo::from_plain_text_by_escaping("\\par qweqwe"),
        &mut out,
    );
    output(&RtfMo::from_markup("\\par{0} End"), &mut out);
    assert_eq!(
        out,
        "\\par Test foo \\{ bar \\} \\\\ baaz \\\\par qweqwe\\par{0} End"
    );
}

/// Java testOutputString
#[test]
fn test_output_string() {
    let mut out = String::new();
    out.push_str(&escape_plain_text("a"));
    out.push_str(&escape_plain_text("{"));
    out.push_str(&escape_plain_text("b}c"));
    assert_eq!(out, "a\\{b\\}c");
}

/// Java testFromPlainTextByEscaping
#[test]
fn test_from_plain_text_by_escaping() {
    let plain_text = "a\\b";
    let mo = RtfMo::from_plain_text_by_escaping(plain_text);
    assert_eq!(mo.get_plain_text_content(), Some(plain_text));
    assert_eq!(mo.get_markup_content(), None);
}

/// Java testFromMarkup
#[test]
fn test_from_markup() {
    let markup = "a \\par b";
    let mo = RtfMo::from_markup(markup);
    assert_eq!(mo.get_markup_content(), Some(markup));
    assert_eq!(mo.get_plain_text_content(), None);
}

/// Java testGetMarkup
#[test]
fn test_get_markup() {
    let markup = "a \\par b";
    let mo = RtfMo::from_markup(markup);
    assert_eq!(get_markup_string(&mo), markup);
    let safe = "abc";
    let mo = RtfMo::from_plain_text_by_escaping(safe);
    assert_eq!(get_markup_string(&mo), safe);
}

/// Java testConcat
#[test]
fn test_concat() {
    assert_mo(
        Some("ab"),
        None,
        &concat(
            &RtfMo::from_plain_text_by_escaping("a"),
            &RtfMo::from_plain_text_by_escaping("b"),
        ),
    );
    assert_mo(
        None,
        Some("ab"),
        &concat(&RtfMo::from_markup("a"), &RtfMo::from_markup("b")),
    );
    assert_mo(
        None,
        Some("{a}\\{b\\}"),
        &concat(
            &RtfMo::from_markup("{a}"),
            &RtfMo::from_plain_text_by_escaping("{b}"),
        ),
    );
    assert_mo(
        None,
        Some("\\{a\\}{b}"),
        &concat(
            &RtfMo::from_plain_text_by_escaping("{a}"),
            &RtfMo::from_markup("{b}"),
        ),
    );
}

fn assert_mo(pc: Option<&str>, mc: Option<&str>, mo: &RtfMo) {
    assert_eq!(mo.get_plain_text_content(), pc);
    assert_eq!(mo.get_markup_content(), mc);
}

/// Java testEscaplePlainText
#[test]
fn test_escape_plain_text() {
    assert_eq!(escape_plain_text(""), "");
    assert_eq!(escape_plain_text("a"), "a");
    assert_eq!(escape_plain_text("{a\\b}"), "\\{a\\\\b\\}");
    assert_eq!(escape_plain_text("a\\b"), "a\\\\b");
    assert_eq!(escape_plain_text("{}"), "\\{\\}");
}

/// Java testGetMimeType
#[test]
fn test_get_mime_type() {
    assert_eq!(get_mime_type(), "application/rtf");
}
