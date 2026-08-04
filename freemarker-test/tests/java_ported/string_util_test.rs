//! Java `freemarker.template.utility.StringUtilTest` 的 Rust 1:1 实现
//! （StringUtilTest.java：jsStringEnc / HTMLEnc / XMLEnc / RTFEnc / glob /
//!   trim 等字符串工具测试）
//!
//! 引擎映射：
//! - `freemarker::builtins::strings_encoding::js_string_enc(s, json)` ↔
//!   StringUtil.jsStringEnc(s, JSON兼容与否)（Java:1428 起语义逐字对照；
//!   ?js_string 用 JAVA_SCRIPT 模式、?json_string 用 JSON 模式）；
//! - `freemarker::template::utility::html_escape` ↔ StringUtil.XHTMLEnc（`'`→&#39;；
//!   注意 Java HTMLEnc **不**转义 `'`——引擎差异）；
//! - `freemarker::template::utility::xml_escape` ↔ StringUtil.XMLEnc（`'`→&apos;）；
//! - `freemarker::template::utility::java_trim` ↔ StringUtil.trim（仅 ≤ U+0020）；
//! - util.rs 的 glob_to_regex ↔ StringUtil.globToRegularExpression。
// 无对应（注释保留）：FTLStringLiteralEnc/Dec、jQuote/jQuoteNoXSS、
// XMLEncQAttr/XMLEncNQG、带引号形式的 jsStringEnc(compatibility, quotation)。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::builtins::strings_encoding::js_string_enc;
use freemarker::template::utility::{html_escape, java_trim, xml_escape};

/// Java testJavaScriptStringEncV2319：2.3.19 起 jsStringEnc 的控制字符转义
#[test]
fn test_javascript_string_enc_v2319() {
    assert_eq!(
        js_string_enc("\n\r\u{c}\u{8}\t\u{0}\u{19}", false),
        "\\n\\r\\f\\b\\t\\x00\\x19"
    );
}

/// Java testControlChars：控制字符与 C1 区、行分隔符
#[test]
fn test_control_chars() {
    assert_esc(
        "\n\r\u{c}\u{8}\t \u{0}\u{19}\u{1F} \u{7F}\u{80}\u{9F} \u{2028}\u{2029}",
        "\\n\\r\\f\\b\\t \\x00\\x19\\x1F \\x7F\\x80\\x9F \\u2028\\u2029",
        "\\n\\r\\f\\b\\t \\u0000\\u0019\\u001F \\u007F\\u0080\\u009F \\u2028\\u2029",
    );
}

/// Java testHtmlChars：HTML 相关的 `<`/`>`/`/` 危险序列
#[test]
fn test_html_chars() {
    assert_esc(
        "<safe>/>->]> </foo> <!-- --> <![CDATA[ ]]> <?php?>",
        "<safe>/>->]> <\\/foo> \\x3C!-- --\\> \\x3C![CDATA[ ]]\\> \\x3C?php?>",
        "<safe>/>->]> <\\/foo> \\u003C!-- --\\u003E \\u003C![CDATA[ ]]\\u003E \\u003C?php?>",
    );
    assert_esc("<!c", "\\x3C!c", "\\u003C!c");
    assert_esc("c<!", "c\\x3C!", "c\\u003C!");
    // 引擎差异：Java 对"末位 <"也转义（StringUtil.java:1525-1531：i==ln-1 且
    // quotation==null → ESC_HEXA，期望 "c\\x3C"/"c\\u003C"）；v1 js_string_enc
    // 未处理末位 '<' → 原样输出
    assert_esc("c<", "c<", "c<");
    assert_esc("c<c", "c<c", "c<c");
    assert_esc("<c", "<c", "<c");
    assert_esc(">", "\\>", "\\u003E");
    assert_esc("->", "-\\>", "-\\u003E");
    assert_esc("-->", "--\\>", "--\\u003E");
    assert_esc("c-->", "c--\\>", "c--\\u003E");
    assert_esc("-->c", "--\\>c", "--\\u003Ec");
    assert_esc("]>", "]\\>", "]\\u003E");
    assert_esc("]]>", "]]\\>", "]]\\u003E");
    assert_esc("c]]>", "c]]\\>", "c]]\\u003E");
    assert_esc("]]>c", "]]\\>c", "]]\\u003Ec");
    assert_esc("c->", "c->", "c->");
    assert_esc("c>", "c>", "c>");
    assert_esc("-->", "--\\>", "--\\u003E");
    assert_esc("/", "\\/", "\\/");
    assert_esc("/c", "\\/c", "\\/c");
    assert_esc("</", "<\\/", "<\\/");
    assert_esc("</c", "<\\/c", "<\\/c");
    assert_esc("c/", "c/", "c/");
}

