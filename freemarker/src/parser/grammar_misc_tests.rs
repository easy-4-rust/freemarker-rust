//! 模板特性、空白剥离与 Java 兼容性测试。

use super::parse;
use crate::core::{ElementKind, Expr, ExprKind};
use crate::span::Span;
use crate::template::{Configuration, Template};
use crate::value::TNumber;
use std::rc::Rc;

fn cfg() -> Rc<Configuration> {
    Rc::new(Configuration::new())
}

fn cfg_strict() -> Rc<Configuration> {
    let mut c = Configuration::new();
    c.settings.strict_syntax = true;
    Rc::new(c)
}

fn parse_with(cfg: &Rc<Configuration>, src: &str) -> Template {
    parse(cfg, "t", src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"))
}

fn parse_ok(src: &str) -> Template {
    parse(&cfg(), "t", src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"))
}

#[allow(dead_code)]
fn parse_err(src: &str) -> String {
    match parse(&cfg(), "t", src) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected parse error for {src:?}"),
    }
}

fn num(v: TNumber) -> ExprKind {
    ExprKind::Num(v)
}

#[allow(dead_code)]
fn ident(n: &str) -> ExprKind {
    ExprKind::Ident(n.to_string())
}

#[allow(dead_code)]
fn strlit(s: &str) -> ExprKind {
    ExprKind::Str(s.to_string())
}

#[test]
fn comments_and_special_text() {
    let t = parse_ok("a<#-- comment -->b");
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
    assert!(matches!(t.root[1].kind, ElementKind::Comment { ref text } if text == " comment "));
    assert!(matches!(t.root[2].kind, ElementKind::Text { ref text, .. } if text == "b"));

    // 多行注释
    let t = parse_ok("<#--\nmulti\nline\n-->x");
    assert!(matches!(t.root[0].kind, ElementKind::Comment { ref text } if text.contains("multi")));

    // <#comment> 块（NO_PARSE 内容原样保留）
    let t = parse_ok("<#comment>raw <#if>x</#if></#comment>x");
    assert!(
        matches!(t.root[0].kind, ElementKind::Comment { ref text } if text == "raw <#if>x</#if>")
    );

    // t / nt / lt / rt：TrimInstruction 解析期消费后即被移除
    // （Java TrimInstruction.isIgnorable=true → postParseCleanup 移除，渲染期 no-op）
    let t = parse_ok("<#t>");
    assert!(
        t.root.is_empty(),
        "TrimInstruction removed from the tree after parse"
    );
    let t = parse_ok("<#nt>");
    assert!(t.root.is_empty());
    // <#lt> 是左裁剪标记（Java TrimInstruction(true,false)），非字面 "<"
    let t = parse_ok("<#lt>");
    assert!(t.root.is_empty());
    let t = parse_ok("<#rt>");
    assert!(t.root.is_empty());
    // Java 对 `<#gt>` 报 Unknown directive（gt 非内置指令名；Java 的 `&gt;` 是
    // 表达式转义，非指令）——词法层对齐后为解析错误
    let msg = parse_err("<#gt>");
    assert!(msg.contains("Unknown directive: #gt"), "{msg}");
    let t = parse_ok("<#noparse>${x} ${y}</#noparse>");
    assert!(matches!(t.root[0].kind, ElementKind::NoParse { ref text, .. } if text == "${x} ${y}"));
}

#[test]
fn ftl_header() {
    // 角度语法头部
    let t = parse_ok(r#"<#ftl encoding="UTF-8">hello"#);
    assert_eq!(t.encoding.as_deref(), Some("UTF-8"));
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "hello"));

    // 方括号语法头部（含换行吞除）
    let t = parse_ok("[#ftl]\nhello");
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "hello"));

    // 头部只允许在模板开头
    let msg = parse_err("x<#ftl>");
    assert!(msg.contains("#ftl header is only allowed"), "{msg}");
}

// -----------------------------------------------------------------------
// 词法规则（docs/03 §2.3）
// -----------------------------------------------------------------------

#[test]
fn angle_bracket_is_text() {
    // `a < b` 是文本（非严格与严格语法）
    let t = parse_ok("a < b");
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a < b"));
    let t = parse_with(&cfg_strict(), "a < b");
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a < b"));
    // 非指令名标签是文本
    let t = parse_ok("a <b> c");
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a <b> c"));
}

