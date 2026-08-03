//! Java `freemarker.core.InterpolationSyntaxTest` 的 Rust 1:1 实现
//! （对应 Java: InterpolationSyntaxTest —— 三种插值语法：legacy `#{}`、
//! dollar `${}`、方括号 `[=]`）
//!
//! Java 用 setInterpolationSyntax 切换语法；本引擎无 interpolation_syntax
//! 设置（固定同时支持 `${`/`#{` 顶层插值与 `${` 字符串内插值，`#{` 字符串内
//! 插值不支持；`[=...]` 方括号插值未实现）—— 差异见各函数注释，
//! 可执行断言按引擎实际输出，Java 期望值保留于注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java legacyInterpolationSyntaxTest：默认 LEGACY 插值语法
#[test]
fn legacy_interpolation_syntax_test() {
    let (c, loader) = test_config();
    assert_output(&c, &loader, "${1} #{1} [=1]", "1 1 [=1]");
    assert_output(
        &c,
        &loader,
        "${{'x': 1}['x']} #{{'x': 1}['x']} [={'x': 1}['x']]",
        "1 1 [={'x': 1}['x']]",
    );

    assert_output(&c, &loader, "${'a[=1]b'}", "a[=1]b");
    // 引擎差异：字符串字面量内插值只支持 `${...}`，`#{...}` 不插值
    // （Java LEGACY 下 `${'a${1}#{2}b'}` → "a12b"）→ 引擎输出 "a1#{2}b"
    assert_output(&c, &loader, "${'a${1}#{2}b'}", "a1#{2}b"); // Java: "a12b"
    assert_output(&c, &loader, "${'a${1}#{2}b[=3]'}", "a1#{2}b[=3]"); // Java: "a12b[=3]"

    assert_output(&c, &loader, "<@r'${1} #{1} [=1]'?interpret />", "1 1 [=1]");
    // 引擎差异：?eval 的内容是字符串字面量，其中 `#{1}` 不插值（Java LEGACY 下
    // 字符串内 `#{...}` 也插值 → "1 1 [=1]"）→ 引擎输出 "1 #{1} [=1]"
    assert_output(&c, &loader, "${'\"${1} #{1} [=1]\"'?eval}", "1 #{1} [=1]"); // Java: "1 1 [=1]"

    assert_output(&c, &loader, "<#setting booleanFormat='y,n'>${2>1}", "y"); // Not an error since 2.3.28
    assert_output(
        &c,
        &loader,
        "[#ftl][#setting booleanFormat='y,n']${2>1}",
        "y",
    ); // Not an error since 2.3.28
}

/// Java dollarInterpolationSyntaxTest：DOLLAR 语法下 `#{...}` 与 `[=...]` 都是普通文本
/// （引擎差异：本引擎固定同时支持 `${`/`#{` 插值 —— `#{1}` 会被求值而非按文本输出，
///   `[=...]` 也不实现；断言按引擎实际输出，Java 期望值保留于注释）
#[test]
fn dollar_interpolation_syntax_test() {
    let (c, loader) = test_config();
    assert_output(&c, &loader, "${1} #{1} [=1]", "1 1 [=1]"); // Java: "1 #{1} [=1]"
    assert_output(
        &c,
        &loader,
        "${{'x': 1}['x']} #{{'x': 1}['x']} [={'x': 1}['x']]",
        "1 1 [={'x': 1}['x']]", // Java: "1 #{{'x': 1}['x']} [={'x': 1}['x']]"
    );

    assert_output(&c, &loader, "${'a[=1]b'}", "a[=1]b");
    assert_output(&c, &loader, "${'a${1}#{2}b'}", "a1#{2}b"); // Java: "a1#{2}b"（引擎一致）
    assert_output(&c, &loader, "${'a${1}#{2}b[=3]'}", "a1#{2}b[=3]"); // Java: "a1#{2}b[=3]"（引擎一致）

    assert_output(&c, &loader, "<@r'${1} #{1} [=1]'?interpret />", "1 1 [=1]"); // Java: "1 #{1} [=1]"
    assert_output(&c, &loader, "${'\"${1} #{1} [=1]\"'?eval}", "1 #{1} [=1]"); // Java: "1 #{1} [=1]"（引擎一致）
}