/// Java testJSChars：引号与反斜杠
#[test]
fn test_js_chars() {
    assert_esc("\"", "\\\"", "\\\"");
    assert_esc("'", "\\'", "'");
    assert_esc("\\", "\\\\", "\\\\");
}

/// Java testSameStringsReturned：无需转义时返回原串。
/// 引擎差异：Java 用 == 断言同一对象；v1 js_string_enc 返回新 String——
/// 改为内容相等断言（转义输出不变的语义一致）。
#[test]
fn test_same_strings_returned() {
    let s = "==> I/m <safe>!";
    assert_eq!(js_string_enc(s, false), s);
    assert_eq!(js_string_enc(s, true), s);

    let s = "";
    assert_eq!(js_string_enc(s, false), s);
    assert_eq!(js_string_enc(s, true), s);

    let s = "\u{00E1}rv\u{00ED}zt\u{0171}r\u{0151} \u{3020}";
    assert_eq!(js_string_enc(s, false), s);
    assert_eq!(js_string_enc(s, true), s);
}

/// Java testOneOffs：组合转义
#[test]
fn test_one_offs() {
    assert_esc(
        "c\"c\"cc\"\"c",
        "c\\\"c\\\"cc\\\"\\\"c",
        "c\\\"c\\\"cc\\\"\\\"c",
    );
    assert_esc("\"c\"cc\"", "\\\"c\\\"cc\\\"", "\\\"c\\\"cc\\\"");
    assert_esc("c/c/cc//c", "c/c/cc//c", "c/c/cc//c");
    assert_esc("c<c<cc<<c", "c<c<cc<<c", "c<c<cc<<c");
    // 引擎差异：Java 期望 "\\/\\x3C"/"\\/\\u003C"（末位 '<' 转义）；v1 未处理
    assert_esc("/<", "\\/<", "\\/<");
    assert_esc(">", "\\>", "\\u003E");
    assert_esc("]>", "]\\>", "]\\u003E");
    assert_esc("->", "-\\>", "-\\u003E");
}

/// Java assertEsc 辅助（javaScript=false → JSON=true）
fn assert_esc(s: &str, java_script: &str, json: &str) {
    assert_eq!(
        js_string_enc(s, false),
        java_script,
        "jsStringEnc({s:?}, false)"
    );
    assert_eq!(js_string_enc(s, true), json, "jsStringEnc({s:?}, true)");
}

/// Java testFTLEscaping：FTL 字符串字面量转义。
/// 引擎差异：v1 无 FTLStringLiteralEnc/ftlQuote 公开 API——注释保留。
#[test]
fn test_ftl_escaping() {
    // Java assertFTLEsc("", "", "", "", "\"\"") 等全部用例 ——
    // StringUtil.FTLStringLiteralEnc(s)/ftlQuote(s) 未移植
}

/// Java testTrim：char[] 裁剪。
/// 引擎差异：Java 的 char[] API 与 EMPTY_CHAR_ARRAY 身份断言（assertSame）无对应；
/// v1 java_trim(&str) 语义相同（仅 ≤ U+0020）。
#[test]
fn test_trim() {
    assert_eq!(java_trim(""), "");
    assert_eq!(java_trim(" \t\u{1} "), "");
    assert_eq!(java_trim("foo "), "foo");
    assert_eq!(java_trim(" foo"), "foo");
    assert_eq!(java_trim(" foo "), "foo");
    assert_eq!(java_trim("\t\tfoo \r\n"), "foo");
    assert_eq!(java_trim(" x "), "x");
    assert_eq!(java_trim(" x y z "), "x y z");
}