#[test]
fn dollar_escape_and_interpolation() {
    // `$${` → 文本 $ + 插值
    let t = parse_ok("$${x}");
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "$"));
    assert!(matches!(t.root[1].kind, ElementKind::Interpolation { .. }));
    // `$` 后非 `{` 为文本
    let t = parse_ok("$x");
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "$x"));
    // `${expr}` 与 `#{expr}` 插值
    let t = parse_ok("a${x}b#{y}c");
    assert_eq!(t.root.len(), 5);
    assert!(matches!(t.root[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
    assert!(matches!(t.root[1].kind, ElementKind::Interpolation { .. }));
    assert!(matches!(t.root[2].kind, ElementKind::Text { ref text, .. } if text == "b"));
    assert!(matches!(t.root[3].kind, ElementKind::Interpolation { .. }));
    assert!(matches!(t.root[4].kind, ElementKind::Text { ref text, .. } if text == "c"));
}

#[test]
fn square_bracket_syntax() {
    let t = parse_ok("[#if x]y[/#if]");
    let ElementKind::If { then, .. } = &t.root[0].kind else {
        panic!("expected If, got {:?}", t.root[0].kind);
    };
    assert!(matches!(then[0].kind, ElementKind::Text { ref text, .. } if text == "y"));

    // 角度语法确立后 `[#` 是文本（Java STATIC_TEXT 语义）
    let t = parse_ok("a<#if x>b</#if>[#if y]c[/#if]");
    assert_eq!(t.root.len(), 3);
    assert!(matches!(t.root[2].kind, ElementKind::Text { ref text, .. } if text.contains("[#if")));
}

#[test]
fn expression_comments() {
    let t = parse_ok("${1 + [#-- c --] 2}");
    let ElementKind::Interpolation { expr, .. } = &t.root[0].kind else {
        panic!("expected Interpolation");
    };
    assert_eq!(
        expr.kind,
        ExprKind::Add(
            Box::new(Expr::new(num(TNumber::Int(1)), Span::new(1, 3))),
            Box::new(Expr::new(num(TNumber::Int(2)), Span::new(1, 18))),
        )
    );
}

// -----------------------------------------------------------------------
// 解析错误（位置 + 期望内容）
// -----------------------------------------------------------------------

#[test]
fn error_positions() {
    // 未闭合标签（Java 2.3.34 ParseException.getMessage 格式，jar 实测——
    // EOF 列=输入末尾（列 7），消息为 unclosed 格式）
    let msg = parse_err("<#if x>");
    assert!(
        msg.contains("Syntax error in template \"t\" in line 1, column 7:"),
        "{msg}"
    );
    assert!(msg.contains("You have an unclosed #if"), "{msg}");

    // 多行模板的行号（EOF 列 = 输入末尾字符列）
    let msg = parse_err("a\nb\n<#if x>");
    assert!(msg.contains("in line 3, column 7"), "{msg}");

    // 未闭合插值（EOF 列 = 长度；unclosed "{"）
    let msg = parse_err("${x");
    assert!(msg.contains("in line 1, column 3"), "{msg}");
    assert!(msg.contains("You have an unclosed \"{\""), "{msg}");

    // 未闭合注释（Java：Unclosed "<#--"，位置 = 注释 token 起始）
    let msg = parse_err("<#-- unclosed");
    assert!(msg.contains("Unclosed \"<#--\""), "{msg}");

    // 非法字符（`@` 是合法标识符起始——Java isLegacyFTLIdStartChar 的 `@`..`Z`
    // 区间；`${a @}` 为相邻标识符 → 插值未闭合错误）
    let msg = parse_err("${a @}");
    assert!(msg.contains("to close the interpolation"), "{msg}");

    // 不匹配的结束标签（JavaCC 嵌套错误格式）
    let msg = parse_err("<#if x></#list>");
    assert!(msg.contains("Encountered \"</#list>\""), "{msg}");
    assert!(msg.contains("this can be closed: \"#if\""), "{msg}");

    // 自闭合块指令（Rust 自闭合检查消息；Java 无对齐基线场景）
    let msg = parse_err("<#if x/>");
    assert!(msg.contains("self-closing"), "{msg}");

    // 未知指令
    let msg = parse_err("<#nosuchdir>");
    assert!(msg.contains("Unknown directive: #nosuchdir"), "{msg}");

    // 孤立的 <#else>
    let msg = parse_err("<#else>");
    assert!(msg.contains("Unexpected directive <#else>"), "{msg}");
}

// -----------------------------------------------------------------------
// 空白剥离标记（docs/08 §5.2；对照 Java TextBlock.postParseCleanup）
// -----------------------------------------------------------------------

#[test]
fn whitespace_stripping_flags() {
    // 剥离在解析期直接改写文本（Java TextBlock.postParseCleanup 的 text = substring
    // 语义，TextBlock.java:128；strip_before/strip_after 标记恒 false）
    // 行首空白 + FTL 标签行 → 剥到首个换行（含）为止
    let t = parse_ok("A\n<#if x>\nB\n</#if>\nC");
    let ElementKind::If { then, .. } = &t.root[1].kind else {
        panic!("expected If");
    };
    let ElementKind::Text { text, .. } = &then[0].kind else {
        panic!("expected Text in then");
    };
    assert_eq!(text, "B\n", "leading newline after <#if> stripped at parse");
    let ElementKind::Text { text, .. } = &t.root[2].kind else {
        panic!("expected Text after if");
    };
    assert_eq!(text, "C", "leading newline after </#if> stripped at parse");

    // 前一同行文本有内容 → 不剥离（Java heedsOpeningWhitespace）
    let t = parse_ok("x<#if y>  \nz</#if>");
    let ElementKind::If { then, .. } = &t.root[1].kind else {
        panic!("expected If");
    };
    let ElementKind::Text { text, .. } = &then[0].kind else {
        panic!("expected Text");
    };
    assert_eq!(text, "  \nz", "same-line previous text blocks stripping");

    // 尾部空白：块后无内容 → 剥离
    let t = parse_ok("<#if y>foo\n  </#if>");
    let ElementKind::If { then, .. } = &t.root[0].kind else {
        panic!("expected If");
    };
    let ElementKind::Text { text, .. } = &then[0].kind else {
        panic!("expected Text");
    };
    assert_eq!(
        text, "foo\n",
        "trailing whitespace of last block text stripped"
    );

    // 尾部空白：同行的下一文本有内容 → 不剥离
    let t = parse_ok("<#if y>foo\n  </#if>bar");
    let ElementKind::If { then, .. } = &t.root[0].kind else {
        panic!("expected If");
    };
    let ElementKind::Text { text, .. } = &then[0].kind else {
        panic!("expected Text");
    };
    assert_eq!(text, "foo\n  ", "same-line following text blocks stripping");

    // 模板首文本不剥（Java 守卫）
    let t = parse_ok("  \n<#if x>y</#if>");
    let ElementKind::Text { text, .. } = &t.root[0].kind else {
        panic!("expected Text");
    };
    assert_eq!(text, "  \n", "first root text never stripped");

    // <#t> 显式裁剪 / <#nt> 显式取消
    let t = parse_ok("<#if y>a\n  <#t></#if>");
    let ElementKind::If { then, .. } = &t.root[0].kind else {
        panic!("expected If");
    };
    // Java deliberateLeftTrim：<#t> 显式裁剪最后一行前导（"  " 全空白 → 裁掉）
    let ElementKind::Text { text, .. } = &then[0].kind else {
        panic!("expected Text");
    };
    assert_eq!(text, "a\n", "<#t> trims the trailing blank line");
    let t = parse_ok("<#if y>a\n  <#nt></#if>");
    let ElementKind::If { then, .. } = &t.root[0].kind else {
        panic!("expected If");
    };
    let ElementKind::Text { text, .. } = &then[0].kind else {
        panic!("expected Text");
    };
    assert_eq!(text, "a\n  ", "<#nt> prevents stripping the preceding text");
}

#[test]
fn stripping_off_when_disabled() {
    // whitespace_stripping=false → 无标记
    let mut c = Configuration::new();
    c.settings.whitespace_stripping = false;
    let cfg = Rc::new(c);
    let t = parse_with(&cfg, "A\n<#if x>\nB\n</#if>\nC");
    let ElementKind::If { then, .. } = &t.root[1].kind else {
        panic!("expected If");
    };
    let ElementKind::Text { strip_before, .. } = &then[0].kind else {
        panic!("expected Text");
    };
    assert!(!*strip_before);
}

// -----------------------------------------------------------------------
// Java 测试套件真实模板冒烟解析（include_str! 嵌入）
// -----------------------------------------------------------------------

#[test]
fn java_suite_helloworld() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/helloworld.ftl"
    )));
    assert!(matches!(t.root[0].kind, ElementKind::Comment { .. }));
    assert!(
        matches!(t.root[1].kind, ElementKind::Text { ref text, .. } if text.contains("<html>"))
    );
}

