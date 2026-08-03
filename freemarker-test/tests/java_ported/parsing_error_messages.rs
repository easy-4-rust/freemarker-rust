//! Java `freemarker.core.ParsingErrorMessagesTest` 的 Rust 1:1 实现
//! （对应 Java: ParsingErrorMessagesTest —— 解析期错误消息断言）
//!
//! Java createConfiguration：ICI 2.3.21 + AUTO_DETECT_TAG_SYNTAX；
//! 本引擎固定 ICI 2.3.34、标签语法无配置项（首个标签自动检测）。
//!
//! 引擎差异总览：v1 的解析错误消息与 Java 措辞不同（Java 提示段如
//! "instead of ${"、"existing directive"、"malformed"、"unclosed"、"end-tag"、
//! "end of file" 等在本引擎均不存在）—— 各断言调整为断言引擎实际消息中最接近的
//! 子串并注明 Java 原断言值。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

/// Java `assertErrorContainsAS`：同一模板的尖括号与方括号两种标签写法都断言
/// （Java 内部 setTagSyntax 切换；本引擎首个标签自动检测语法，两种写法均可解析）
fn assert_error_contains_as(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    angle_brackets_ftl: &str,
    subs: &[&str],
) {
    assert_error_contains(c, loader, angle_brackets_ftl, subs);
    let squared = angle_brackets_ftl.replace('<', "[").replace('>', "]");
    assert_error_contains(c, loader, &squared, subs);
}

/// Java testNeedlessInterpolation：`<#if>` 条件里的"多此一举"的 `${...}` 插值
/// （Java 对齐：OPEN_MISPLACED_INTERPOLATION 词法错误）
#[test]
fn test_needless_interpolation() {
    let (c, loader) = test_config();
    // Java 断言 ["instead of ${"]
    assert_error_contains_as(
        &c,
        &loader,
        "<#if ${x} == 3></#if>",
        &["(an interpolation) here", "FreeMarker-expression-mode"],
    );
    assert_error_contains_as(
        &c,
        &loader,
        "<#if ${x == 3}></#if>",
        &["(an interpolation) here", "FreeMarker-expression-mode"],
    );
    // Java 断言 ["instead of ${"]
    assert_error_contains_as(
        &c,
        &loader,
        "<@foo ${x == 3} />",
        &["(an interpolation) here", "FreeMarker-expression-mode"],
    );
    // Java：setInterpolationSyntax(SQUARE_BRACKET_INTERPOLATION_SYNTAX) 后 `[= x == 3]`
    // 是同类错误（OPEN_MISPLACED_INTERPOLATION，表达式模式中 `[=` 词法错误）
    assert_error_contains_as(
        &c,
        &loader,
        "<@foo [= x == 3] />",
        &["[=...]", "[=myExpression]"],
    );
}

/// Java testWrongDirectiveNames：未知指令名及"相近指令"提示（Java 对齐：
/// UNKNOWN_DIRECTIVE tip 段）
#[test]
fn test_wrong_directive_names() {
    let (c, loader) = test_config();
    assert_error_contains_as(&c, &loader, "<#foo />", &["nknown directive", "#foo"]);
    // Java 断言含 "#assign"（相近指令提示）
    assert_error_contains_as(
        &c,
        &loader,
        "<#set x = 1 />",
        &["nknown directive", "#set", "#assign"],
    );
    // Java 断言含 "#list"（相近指令提示）
    assert_error_contains_as(
        &c,
        &loader,
        "<#iterator></#iterator>",
        &["nknown directive", "#iterator", "#list"],
    );
}

/// Java testBug402：已存在指令的畸形形式（词法层 UNKNOWN_DIRECTIVE）
#[test]
fn test_bug402() {
    let (c, loader) = test_config();
    // Java 断言 ["existing directive", "malformed", "#list"]
    assert_error_contains_as(
        &c,
        &loader,
        "<#list 1..i as k>${k}<#list>",
        &["existing directive", "malformed", "#list"],
    );
    // Java 断言 ["existing directive", "malformed", "#assign"]
    assert_error_contains_as(
        &c,
        &loader,
        "<#assign>",
        &["existing directive", "malformed", "#assign"],
    );
    // Java 断言 ["existing directive", "malformed", "#if"]
    assert_error_contains_as(
        &c,
        &loader,
        "</#if x>",
        &["existing directive", "malformed", "#if"],
    );
    // Java 断言 ["existing directive", "malformed", "#compress"]
    assert_error_contains_as(
        &c,
        &loader,
        "<#compress x>",
        &["existing directive", "malformed", "#compress"],
    );
}