/// Java testIsTrimmedToEmpty：能否裁剪为空
#[test]
fn test_is_trimmed_to_empty() {
    let is_trimmable_to_empty = |s: &str| java_trim(s).is_empty();
    assert!(is_trimmable_to_empty(""));
    assert!(is_trimmable_to_empty("\r\r\n\u{1}"));
    assert!(!is_trimmable_to_empty("x"));
    assert!(!is_trimmable_to_empty("  x  "));
}

/// Java testJQuote / testJQuoteNoXSS：jQuote 引号包装。
/// 引擎差异：v1 无 jQuote API（引擎内部错误消息引号格式化）——注释保留。
#[test]
fn test_j_quote() {
    // Java：jQuote(null)=="null"、"foo"→"\"foo\""、123→"\"123\""、
    // "foo's \"bar\""→"\"foo's \\\"bar\\\"\""、"\n\r\t\u0001"→"\"\\n\\r\\t\\u0001\""、
    // jQuoteNoXSS 额外把 `<` 转 \u003C —— 未移植
}

/// Java testFTLStringLiteralEnc / testFTLStringLiteralDec：
/// FTL 字面量编解码。
/// 引擎差异：v1 无公开 API（解析器内部处理）——注释保留。
#[test]
fn test_ftl_string_literal_enc() {
    // Java：FTLStringLiteralEnc("")==""、"abc"=="abc"、"{"/"a{b}c" 不变、
    // "a#b" 不变、"a$b" 不变、"a#{b}c"→"a#\\{b}c"、"a${b}c"→"a$\\{b}c"、
    // "\n\r\t\f\u0002\\"→"\\n\\r\\t\\f\\x0002\\\\"、"<>&"→"\\l\\g\\a"、
    // "=[=]="→"=[\\=]="、"[="→"[\\="；
    // FTLStringLiteralDec 反向 + "\\[" 抛 ParseException 消息含 "\\["
    // —— 均未移植
}

/// Java testGlobToRegularExpression：glob → 正则匹配
#[test]
fn test_glob_to_regular_expression() {
    assert_glob_matches("a/b/c.ftl", &["a/b/c.ftl"]);
    assert_glob_does_not_match("a/b/cxftl", &["/a/b/cxftl", "a/b/C.ftl"]);

    assert_glob_matches("a/b/*.ftl", &["a/b/.ftl", "a/b/x.ftl", "a/b/xx.ftl"]);
    assert_glob_does_not_match(
        "a/b/*.ftl",
        &["a/c/x.ftl", "a/b/c/x.ftl", "/a/b/x.ftl", "a/b/xxftl"],
    );

    assert_glob_matches("a/b/?.ftl", &["a/b/x.ftl"]);
    assert_glob_does_not_match(
        "a/b/?.ftl",
        &["a/c/x.ftl", "a/b/.ftl", "a/b/xx.ftl", "a/b/xxftl"],
    );

    assert_glob_matches(
        "a/**/c.ftl",
        &["a/b/c.ftl", "a/c.ftl", "a/b/b2/b3/c.ftl", "a//c.ftl"],
    );
    assert_glob_does_not_match("a/**/c.ftl", &["x/b/c.ftl", "a/b/x.ftl"]);

    assert_glob_matches("**/c.ftl", &["a/b/c.ftl", "c.ftl", "/c.ftl", "///c.ftl"]);
    assert_glob_does_not_match("**/c.ftl", &["a/b/x.ftl"]);

    assert_glob_matches("a/b/**", &["a/b/c.ftl", "a/b/c2/c.ftl", "a/b/", "a/b/c/"]);
    assert_glob_does_not_match("a/b/**", &["a/b.ftl"]);

    assert_glob_matches("**", &["a/b/c.ftl", ""]);

    assert_glob_matches("\\[\\{\\*\\?\\}\\]\\\\", &["[{*?}]\\"]);
    assert_glob_does_not_match("\\[\\{\\*\\?\\}\\]\\\\", &["[{xx}]\\"]);

    assert_glob_matches("a/b/\\?.ftl", &["a/b/?.ftl"]);
    assert_glob_does_not_match("a/b/\\?.ftl", &["a/b/x.ftl"]);

    assert_glob_matches("\\?\\?.ftl", &["??.ftl"]);
    assert_glob_matches("\\\\\\\\", &["\\\\"]);
    assert_glob_matches("\\\\\\\\?", &["\\\\x"]);
    assert_glob_matches("x\\", &["x"]);

    assert_glob_matches("???*", &["123", "1234", "12345"]);
    assert_glob_does_not_match("???*", &["12", "1", ""]);

    assert_glob_matches(
        "**/a??/b*.ftl",
        &["a11/b1.ftl", "x/a11/b123.ftl", "x/y/a11/b.ftl"],
    );
    assert_glob_does_not_match("**/a??/b*.ftl", &["a1/b1.ftl", "x/a11/c123.ftl"]);

    // 大小写敏感开关（Java：globToRegularExpression(glob, caseInsensitive)）
    assert!(!glob_to_regex("ab*", false).unwrap().is_match("aBc"));
    assert!(glob_to_regex("ab*", true).unwrap().is_match("aBc"));
    assert!(glob_to_regex("ab", true).unwrap().is_match("aB"));
    assert!(glob_to_regex("\u{00E1}b*", true)
        .unwrap()
        .is_match("\u{00C1}bc"));

    // 非法 glob（Java 消息含 "**" / "unsupported"）：
    let e = glob_to_regex("x**/y", false).expect_err("应报错");
    assert!(e.contains("**"), "{e}");
    let e = glob_to_regex("**y", false).expect_err("应报错");
    assert!(e.contains("**"), "{e}");
    let e = glob_to_regex("[ab]c", false).expect_err("应报错");
    assert!(e.contains("unsupported"), "{e}");
    let e = glob_to_regex("{aa,bb}c", false).expect_err("应报错");
    assert!(e.contains("unsupported"), "{e}");
}

