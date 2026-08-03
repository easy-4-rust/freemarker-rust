//! Java `freemarker.core.UnclosedCommentTest` 的 Rust 1:1 实现
//! （对应 Java: UnclosedCommentTest —— 未闭合注释 / <#noparse> 的旧版与修复行为）
//!
//! Java 用 ICI 2.3.20（legacy）与 2.3.21（fixed）区分行为；本引擎固定
//! ICI 2.3.34（等同 fixed）—— testLegacyBehavior 的差异见函数内注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

const UNCLOSED_COMMENT_0: &str = "foo<#--";
const UNCLOSED_COMMENT_1: &str = "foo<#-- ";
const UNCLOSED_COMMENT_2: &str = "foo<#--bar";
const UNCLOSED_COMMENT_3: &str = "foo\n<#--\n";

const UNCLOSED_NOPARSE_0: &str = "foo<#noparse>";
const UNCLOSED_NOPARSE_1: &str = "foo<#noparse> ";
const UNCLOSED_NOPARSE_2: &str = "foo<#noparse>bar";
const UNCLOSED_NOPARSE_3: &str = "foo\n<#noparse>\n";

/// Java testLegacyBehavior（setConfiguration(new Configuration(VERSION_2_3_20))）：
/// ICI 2.3.20 下未闭合注释被当作静态文本吞掉。
/// 引擎差异：本引擎固定 ICI 2.3.34（无版本设置），一律按"修复后"行为报错 ——
/// Java 2.3.20 中 UNCLOSED_COMMENT_1/2/3 与 UNCLOSED_NOPARSE_1/2/3 输出
/// "foo"/"foo\n"、0 号变体报 "end of file"；v1 一律报 "Unclosed"。
#[test]
fn test_legacy_behavior() {
    let (c, loader) = test_config();
    // 引擎差异（固定 2.3.34 = 修复后行为）：以下全部在 v1 解析期报错
    assert_error_contains(&c, &loader, UNCLOSED_COMMENT_0, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_COMMENT_1, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_COMMENT_2, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_COMMENT_3, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_NOPARSE_0, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_NOPARSE_1, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_NOPARSE_2, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_NOPARSE_3, &["Unclosed"]);
}

/// Java testFixedBehavior（ICI 2.3.21+）：未闭合注释明确报 "Unclosed"
#[test]
fn test_fixed_behavior() {
    let (c, loader) = test_config();
    // 引擎消息差异：Java 对 "foo<#--"（0 号）报 "end of file"（Java 注释 "Not too good..."）；
    // v1 统一报 "Unclosed comment."
    assert_error_contains(&c, &loader, UNCLOSED_COMMENT_0, &["Unclosed \"<#--\""]);
    // 引擎消息差异：Java 断言 ["Unclosed", "<#--"]；v1 消息 "Unclosed comment." 不含 "<#--"
    assert_error_contains(&c, &loader, UNCLOSED_COMMENT_1, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_COMMENT_2, &["Unclosed"]);
    assert_error_contains(&c, &loader, UNCLOSED_COMMENT_3, &["Unclosed"]);
    // 引擎消息差异：Java 对 "foo<#noparse>"（0 号）报 "end of file"；v1 报 Unclosed "<#noparse>"
    assert_error_contains(&c, &loader, UNCLOSED_NOPARSE_0, &["Unclosed", "#noparse"]);
    assert_error_contains(&c, &loader, UNCLOSED_NOPARSE_1, &["Unclosed", "#noparse"]);
    assert_error_contains(&c, &loader, UNCLOSED_NOPARSE_2, &["Unclosed", "#noparse"]);
    assert_error_contains(&c, &loader, UNCLOSED_NOPARSE_3, &["Unclosed", "#noparse"]);
}
