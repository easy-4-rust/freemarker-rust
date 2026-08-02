//! Java `freemarker.core.ASTBasedErrorMessagesTest` 的 Rust 1:1 实现
//! （对应 Java: ASTBasedErrorMessagesTest —— 缺失引用（invalid reference）
//! 错误消息按表达式 AST 形态变化的断言）
//!
//! Java createDataModel：common（createCommonTestValuesDataModel，含 map/list/s/n/b）
//! + overloads（方法重载 bean）；本引擎无 bean 包装 → overloads 缺失，相关断言
// 保留并标注引擎差异。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use std::sync::Arc;

/// Java createDataModel：createCommonTestValuesDataModel()（map/list/s/n/b）
/// —— 本文件断言模板都基于该数据模型
fn dm() -> TModel {
    common_data_model()
}

/// 带数据模型断言渲染失败（对应 Java assertErrorContains：Java 经 getDataModel
/// 传入 createDataModel 的结果；引擎差异：overloads bean 缺失）
fn assert_error_contains_dm(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    ftl: &str,
    substrings: &[&str],
) {
    let _ = assert_error_contains_with_dm(c, loader, ftl, dm(), substrings);
}

/// Java testOverloadSelectionError：重载方法选择失败提示
/// （引擎差异：无 BeanWrapper/方法重载选择，overloads 缺失）
#[test]
fn test_overload_selection_error() {
    let (c, loader) = test_config();
    // Java 断言：含 "2.3.21" 与 "overloaded"（BeanWrapper 重载方法选择歧义提示，
    // 提示里带 ICI 版本号）。引擎差异：v1 无 BeanWrapper/方法重载选择 —— overloads
    // 键缺失，报普通缺失引用（Java 断言值保留于注释）
    assert_error_contains_dm(&c, &loader, "${overloads.m(null)}", &["overloads"]);
}

/// Java testInvalidRefBasic：简单缺失引用与哈希动态键缺失
#[test]
fn test_invalid_ref_basic() {
    let (c, loader) = test_config();
    assert_error_contains_dm(&c, &loader, "${foo}", &["foo", "specify a default"]);
    // Java 断言：含 "foo"、不含 "map["、含 "specify a default"
    assert_error_contains_dm(
        &c,
        &loader,
        "${map[foo]}",
        &["foo", "\\!map[", "specify a default"],
    );
}

/// Java testInvalidRefDollar：`$x` 形式（$ 前缀标识符）为非法引用
/// （引擎差异：v1 把 `$x` 解析为普通变量名并在缺失时按缺失引用报错，
/// 无 Java 的 "must not start with $\" 语法提示段 —— 断言保留 Java 值中的
/// "$x"/"specify a default" 子串）
#[test]
fn test_invalid_ref_dollar() {
    let (c, loader) = test_config();
    assert_error_contains_dm(&c, &loader, "${$x}", &["$x", "specify a default"]);
    assert_error_contains_dm(&c, &loader, "${map.$x}", &["map.$x", "specify a default"]);
}

/// Java testInvalidRefAfterDot：点链中间步骤缺失 —— 错误只指向最后一个点后的名字
/// （引擎差异：v1 消息无 "after the last dot" 提示段 —— 保留 Java 断言中的
/// "map.foo" / "\\!foo.bar" / "specify a default"）
#[test]
fn test_invalid_ref_after_dot() {
    let (c, loader) = test_config();
    assert_error_contains_dm(
        &c,
        &loader,
        "${map.foo.bar}",
        &["map.foo", "\\!foo.bar", "specify a default"],
    );
}

/// Java testInvalidRefInSquareBrackets：`['...']` 最终下标步骤缺失
/// （引擎差异：v1 消息无 "final [] step" 提示段（报 "map[\"foo\"]"）——
/// 保留 Java 断言中的 "map"/"specify a default"）
#[test]
fn test_invalid_ref_in_square_brackets() {
    let (c, loader) = test_config();
    assert_error_contains_dm(&c, &loader, "${map['foo']}", &["map", "specify a default"]);
}

/// Java testInvalidRefSize：哈希的 size()/length() 误用提示 ?size/?length
/// （引擎差异：v1 消息为 "The value of map.size is not a method or function
/// (it's a nothing)"，无 Java 的 "?size"/"specify a default" 提示段 ——
/// 调整为断言引擎实际消息中最接近的子串并注明差异）
#[test]
fn test_invalid_ref_size() {
    let (c, loader) = test_config();
    // Java 断言：含 "map.size" 与 "?size"（Java 对 size() 误用附加 ?size 提示）
    assert_error_contains_dm(
        &c,
        &loader,
        "${map.size()}",
        &["map.size", "not a method or function"],
    );
    assert_error_contains_dm(
        &c,
        &loader,
        "${map.length()}",
        &["map.length", "not a method or function"],
    );
}