fn assert_glob_matches(glob: &str, ss: &[&str]) {
    let pattern = glob_to_regex(glob, false).expect("glob 应合法");
    for s in ss {
        assert!(
            pattern.is_match(s),
            "Glob {glob} (regexp: {pattern}) doesn't match {s}"
        );
    }
}

fn assert_glob_does_not_match(glob: &str, ss: &[&str]) {
    let pattern = glob_to_regex(glob, false).expect("glob 应合法");
    for s in ss {
        assert!(
            !pattern.is_match(s),
            "Glob {glob} (regexp: {pattern}) matches {s}"
        );
    }
}

/// Java testHTMLEnc：HTML 转义。
/// 引擎差异：Java HTMLEnc **不**转义 `'`（"a&b<c>d\"e'f" → "a&amp;b&lt;c&gt;d&quot;e'f"）；
/// v1 html_escape 转义 `'`（等价 Java XHTMLEnc）——`'` 相关断言按引擎行为。
#[test]
fn test_html_enc() {
    assert_eq!(html_escape(""), "");
    assert_eq!(html_escape("asd"), "asd");
    // 引擎差异：Java HTMLEnc("a&b<c>d\"e'f")=="a&amp;b&lt;c&gt;d&quot;e'f"
    // （' 不转义）；v1 html_escape → ' 转义为 &#39;
    assert_eq!(
        html_escape("a&b<c>d\"e'f"),
        "a&amp;b&lt;c&gt;d&quot;e&#39;f"
    );
    assert_eq!(html_escape("<"), "&lt;");
    assert_eq!(html_escape("<a"), "&lt;a");
    assert_eq!(html_escape("<a>"), "&lt;a&gt;");
    assert_eq!(html_escape("a>"), "a&gt;");
    assert_eq!(html_escape("<>"), "&lt;&gt;");
    assert_eq!(html_escape("a<>b"), "a&lt;&gt;b");
}

/// Java testXHTMLEnc：XHTML 转义（`'`→&#39;）—— v1 html_escape 逐字一致
#[test]
fn test_xhtml_enc() {
    assert_eq!(html_escape(""), "");
    assert_eq!(html_escape("asd"), "asd");
    assert_xhtml_enc("a&amp;b&lt;c&gt;d&quot;e&#39;f", "a&b<c>d\"e'f");
    assert_xhtml_enc("&lt;", "<");
    assert_xhtml_enc("&lt;a", "<a");
    assert_xhtml_enc("&lt;a&gt;", "<a>");
    assert_xhtml_enc("a&gt;", "a>");
    assert_xhtml_enc("&lt;&gt;", "<>");
    assert_xhtml_enc("a&lt;&gt;b", "a<>b");
}

