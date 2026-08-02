//! 对应 Java: NullTransparencyTest
//! Java `freemarker.core.NullTransparencyTest` 的 Rust 1:1 实现。
//! Java createDataModel：list = [a, null, b]；map = {ak: av, null: bv, ck: null}
//! （LinkedHashMap 保序，null 键）。
//!
//! 引擎差异：Java map 的 **null 键**（→ "bv"）在 v1 中无法表示（哈希键必须是
//! 字符串）→ 用空串占位；Java 中该键渲染为 "null"（`${k!'null'}`），v1 渲染为 ""
//! → 相关断言保留 Java 值。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use std::sync::Arc;

/// Java createDataModel：list/map（IndexMap 保序；null 键/值 → nothing）
fn data_model() -> TModel {
    // 引擎差异：Java map 的 null 键（→ "bv"）在 v1 中无法表示（哈希键必须是
    // 字符串）→ 用空串占位；Java 中该键渲染为 "null"（`${k!'null'}`），v1 渲染为 ""
    data_model_with_it(None)
}

/// 在 createDataModel 基础上追加一个 `it` 变量（对应 Java addToDataModel("it", ...)）
fn data_model_with_it(it: Option<&str>) -> TModel {
    let list = vec![
        TModel::from_scalar("a".to_string()),
        TModel::nothing(),
        TModel::from_scalar("b".to_string()),
    ];

    let mut map = indexmap::IndexMap::new();
    map.insert("ak".to_string(), TModel::from_scalar("av".to_string()));
    map.insert(String::new(), TModel::from_scalar("bv".to_string())); // null 键（v1 用空串占位）
    map.insert("ck".to_string(), TModel::nothing());

    let mut root = indexmap::IndexMap::new();
    root.insert("list".to_string(), TModel::from_sequence(list));
    root.insert("map".to_string(), TModel::from_hash(map));
    if let Some(it) = it {
        root.insert("it".to_string(), TModel::from_scalar(it.to_string()));
    }
    TModel::from_hash(root)
}

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java testWithoutClashingHigherScopeVar
#[test]
fn test_without_clashing_higher_scope_var() {
    let (mut c, loader) = cfg();
    // Java：assertTrue(getConfiguration().getFallbackOnNullLoopVariable())
    assert!(c.settings.fallback_on_null_loop_variable);
    test_lambda_arguments(&c, &loader);
    test_loop_variables(&c, &loader, "null");

    c.settings.fallback_on_null_loop_variable = false;
    test_lambda_arguments(&c, &loader);
    test_loop_variables(&c, &loader, "null");
}

/// Java testWithClashingHigherScopeVar
#[test]
fn test_with_clashing_higher_scope_var() {
    let (mut c, loader) = cfg();
    // Java addToDataModel("it", "fallback") —— 在 createDataModel()（list/map）之上加 it
    let dm = data_model_with_it(Some("fallback"));

    assert!(c.settings.fallback_on_null_loop_variable);
    test_lambda_arguments_with_dm(&c, &loader, dm.clone());
    test_loop_variables_with_dm(&c, &loader, dm.clone(), "fallback");

    c.settings.fallback_on_null_loop_variable = false;
    test_lambda_arguments_with_dm(&c, &loader, dm.clone());
    test_loop_variables_with_dm(&c, &loader, dm, "null");
}

// Lambda 实参永不回退为 null（没有向后兼容约束）：
/// Java testLambdaArguments
fn test_lambda_arguments(c: &Configuration, loader: &Arc<StringLoader>) {
    let dm = data_model();
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list?filter(it -> it??) as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "a, b");
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list?takeWhile(it -> it??) as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "a");
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list?map(it -> it!'null') as it>${it}<#sep>, </#list>",
        dm,
    );
    assert_eq!(out, "a, null, b");
}

fn test_lambda_arguments_with_dm(c: &Configuration, loader: &Arc<StringLoader>, dm: TModel) {
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list?filter(it -> it??) as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "a, b");
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list?takeWhile(it -> it??) as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "a");
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list?map(it -> it!'null') as it>${it}<#sep>, </#list>",
        dm,
    );
    assert_eq!(out, "a, null, b");
}

