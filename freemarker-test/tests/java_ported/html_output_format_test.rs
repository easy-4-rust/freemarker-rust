//! Java `freemarker.core.HTMLOutputFormatTest` 的 Rust 1:1 实现
//! （对应 Java: HTMLOutputFormatTest —— CommonMarkupOutputFormat 家族：
//!   output/fromPlainTextByEscaping/fromMarkup/getMarkupString/concat/isEmpty/
//!   escapePlainText/getMimeType）
//!
//! v1 无 TemplateMarkupOutputModel 类型 —— 用本地 MO 镜像结构（HtmlMo）复刻
//! CommonMarkupOutputFormat 逻辑，转义用 freemarker::template::utility::html_escape
//! （对应 Java StringUtil.HTMLEnc，字符集 & < > " ' 一致）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use std::cell::RefCell;
use std::rc::Rc;

/// 标记输出模型镜像（对应 Java TemplateHTMLOutputModel：plainTextContent/markupContent）
struct HtmlMo {
    plain: Option<String>,
    markup: Option<String>,
    /// 惰性计算的 markup（对应 CommonMarkupOutputFormat.getMarkupString 缓存；
    /// 缓存 Rc 本体以实现 Java assertSame 的同一性语义）
    cached_markup: Rc<RefCell<Option<Rc<String>>>>,
}

impl HtmlMo {
    fn from_plain_text_by_escaping(s: &str) -> HtmlMo {
        HtmlMo {
            plain: Some(s.to_string()),
            markup: None,
            cached_markup: Rc::new(RefCell::new(None)),
        }
    }
    fn from_markup(s: &str) -> HtmlMo {
        HtmlMo {
            plain: None,
            markup: Some(s.to_string()),
            cached_markup: Rc::new(RefCell::new(None)),
        }
    }
    fn get_plain_text_content(&self) -> Option<&str> {
        self.plain.as_deref()
    }
    fn get_markup_content(&self) -> Option<&str> {
        self.markup.as_deref()
    }
}

/// Java CommonMarkupOutputFormat.output(MO, out)：mc != null 直写；否则 escapePlainText(pc)
fn output(mo: &HtmlMo, out: &mut String) {
    match mo.get_markup_content() {
        Some(mc) => out.push_str(mc),
        None => out.push_str(&escape_plain_text(
            mo.get_plain_text_content().unwrap_or(""),
        )),
    }
}

/// Java CommonMarkupOutputFormat.getMarkupString：mc 或 escapePlainText(pc)（结果缓存；
/// Java assertSame(mc, getMarkupString(mo)) 要求缓存命中返回同一对象 → 缓存 Rc）
fn get_markup_string(mo: &HtmlMo) -> Rc<String> {
    if let Some(mc) = mo.get_markup_content() {
        return Rc::new(mc.to_string());
    }
    let mut cache = mo.cached_markup.borrow_mut();
    if cache.is_none() {
        *cache = Some(Rc::new(escape_plain_text(
            mo.get_plain_text_content().unwrap_or(""),
        )));
    }
    cache.clone().unwrap()
}

/// Java CommonMarkupOutputFormat.concat（pc 合并 / mc 合并 / 单侧转义拼接）
fn concat(mo1: &HtmlMo, mo2: &HtmlMo) -> HtmlMo {
    let pc1 = mo1.get_plain_text_content();
    let mc1 = mo1.get_markup_content();
    let pc2 = mo2.get_plain_text_content();
    let mc2 = mo2.get_markup_content();
    match (pc1, pc2) {
        (Some(a), Some(b)) => HtmlMo {
            plain: Some(a.to_string() + b),
            markup: None,
            cached_markup: Rc::new(RefCell::new(None)),
        },
        _ => match (mc1, mc2) {
            (Some(a), Some(b)) => HtmlMo {
                plain: None,
                markup: Some(a.to_string() + b),
                cached_markup: Rc::new(RefCell::new(None)),
            },
            _ => {
                // 单侧 plain（Java：pc1 != null → getMarkupString(mo1) + mc2）
                let m = if pc1.is_some() {
                    format!("{}{}", get_markup_string(mo1), mc2.unwrap_or(""))
                } else {
                    format!("{}{}", mc1.unwrap_or(""), get_markup_string(mo2))
                };
                HtmlMo {
                    plain: None,
                    markup: Some(m),
                    cached_markup: Rc::new(RefCell::new(None)),
                }
            }
        },
    }
}

/// Java HTMLOutputFormat.escapePlainText（= StringUtil.HTMLEnc）
fn escape_plain_text(s: &str) -> String {
    freemarker::template::utility::html_escape(s)
}

/// Java HTMLOutputFormat.isEmpty
fn is_empty(mo: &HtmlMo) -> bool {
    match mo.get_plain_text_content() {
        Some(s) => s.is_empty(),
        None => mo.get_markup_content().unwrap_or("").is_empty(),
    }
}

/// Java HTMLOutputFormat.getMimeType
fn get_mime_type() -> &'static str {
    "text/html"
}

/// Java testOutputMO：混合输出 MO（markup 原样、plain 转义）
#[test]
fn test_output_mo() {
    let mut out = String::new();
    output(&HtmlMo::from_markup("<p>Test "), &mut out);
    output(&HtmlMo::from_plain_text_by_escaping("foo & bar "), &mut out);
    output(&HtmlMo::from_plain_text_by_escaping("baaz "), &mut out);
    output(
        &HtmlMo::from_plain_text_by_escaping("<b>A</b> <b>B</b> <b>C</b>"),
        &mut out,
    );
    output(&HtmlMo::from_plain_text_by_escaping(""), &mut out);
    output(
        &HtmlMo::from_plain_text_by_escaping("\"' x's \"y\" \""),
        &mut out,
    );
    output(&HtmlMo::from_markup("</p>"), &mut out);
    assert_eq!(
        out,
        "<p>Test foo &amp; bar baaz &lt;b&gt;A&lt;/b&gt; &lt;b&gt;B&lt;/b&gt; &lt;b&gt;C&lt;/b&gt;&quot;&#39; x&#39;s &quot;y&quot; &quot;</p>"
    );
}