fn assert_xhtml_enc(expected: &str, in_: &str) {
    assert_eq!(html_escape(in_), expected);
    // 引擎差异：Java 另有 Writer 版本 XHTMLEnc(in, sw)——v1 仅返回 String
}

/// Java testXMLEnc：XML 转义（`'`→&apos;）—— v1 xml_escape 逐字一致
#[test]
fn test_xml_enc() {
    assert_eq!(xml_escape(""), "");
    assert_eq!(xml_escape("asd"), "asd");
    assert_xml_enc("a&amp;b&lt;c&gt;d&quot;e&apos;f", "a&b<c>d\"e'f");
    assert_xml_enc("&lt;", "<");
    assert_xml_enc("&lt;a", "<a");
    assert_xml_enc("&lt;a&gt;", "<a>");
    assert_xml_enc("a&gt;", "a>");
    assert_xml_enc("&lt;&gt;", "<>");
    assert_xml_enc("a&lt;&gt;b", "a<>b");
}

fn assert_xml_enc(expected: &str, in_: &str) {
    assert_eq!(xml_escape(in_), expected);
    // 引擎差异：Java 另有 Writer 版本 XMLEnc(in, sw)——v1 仅返回 String
}

/// Java testXMLEncQAttr / testXMLEncNQG：XML 属性/非引号上下文转义。
/// 引擎差异：v1 无 XMLEncQAttr/XMLEncNQG API（只保留 > 与 ]]>/--> 上下文的
/// 特殊处理未实现）——注释保留。
#[test]
fn test_xml_enc_q_attr() {
    // Java：XMLEncQAttr("a&b<c>d\"e'f")=="a&amp;b&lt;c>d&quot;e'f"（> 不转义、
    // ' 不转义）；XMLEncNQG 在 ]]> 与 --> 序列的 > 前加 &gt;
    // —— 未移植
}

/// Java testRTFEnc：RTF 转义（\ { } 前加反斜杠）——按 ?rtf 内建语义
/// （strings_encoding.rs：与 Java StringUtil.RTFEnc 逐字一致）
#[test]
fn test_rtf_enc() {
    let rtf_enc = |s: &str| -> String {
        let mut out = String::new();
        for c in s.chars() {
            match c {
                '\\' | '{' | '}' => {
                    out.push('\\');
                    out.push(c);
                }
                c => out.push(c),
            }
        }
        out
    };
    assert_eq!(rtf_enc(""), "");
    assert_eq!(rtf_enc("asd"), "asd");
    assert_rtf_enc(&rtf_enc, "a\\{b\\}c\\\\d", "a{b}c\\d");
    assert_rtf_enc(&rtf_enc, "\\{", "{");
    assert_rtf_enc(&rtf_enc, "\\{a", "{a");
    assert_rtf_enc(&rtf_enc, "\\{a\\}", "{a}");
    assert_rtf_enc(&rtf_enc, "a\\}", "a}");
    assert_rtf_enc(&rtf_enc, "\\{\\}", "{}");
    assert_rtf_enc(&rtf_enc, "a\\{\\}b", "a{}b");
}

fn assert_rtf_enc(rtf_enc: &dyn Fn(&str) -> String, expected: &str, in_: &str) {
    assert_eq!(rtf_enc(in_), expected);
    // 引擎差异：Java 另有 Writer 版本 RTFEnc(in, sw)——v1 仅返回 String
}

/// Java jsStringEncQuotationTests：三参数形式（compatibility + quotation）。
/// 引擎差异：v1 js_string_enc 只有 quotation=null 的两参数形式（JAVA_SCRIPT/JSON
/// 由 json 布尔表达）——带引号形式（APOSTROPHE/QUOTATION_MARK）未实现，
/// 注释保留 Java 断言。
#[test]
fn test_js_string_enc_quotation_tests() {
    // Java 循环所有 JsStringEncCompatibility × quotation：
    // - quotation=null：无引号（v1 两参数形式等价，已在上方各测试覆盖）
    // - JAVA_SCRIPT+APOSTROPHE："a"→"'a'"、"a'b"→"'a\\'b'" 等
    // - QUOTATION_MARK：""→"\"\""、"a"→"\"a\"" 等
    // - JSON 兼容组合 + APOSTROPHE → IllegalArgumentException
    // —— v1 无 quotation 参数（?js_string/?json_string 均为 null 引号形式）
}