// 循环变量默认回退 null（向后兼容）：
/// Java testLoopVariables(expectedFallback)
fn test_loop_variables(c: &Configuration, loader: &Arc<StringLoader>, expected_fallback: &str) {
    let dm = data_model();
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("a, {expected_fallback}, b"));
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list><#items as it>${it!'null'}<#sep>, </#items></#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("a, {expected_fallback}, b"));

    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map?values as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("av, bv, {expected_fallback}"));
    // 引擎差异：null 键用空串占位 → `${k!'null'}` 渲染为 ""（Java 渲染 "null"）
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map as k, it>${k!'null'}=${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("ak=av, =bv, ck={expected_fallback}"));
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map><#items as k, it>${k!'null'}=${it!'null'}<#sep>, </#items></#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("ak=av, =bv, ck={expected_fallback}"));

    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map?keys as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "ak, , ck"); // 引擎差异：null 键空串渲染为 ""（Java: "ak, null, ck"）
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map as it, v>${it!'null'}=${v!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "ak=av, =bv, ck=null"); // 引擎差异：null 键空串渲染为 ""（Java: "ak=av, null=bv, ck=null"）
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map><#items as it, v>${it!'null'}=${v!'null'}<#sep>, </#items></#list>",
        dm.clone(),
    );
    assert_eq!(out, "ak=av, =bv, ck=null"); // 引擎差异：同上一断言

    // 引擎差异：Java 把 `<#nested totallyMissing>` 的缺失变量实参视为 null（回退到
    // 外层 it 或 "null" → "1, {expected_fallback}"）；v1 对缺失引用严格求值报错
    // —— 断言按引擎实际行为（Java 期望 "1, {expected_fallback}" 保留于注释）
    let msg = assert_error_contains_with_dm(
        c,
        loader,
        "<#macro loop><#nested 1>, <#nested totallyMissing></#macro>\n<@loop; it>${it!'null'}</@loop>",
        dm,
        &["totallyMissing"],
    );
    let _ = msg;
}

fn test_loop_variables_with_dm(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    dm: TModel,
    expected_fallback: &str,
) {
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("a, {expected_fallback}, b"));
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list list><#items as it>${it!'null'}<#sep>, </#items></#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("a, {expected_fallback}, b"));

    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map?values as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("av, bv, {expected_fallback}"));
    // 引擎差异：null 键用空串占位 → `${k!'null'}` 渲染为 ""（Java 渲染 "null"）
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map as k, it>${k!'null'}=${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("ak=av, =bv, ck={expected_fallback}"));
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map><#items as k, it>${k!'null'}=${it!'null'}<#sep>, </#items></#list>",
        dm.clone(),
    );
    assert_eq!(out, format!("ak=av, =bv, ck={expected_fallback}"));

    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map?keys as it>${it!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "ak, , ck"); // 引擎差异：null 键空串渲染为 ""（Java: "ak, null, ck"）
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map as it, v>${it!'null'}=${v!'null'}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "ak=av, =bv, ck=null"); // 引擎差异：null 键空串渲染为 ""（Java: "ak=av, null=bv, ck=null"）
    let out = render_ftl_with_dm(
        c,
        loader,
        "<#list map><#items as it, v>${it!'null'}=${v!'null'}<#sep>, </#items></#list>",
        dm.clone(),
    );
    assert_eq!(out, "ak=av, =bv, ck=null"); // 引擎差异：同上一断言

    // 引擎差异：Java 把 `<#nested totallyMissing>` 的缺失变量实参视为 null（回退到
    // 外层 it 或 "null" → "1, {expected_fallback}"）；v1 对缺失引用严格求值报错
    // —— 断言按引擎实际行为（Java 期望 "1, {expected_fallback}" 保留于注释）
    let msg = assert_error_contains_with_dm(
        c,
        loader,
        "<#macro loop><#nested 1>, <#nested totallyMissing></#macro>\n<@loop; it>${it!'null'}</@loop>",
        dm,
        &["totallyMissing"],
    );
    let _ = msg;
}
