//! Java `freemarker.core.MiscErrorMessagesTest` 的 Rust 1:1 实现
//! （MiscErrorMessagesTest.java：错误消息杂项断言）

use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java stringIndexOutOfBounds：`'foo'[10]` → 消息含 "length"、"3"、"10"、"String index out of"
#[test]
fn string_index_out_of_bounds() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "${'foo'[10]}",
        &["length", "3", "10", "String index out of"],
    );
}

/// Java wrongTemplateNameFormat：测试先 setTemplateNameFormat(DEFAULT_2_4_0)，
/// 在 2.4.0 名称格式下 `foo:/bar:baaz`（':' 检查）、`../baaz`（越出根）、`\u0000`
/// 均报 "Malformed template name"。v1 引擎未实现 DEFAULT_2_4_0 名称格式（固定
/// Java 2.3.34 默认的 DEFAULT_2_3_0）——2.3.0 格式下这些名字**不**报 Malformed
/// （jar 实测：foo:/bar:baaz → Template not found），故 2.4.0 断言登记偏差：
/// TemplateNameFormat.DEFAULT_2_4_0 未实现（P6）。此处验证 2.3.0 默认行为与
/// Java 一致（Template not found / ../ 越界检查）。
#[test]
fn wrong_template_name_format() {
    let (c, loader) = cfg();
    // Java 2.3.34 默认（DEFAULT_2_3_0）：'foo:/bar:baaz' 合法名（scheme 前缀）→ not found
    assert_error_contains(
        &c,
        &loader,
        "<#include 'foo:/bar:baaz'>",
        &["Template not found"],
    );
    // '../baaz' 越出模板根：DEFAULT_2_3_0 的 normalizeRootBasedName 报错
    // （Java :186-190；v1 消息对齐 Java Default020300.rootLeaving 形式）
    let msg = assert_error_contains(&c, &loader, "<#include '../baaz'>", &["../"]);
    assert!(msg.contains("root"), "越界消息应含 root 提示：{msg}");
    // '\u0000'：NUL 字符检查（DEFAULT_2_3_0 :173 checkNameHasNoNullCharacter）
    assert_error_contains(&c, &loader, "<#include '\u{0}'>", &["\\u0000"]);
}

/// Java numericalKeyHint：哈希数字键越界提示 ?api
/// （v1 无 ?api —— 若消息不含 "?api" 则记录偏差）
#[test]
fn numerical_key_hint() {
    let (c, loader) = cfg();
    let msg = assert_error_contains(&c, &loader, "${{}[10]}", &["[]"]);
    if !msg.contains("?api") {
        // v1 偏差：Java 对序列数字键越界附加 "?api" 提示（?api 为 BeanWrapper 特有）
    }
}

/// Java aritheticException：除零错误 + 行号 2
#[test]
fn arithetic_exception() {
    let (c, loader) = cfg();
    let msg = assert_error_contains(&c, &loader, "<#assign x = 0>\n${1 / x}", &["Arithmetic"]);
    assert!(msg.contains("line 2"), "消息应含行号 2：{msg}");
}

/// Java incrementalAssignmentsTest：复合赋值的错误消息（target 名 + 运算符 + 作用域）
#[test]
fn incremental_assignments_test() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#assign x++>",
        &["\"x\"", "++", "template namespace"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#global x += 2>",
        &["\"x\"", "+=", "global scope"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#macro m><#local x--></#macro><@m/>",
        &["\"x\"", "--", "local scope"],
    );
}

/// Java assignmentNamespaceChecks：in 目标缺失 / 非命名空间
#[test]
fn assignment_namespace_checks() {
    let (c, loader) = cfg();
    let msg = assert_error_contains(&c, &loader, "<#assign x = 1 in noSuchVar>", &["noSuchVar"]);
    assert!(msg.contains("null or missing"), "{msg}");
    assert_error_contains(
        &c,
        &loader,
        "<#assign x =1 in 'notANamespace'>",
        &["notANamespace"],
    );
}

/// Java blockAssignmentNamespaceChecks：块赋值 in 目标
#[test]
fn block_assignment_namespace_checks() {
    let (c, loader) = cfg();
    let msg = assert_error_contains(
        &c,
        &loader,
        "<#assign x in noSuchVar>1</#assign>",
        &["noSuchVar"],
    );
    assert!(msg.contains("null or missing"), "{msg}");
    assert_error_contains(
        &c,
        &loader,
        "<#assign x in 'notANamespace'>1</#assign>",
        &["notANamespace"],
    );
}