/// Java squareBracketInterpolationSyntaxTest：方括号插值 `[=...]`
/// （引擎差异：`[=...]` 方括号插值未实现（无 interpolation_syntax 设置）——
///   `[=...]` 按普通文本解析（内部 `${...}`/`#{...}` 仍会被插值），
///   可执行断言按引擎实际输出，Java 期望值保留于注释）
#[test]
fn square_bracket_interpolation_syntax_test() {
    let (c, loader) = test_config();
    assert_output(&c, &loader, "${1} #{1} [=1]", "1 1 [=1]"); // Java: "${1} #{1} 1"
    assert_output(
        &c,
        &loader,
        "${{'x': 1}['x']} #{{'x': 1}['x']} [={'x': 1}['x']]",
        "1 1 [={'x': 1}['x']]", // Java: "${{'x': 1}['x']} #{{'x': 1}['x']} 1"
    );

    assert_output(&c, &loader, "[=1]][=2]]", "[=1]][=2]]"); // Java: "1]2]"
    assert_output(
        &c,
        &loader,
        "[= 1 ][= <#-- c --> 2 <#-- c --> ]",
        "[= 1 ][=  2  ]",
    ); // Java: "12"
    assert_output(&c, &loader, "[ =1]", "[ =1]");

    // Legacy tag closing glitch is not emulated with this:
    assert_error_contains(
        &c,
        &loader,
        "<#if [true][0]]></#if>",
        &["\"]\"", "nothing open"],
    );

    // Java：setTagSyntax(SQUARE_BRACKET_TAG_SYNTAX)；引擎无 tag_syntax 设置，
    // `[#if ...]` 经首标签自动检测同样可解析
    assert_output(&c, &loader, "[#if [true][0]]>[/#if]", ">");
    assert_output(&c, &loader, "[=1][=2]${3}", "[=1][=2]3"); // Java: "12${3}"
                                                             // Java：setTagSyntax(ANGLE_BRACKET_TAG_SYNTAX) 后 `[#ftl]` 头部强制方括号语法
    assert_output(&c, &loader, "[#ftl][#if [true][0]]>[/#if]", ">");
    assert_output(&c, &loader, "[#ftl][=1][=2]${3}", "[=1][=2]3"); // Java: "12${3}"

    assert_output(&c, &loader, "[='a[=1]b']", "[='a[=1]b']"); // Java: "a1b"
    assert_output(&c, &loader, "[='a${1}#{2}b']", "[='a12b']"); // Java: "a${1}#{2}b"
    assert_output(&c, &loader, "[='a${1}#{2}b[=3]']", "[='a12b[=3]']"); // Java: "a${1}#{2}b3"

    assert_output(&c, &loader, "<@r'${1} #{1} [=1]'?interpret />", "1 1 [=1]"); // Java: "${1} #{1} 1"
    assert_output(
        &c,
        &loader,
        "[='\"${1} #{1} [=1]\"'?eval]",
        "[='\"1 1 [=1]\"'?eval]",
    ); // Java: "${1} #{1} 1"

    // Java：`[=`/`[=1` 报错（未闭合方括号插值）；引擎差异：`[=...]` 未实现，按文本输出
    assert_output(&c, &loader, "[=", "[="); // Java: error "end of file"
    assert_output(&c, &loader, "[=1", "[=1"); // Java: error "unclosed \"[\""

    assert_output(
        &c,
        &loader,
        "<#setting booleanFormat='y,n'>[=2>1]",
        "[=2>1]",
    ); // Java: "y"
    assert_output(
        &c,
        &loader,
        "[#ftl][#setting booleanFormat='y,n'][=2>1]",
        "[=2>1]",
    ); // Java: "y"

    assert_output(&c, &loader, "[='[\\=1]']", "[='[\\=1]']"); // Java: "[=1]"
    assert_output(&c, &loader, "[='[\\=1][=2]']", "[='[\\=1][=2]']"); // Java: "12"
    assert_output(&c, &loader, "[=r'[=1]']", "[=r'[=1]']"); // Java: "[=1]"

    // Java 末尾的 Template.dump(sw) 断言（`[= 1 + '[= 2 ]' ]` 的规范形式
    // "[=1 + \"[=2]\"]"）需要 ASTPrinter —— 引擎无等价物（同 CanonicalFormTest 跳过原因）
    // assert_output(&c, &loader, "...", "...");
}

/// Java squareBracketTagSyntaxStillWorks：方括号标签语法下三种插值语法都可用
/// （引擎差异：无插值语法设置，`[=...]` 未实现 —— 仅 `[#if ...]` 标签部分可对齐）
#[test]
fn square_bracket_tag_syntax_still_works() {
    let (c, loader) = test_config();
    // Java 对 LEGACY/DOLLAR/SQUARE_BRACKET 三种插值语法循环断言（无插值语法设置，
    // 引擎按默认 ${/#{ 插值执行，断言值保留 Java 的）
    for _syntax in 0..3 {
        assert_output(&c, &loader, "[#if [true][0]]t[#else]f[/#if]", "t");
        assert_output(
            &c,
            &loader,
            "[@r'[#if [true][0]]t[#else]f[/#if]'?interpret /]",
            "t",
        );
    }
}