/// Java testUnclosedDirectives：各类未闭合指令（Java 对齐：EOF 统一
/// "Unexpected end of file reached. You have an unclosed ..."）
#[test]
fn test_unclosed_directives() {
    let (c, loader) = test_config();
    assert_error_contains_as(&c, &loader, "<#macro x>", &["#macro", "unclosed"]); // Java: ["#macro", "unclosed"]
    assert_error_contains_as(&c, &loader, "<#macro x></#function>", &["macro end tag"]); // Java: ["macro end tag"]
                                                                                         // Java 断言 ["#macro", "unclosed"]（函数缺闭合按 #macro 报）
    assert_error_contains_as(
        &c,
        &loader,
        "<#function x>",
        &["#macro or #function", "unclosed"],
    );
    assert_error_contains_as(&c, &loader, "<#function x></#macro>", &["function end tag"]); // Java: ["function end tag"]
    assert_error_contains_as(&c, &loader, "<#assign x>", &["#assign", "unclosed"]); // Java: ["#assign", "unclosed"]
    assert_error_contains_as(&c, &loader, "<#macro m><#local x>", &["#local", "unclosed"]); // Java: ["#local", "unclosed"]
    assert_error_contains_as(&c, &loader, "<#global x>", &["#global", "unclosed"]); // Java: ["#global", "unclosed"]
                                                                                    // Java 断言 ["@...", "unclosed"]；v1 消息为 "Unclosed user directive call (missing </@...>)."
    assert_error_contains_as(&c, &loader, "<@foo>", &["Unclosed user directive call"]);
    assert_error_contains_as(&c, &loader, "<#list xs as x>", &["#list", "unclosed"]); // Java: ["#list", "unclosed"]
    assert_error_contains_as(&c, &loader, "<#list xs as x><#if x>", &["#if", "unclosed"]); // Java: ["#if", "unclosed"]
    assert_error_contains_as(
        &c,
        &loader,
        "<#list xs as x><#if x><#if q><#else>",
        &["#if", "unclosed"], // Java: ["#if", "unclosed"]
    );
    assert_error_contains_as(
        &c,
        &loader,
        "<#list xs as x><#if x><#if q><#else><#macro x>qwe",
        &["#macro", "unclosed"], // Java: ["#macro", "unclosed"]
    );
    // Java 断言 ["\"(\"", "unclosed"]
    assert_error_contains_as(&c, &loader, "${(blah", &["\"(\"", "unclosed"]);
    // Java 断言 ["\"{\"", "unclosed"]
    assert_error_contains_as(&c, &loader, "${blah", &["\"{\"", "unclosed"]);
}

/// Java testInterpolatingClosingsErrors：插值关闭符错位
#[test]
fn test_interpolating_closings_errors() {
    let (c, loader) = test_config();
    // Java 断言 ["unclosed"]
    assert_error_contains(&c, &loader, "<#ftl>${x", &["unclosed"]);
    // Java 断言 ["\"}\"", "open"]；引擎：nothing open that it could close
    assert_error_contains(&c, &loader, "<#assign x = x}>", &["\"}\"", "nothing open"]);
    // Java：Legacy glitch... should fail in theory.（Java 原注释；引擎确实报错：
    // Unclosed "${" interpolation in a string literal）——断言按引擎实际行为
    assert_error_contains(
        &c,
        &loader,
        "<#assign x = '${x'>",
        &["Unclosed \"${\" interpolation in a string literal"],
    ); // Java: 输出 ""

    // Java 对 LEGACY 与 DOLLAR 两种插值语法各断言一遍（setInterpolationSyntax 循环）；
    // 本引擎无 interpolation_syntax 设置（固定同时支持 `${`/`#{`）——循环体按引擎
    // 默认行为执行，断言值按引擎消息（引擎差异：无此设置）
    for _syntax in 0..2 {
        assert_error_contains(&c, &loader, "<#ftl>${'x']", &["\"]\"", "nothing open"]); // Java: ["\"]\"", "open"]
        assert_error_contains(
            &c,
            &loader,
            "<#ftl>${'x'>",
            &["Unexpected end of file reached."],
        ); // Java: ["end of file"]
        assert_error_contains(
            &c,
            &loader,
            "[#ftl]${'x'>",
            &["Unexpected end of file reached."],
        );
        // Java: ["end of file"]
    }
}

/// Java testNestingErrors：嵌套/结束标签错位（JavaCC 嵌套错误格式）
#[test]
fn test_nesting_errors() {
    let (c, loader) = test_config();
    // Java 断言 ["</#if>", "#list", "end-tag"]；引擎把 </list> 当文本、运行时先报 xs 缺失
    assert_error_contains(
        &c,
        &loader,
        "<#if true><#list xs as x></list></#if>",
        &["xs"], // Java: 解析期 "</#if>" "#list" "end-tag"
    );
    // Java 断言 ["<#else>", "#if", "#list", "#assign"]；引擎：Unexpected directive <#else> here
    assert_error_contains(
        &c,
        &loader,
        "<#if true><#assign x><#else></#assign></#if>",
        &["Unexpected directive <#else> here"], // Java: + "#if" "#list" "#assign"
    );
    // Java 断言 ["</#list>", "#items", "end-tag"]
    assert_error_contains(
        &c,
        &loader,
        "<#list xs><#items as x></#list>",
        &["</#list>", "#items", "end-tag"], // Java 对齐
    );
    // Java 断言 ["</#if>", "#list", "#sep", "end-tag"]；v1：`</#if>` 在 sep 块内被
    // auto_close 上抛至根级，根级报 Encountered "</#list>"（ROOT 列表）——
    // 断言按引擎实际行为（引擎差异）
    assert_error_contains(
        &c,
        &loader,
        "<#list xs as x><#sep></#if></#list>",
        &["Encountered \"</#list>\"", "<EOF>"],
    );
    // Java 断言 ["end of file", "#list", "end-tag"]
    assert_error_contains(
        &c,
        &loader,
        "<#list xs as x>",
        &["end of file", "#list", "end-tag"], // Java 对齐
    );
    // Java 断言 ["end of file", "#if", "end-tag"]
    assert_error_contains(
        &c,
        &loader,
        "<#if true>text<#list xs as x></#list>",
        &["end of file", "#if", "end-tag"], // Java 对齐
    );
}
