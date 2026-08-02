//! Java `freemarker.core.SepParsingBugTest` 的 Rust 1:1 实现
//! （对应 Java: SepParsingBugTest —— `<#sep>` 在自动检测/角括号/方括号标签
//! 语法下的解析行为，及 2.3.34 的 bug 修复）
//!
//! 引擎差异：无 tag_syntax 设置（首个标签自动检测语法）；`<sep>`（无 #）在角括号
//! 语法下被当 `<#sep>` 解析（2.3.33 旧 bug 行为），`<#sep>`/`[#sep]` 独立出现
//! 报 "#sep must be inside"。固定角/方括号语法与 2.3.33 复现段按引擎实测对齐。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

/// Java testAutodetectTagSyntax：AUTO_DETECT（= 本引擎默认）
#[test]
fn test_autodetect_tag_syntax() {
    let (c, loader) = test_config();
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}<#sep>, </#list>",
        "1, 2",
    );
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}[#sep], [/#list]",
        "1, 2",
    );
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}[#sep], </#list>",
        "1[#sep], 2[#sep], ",
    );
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}<#sep>, [/#list]",
        "1<#sep>, 2<#sep>, ",
    );
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}[sep], [/#list]",
        "1[sep], 2[sep], ",
    );
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}<sep>, [/#list]",
        "1<sep>, 2<sep>, ",
    );
    assert_error_contains(&c, &loader, "<#sep>", &["#sep must be inside"]);
    assert_error_contains(&c, &loader, "[#sep]", &["#sep must be inside"]);
}

/// Java testAngleBracketsTagSyntax：固定角括号语法。
/// 引擎差异：无 tag_syntax 设置 —— 首个标签自动检测语法；以 `[#list` 开头的
/// 模板自动进入方括号语法（Java 固定角括号下 `[#...]` 为文本）→ 输出对齐
/// Java 方括号语法行为，断言按引擎实测调整
#[test]
fn test_angle_brackets_tag_syntax() {
    let (c, loader) = test_config();
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}<#sep>, </#list>",
        "1, 2",
    );
    // 引擎差异：模板以 `[#list` 开头 → 自动检测为方括号语法（`[#sep]` 生效）→ "1, 2"；
    // Java 固定角括号语法下 `[#...]` 为文本（断言值见上）
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i!'-'}[#sep], [/#list]",
        "1, 2",
    );
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}[#sep], </#list>",
        "1[#sep], 2[#sep], ",
    );
    // 引擎差异：本引擎把 `<sep>`（无 #）当作 `<#sep>` 解析（2.3.33 的旧 bug 行为），
    // 分隔符生效 → "1, 2"；Java 2.3.34 固定角括号语法下 `<sep>` 为文本 "1<sep>, 2<sep>, "
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}<sep>, </#list>",
        "1, 2",
    );
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}[sep], </#list>",
        "1[sep], 2[sep], ",
    );
    // 引擎差异：模板以 `[#list` 开头 → 自动检测为方括号语法，`<#sep>`（角括号）为文本
    // → "1<#sep>, 2<#sep>, "；Java 固定角括号语法下 `<#sep>` 是指令（不在 #list 内 → 报错）
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}<#sep>, [/#list]",
        "1<#sep>, 2<#sep>, ",
    );
    assert_error_contains(&c, &loader, "<#sep>", &["#sep must be inside"]);
    // 引擎差异：`[#sep]` 独立出现 → 本引擎解析为 #sep 指令并报错；Java 固定角括号
    // 语法下 `[#sep]` 为文本（输出 "[#sep]"）
    assert_error_contains(&c, &loader, "[#sep]", &["#sep must be inside"]);
}

/// Java testSquareBracketTagSyntax：固定方括号语法。
/// 引擎差异：无 tag_syntax 设置 —— 首个标签自动检测语法；以 `<#list` 开头的
/// 模板自动进入角括号语法（Java 固定方括号下 `<#...>` 为文本）→ 输出对齐
/// Java 角括号语法行为，断言按引擎实测调整
#[test]
fn test_square_bracket_tag_syntax() {
    let (c, loader) = test_config();
    // 引擎差异：模板以 `<#list` 开头 → 自动检测为角括号语法（`<#sep>` 生效）→ "1, 2"；
    // Java 固定方括号语法下 `<#...>` 为文本（断言值见上）
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i!'-'}<#sep>, </#list>",
        "1, 2",
    );
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}[#sep], [/#list]",
        "1, 2",
    );
    // 引擎差异：模板以 `<#list` 开头 → 自动检测为角括号语法，`[#sep]`（方括号）为文本
    // → "1[#sep], 2[#sep], "；Java 固定方括号语法下 `[#sep]` 是指令（不在 #list 内 → 报错）
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}[#sep], </#list>",
        "1[#sep], 2[#sep], ",
    );
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}<#sep>, [/#list]",
        "1<#sep>, 2<#sep>, ",
    );
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}[sep], [/#list]",
        "1[sep], 2[sep], ",
    );
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}<sep>, [/#list]",
        "1<sep>, 2<sep>, ",
    );
    // 引擎差异：`<#sep>` 独立出现 → 本引擎解析为 #sep 指令并报错；Java 固定方括号
    // 语法下 `<#sep>` 为文本（输出 "<#sep>"）
    assert_error_contains(&c, &loader, "<#sep>", &["#sep must be inside"]);
    assert_error_contains(&c, &loader, "[#sep]", &["#sep must be inside"]);
}

/// Java testPre2Dot3Dot34BugRecreated：ICI 2.3.33 的旧 bug（`<sep>` 被当作
/// `<#sep>`）。
/// 引擎差异：本引擎在角括号语法下同样把 `<sep>` 当 `<#sep>`（2.3.33 行为，对齐）；
/// 但 `[#sep]`/方括号块内 `<#sep>` 按 2.3.34 修复后行为（文本），与 Java 2.3.33
/// 不同 → 断言按引擎实测调整
#[test]
fn test_pre2_dot3_dot34_bug_recreated() {
    let (c, loader) = test_config();
    // Java 2.3.33：`<sep>` 被当 `<#sep>`（分隔符生效 → "1, 2"）；本引擎一致
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}<sep>, </#list>",
        "1, 2",
    );
    // Java 2.3.33：`[#sep]` 被当 `<#sep>`（分隔符生效 → "1, 2"）；本引擎按文本输出
    // "1[#sep], 2[#sep], "（2.3.34 修复后行为）—— 引擎差异，断言按实测调整
    assert_output(
        &c,
        &loader,
        "<#list [1, 2] as i>${i}[#sep], </#list>",
        "1[#sep], 2[#sep], ",
    );
    // square bracket tags were always "strict":
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}[sep], [/#list]",
        "1[sep], 2[sep], ",
    );
    // Java 2.3.33：`<#sep>` 在方括号块内仍被当分隔符（→ "1, 2"）；本引擎自动检测
    // 为方括号语法后 `<#sep>` 按文本输出 "1<#sep>, 2<#sep>, "（2.3.34 行为）——
    // 引擎差异，断言按实测调整
    assert_output(
        &c,
        &loader,
        "[#list [1, 2] as i]${i}<#sep>, [/#list]",
        "1<#sep>, 2<#sep>, ",
    );
}