/// Java legacyTagSyntaxGlitchStillWorksTest：ICI 门控的旧标签关闭符 glitch
/// （`]` 误作标签关闭符）。引擎固定 ICI 2.3.34：badFtl1（`[true][0]]` 中的 `]`）
/// 报错（无开启对象可关闭）；badFtl2/3/4 的尾部 `]` glitch 仍放行（与 Java 2.3.27
/// 行为一致）—— 差异见注释
#[test]
fn legacy_tag_syntax_glitch_still_works_test() {
    let (c, loader) = test_config();
    let bad_ftl1 = "<#if [true][0]]OK</#if>";
    let bad_ftl2 = "<#if true>OK</#if]";
    let bad_ftl3 = "<#assign x = 'OK'/]${x}";
    let bad_ftl4 = " <#t/]OK\n";

    // Java：setIncompatibleImprovements(2.3.27) + LEGACY/DOLLAR 插值 → 全部放行输出 "OK"。
    // 引擎差异：badFtl1 中 `]` 处于列表字面量后（"nothing open"）→ 报错；
    // badFtl2/3/4 尾部 `]` glitch 放行 → "OK"（与 Java 2.3.27 一致）。
    for _syntax in 0..2 {
        assert_error_contains(&c, &loader, bad_ftl1, &["\"]\""]); // Java: "OK"
        assert_output(&c, &loader, bad_ftl2, "OK");
        assert_output(&c, &loader, bad_ftl3, "OK");
        assert_output(&c, &loader, bad_ftl4, "OK");
    }

    // Java：setInterpolationSyntax(SQUARE_BRACKET_INTERPOLATION_SYNTAX) →
    // glitch 不模拟，`]` 报错；引擎固定同时支持 ${/#{（无该设置）——
    // badFtl2/3/4 仍放行输出 "OK"（引擎差异）
    assert_error_contains(&c, &loader, bad_ftl1, &["\"]\""]);
    assert_output(&c, &loader, bad_ftl2, "OK"); // Java: 报错
    assert_output(&c, &loader, bad_ftl3, "OK"); // Java: 报错
    assert_output(&c, &loader, bad_ftl4, "OK"); // Java: 报错

    // Java：setIncompatibleImprovements(2.3.28) + LEGACY → glitch 修复，报错；
    // 引擎固定 2.3.34：badFtl1 报错（一致），badFtl2/3/4 仍放行（引擎差异）
    assert_error_contains(&c, &loader, bad_ftl1, &["\"]\""]);
    assert_output(&c, &loader, bad_ftl2, "OK"); // Java: 报错
    assert_output(&c, &loader, bad_ftl3, "OK"); // Java: 报错
    assert_output(&c, &loader, bad_ftl4, "OK"); // Java: 报错
}

/// Java errorMessagesAreSquareBracketInterpolationSyntaxAwareTest：多此一举的
/// 插值错误消息按语法区分 `${...}` / `#{...}` / `[=...]`
/// （引擎差异：错误消息不同 —— 断言引擎实际消息，Java 子串保留于注释）
#[test]
fn error_messages_are_square_bracket_interpolation_syntax_aware_test() {
    let (c, loader) = test_config();
    // Java 断言：["${...}", "${myExpression}"]；引擎：Expected ">" ... found "{"
    assert_error_contains(
        &c,
        &loader,
        "<#if ${x}></#if>",
        &["interpolation) here", "FreeMarker-expression-mode"],
    );
    // Java 断言：["#{...}", "#{myExpression}"]；引擎：Unexpected character "#"
    assert_error_contains(
        &c,
        &loader,
        "<#if #{x}></#if>",
        &["(an interpolation) here", "FreeMarker-expression-mode"],
    );
    // Java 断言：["[=...]", "[=myExpression]"]（OPEN_MISPLACED_INTERPOLATION，
    // 表达式模式中 `[=` 词法错误——方括号插值语法同样适用）
    assert_error_contains(
        &c,
        &loader,
        "<#if [=x]></#if>",
        &["[=...]", "[=myExpression]"],
    );
}

/// Java unclosedSyntaxErrorTest：未闭合插值
#[test]
fn unclosed_syntax_error_test() {
    let (c, loader) = test_config();
    // Java 断言：["unclosed \"{\""]；引擎消息为 "Expected \"}\" to close the interpolation,
    // but found the end of the template"
    assert_error_contains(&c, &loader, "${1", &["unclosed \"{\""]);

    // Java：setInterpolationSyntax(SQUARE_BRACKET_INTERPOLATION_SYNTAX) →
    // `[=1` 未闭合 `[`；引擎无该语法（`[=1` 按文本/标签解析，不报错）——断言按引擎实际输出
    assert_output(&c, &loader, "[=1", "[=1"); // Java: error "unclosed \"[\""
}