#[test]
fn java_suite_escapes() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/escapes.ftl"
    )));
    // <#escape> 块 + <#noescape> 块
    assert!(
        t.root
            .iter()
            .any(|e| matches!(e.kind, ElementKind::Escape { .. })),
        "expected an #escape block in escapes.ftl"
    );
}

#[test]
fn java_suite_if() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/if.ftl"
    )));
    assert!(!t.root.is_empty());
}

#[test]
fn java_suite_boolean() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/boolean.ftl"
    )));
    assert!(!t.root.is_empty());
}

#[test]
fn java_suite_comment() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/comment.ftl"
    )));
    assert!(!t.root.is_empty());
}

#[test]
fn java_suite_lastcharacter() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/lastcharacter.ftl"
    )));
    assert!(!t.root.is_empty());
}

#[test]
fn java_suite_default() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/default.ftl"
    )));
    assert!(!t.root.is_empty());
}

#[test]
fn java_suite_localization() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/localization.ftl"
    )));
    assert!(!t.root.is_empty());
}

#[test]
fn java_suite_boolean_formatting() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/boolean-formatting.ftl"
    )));
    assert!(!t.root.is_empty());
}

#[test]
fn java_suite_include() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/include.ftl"
    )));
    assert!(!t.root.is_empty());
}

#[test]
fn java_suite_import() {
    let t = parse_ok(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite/templates/import.ftl"
    )));
    assert!(!t.root.is_empty());
}
#[test]
fn probe_atat_markup_parse() {
    let cfg = Rc::new(Configuration::default());
    let t = parse(&cfg, "t.ftl", "${doc.@@markup}");
    assert!(t.is_ok(), "doc.@@markup should parse: {:?}", t.err());
}

#[test]
fn probe_recurse_parse() {
    let cfg = Rc::new(Configuration::default());
    let t = parse(
        &cfg,
        "t.ftl",
        "<#recurse doc >\n<#recurse .node.title>\n<#recurse>",
    );
    assert!(t.is_ok(), "recurse should parse: {:?}", t.err());
}