/// Java testOutputString：output(String) 直接转义
#[test]
fn test_output_string() {
    let mut out = String::new();
    out.push_str(&escape_plain_text("a"));
    out.push_str(&escape_plain_text("<"));
    out.push_str(&escape_plain_text("b'c"));
    assert_eq!(out, "a&lt;b&#39;c");
}

/// Java testFromPlainTextByEscaping：pc 原样、mc 为 null
#[test]
fn test_from_plain_text_by_escaping() {
    let plain_text = "a&b";
    let mo = HtmlMo::from_plain_text_by_escaping(plain_text);
    assert_eq!(mo.get_plain_text_content(), Some(plain_text));
    assert_eq!(mo.get_markup_content(), None); // Not the MO's duty to calculate it!
}

/// Java testFromMarkup：mc 原样、pc 为 null
#[test]
fn test_from_markup() {
    let markup = "a&amp;b";
    let mo = HtmlMo::from_markup(markup);
    assert_eq!(mo.get_markup_content(), Some(markup));
    assert_eq!(mo.get_plain_text_content(), None);
}

/// Java testGetMarkup：getMarkupString（含缓存同一性 assertSame）
#[test]
fn test_get_markup() {
    // fromMarkup：mc 原样返回
    let markup = "a&amp;b";
    let mo = HtmlMo::from_markup(markup);
    assert_eq!(get_markup_string(&mo).as_str(), markup);
    // fromPlainTextByEscaping：安全串原样
    for safe in ["abc", ""] {
        let mo = HtmlMo::from_plain_text_by_escaping(safe);
        assert_eq!(get_markup_string(&mo).as_str(), safe);
    }
    let cases = [
        ("<abc", "&lt;abc"),
        ("abc>", "abc&gt;"),
        ("<abc>", "&lt;abc&gt;"),
        ("a&bc", "a&amp;bc"),
        ("a&b&c", "a&amp;b&amp;c"),
        ("a<&>b&c", "a&lt;&amp;&gt;b&amp;c"),
        ("\"<a<&>b&c>\"", "&quot;&lt;a&lt;&amp;&gt;b&amp;c&gt;&quot;"),
        ("<", "&lt;"),
    ];
    for (input, expected) in cases {
        let mo = HtmlMo::from_plain_text_by_escaping(input);
        assert_eq!(get_markup_string(&mo).as_str(), expected);
    }
    // 缓存同一性（Java assertSame(mc, getMarkupString(mo))）：二次调用同 Rc
    let mo = HtmlMo::from_plain_text_by_escaping("'");
    let mc1 = get_markup_string(&mo);
    assert_eq!(mc1.as_str(), "&#39;");
    let mc2 = get_markup_string(&mo);
    assert!(Rc::ptr_eq(&mc1, &mc2), "缓存命中应返回同一对象");
}

/// Java testConcat：concat 的 pc/mc 合并与单侧转义
#[test]
fn test_concat() {
    assert_mo(
        Some("ab"),
        None,
        &concat(
            &HtmlMo::from_plain_text_by_escaping("a"),
            &HtmlMo::from_plain_text_by_escaping("b"),
        ),
    );
    assert_mo(
        None,
        Some("ab"),
        &concat(&HtmlMo::from_markup("a"), &HtmlMo::from_markup("b")),
    );
    assert_mo(
        None,
        Some("<a>&lt;b&gt;"),
        &concat(
            &HtmlMo::from_markup("<a>"),
            &HtmlMo::from_plain_text_by_escaping("<b>"),
        ),
    );
    assert_mo(
        None,
        Some("&lt;a&gt;<b>"),
        &concat(
            &HtmlMo::from_plain_text_by_escaping("<a>"),
            &HtmlMo::from_markup("<b>"),
        ),
    );
}

fn assert_mo(pc: Option<&str>, mc: Option<&str>, mo: &HtmlMo) {
    assert_eq!(mo.get_plain_text_content(), pc);
    assert_eq!(mo.get_markup_content(), mc);
}

/// Java testEscaplePlainText：escapePlainText 各种输入
#[test]
fn test_escape_plain_text() {
    assert_eq!(escape_plain_text(""), "");
    assert_eq!(escape_plain_text("a"), "a");
    assert_eq!(
        escape_plain_text("<a&b'c\"d>"),
        "&lt;a&amp;b&#39;c&quot;d&gt;"
    );
    assert_eq!(escape_plain_text("a&b"), "a&amp;b");
    assert_eq!(escape_plain_text("<>"), "&lt;&gt;");
}

/// Java testIsEmpty
#[test]
fn test_is_empty() {
    assert!(is_empty(&HtmlMo::from_markup("")));
    assert!(is_empty(&HtmlMo::from_plain_text_by_escaping("")));
    assert!(!is_empty(&HtmlMo::from_markup(" ")));
    assert!(!is_empty(&HtmlMo::from_plain_text_by_escaping(" ")));
}

/// Java testGetMimeType
#[test]
fn test_get_mime_type() {
    assert_eq!(get_mime_type(), "text/html");
}
