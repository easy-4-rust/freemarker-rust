//! Java `freemarker.core.ListBreakContinueTest` 的 Rust 1:1 实现
//! （对应 Java: ListBreakContinueTest —— #list 遍历序列/集合/哈希时的
//! #break/#continue/#sep 行为）
//!
//! Java createConfiguration：ICI 2.3.27 + DefaultObjectWrapperBuilder
//! （setForceLegacyNonListCollections(false)）；本引擎无 ObjectWrapper，
//! 数据模型直接用 TModel 序列/哈希构造（等价物）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use std::sync::Arc;

/// 1..5 的序列（Java ImmutableList/ImmutableSet 经对象包装为
/// TemplateSequenceModel/TemplateCollectionModel；v1 用 from_sequence）
fn listed_sequence() -> TModel {
    TModel::from_sequence(
        (1..=5)
            .map(|i| TModel::from_number(TNumber::Int(i)))
            .collect(),
    )
}

/// a=1..e=5 的哈希（Java ImmutableMap 经包装为 TemplateHashModelEx2；
/// v1 用 from_hash；NonEx2Hash 包装器在 v1 无 TemplateHashModelEx 家族区分，
/// 行为一致 —— 注释见 testHash）
fn listed_hash() -> TModel {
    let mut map = indexmap::IndexMap::new();
    for (i, k) in ["a", "b", "c", "d", "e"].iter().enumerate() {
        map.insert(
            k.to_string(),
            TModel::from_number(TNumber::Int((i + 1) as i32)),
        );
    }
    TModel::from_hash(map)
}

/// 带数据模型渲染并断言输出（对应 Java addToDataModel("listed", ...) +
/// assertOutput）
fn assert_output_dm(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    listed: &TModel,
    ftl: &str,
    expected: &str,
) {
    let mut root = indexmap::IndexMap::new();
    root.insert("listed".to_string(), listed.clone());
    let dm = TModel::from_hash(root);
    let out = render_ftl_with_dm(c, loader, ftl, dm);
    assert_eq!(out, expected, "ftl: {ftl}");
}

/// Java testNonHash：列出序列/集合（TemplateSequenceModel / TemplateCollectionModel）
#[test]
fn test_non_hash() {
    // Java 分别用 ImmutableList（TemplateSequenceModel）与 ImmutableSet
    // （TemplateCollectionModel）验证；v1 序列模型同时具备两角色，验证一次
    test_non_hash_impl(listed_sequence());
}

fn test_non_hash_impl(listed: TModel) {
    let (c, loader) = test_config();
    assert_output_dm(
        &c,
        &loader,
        &listed,
        "<#list listed as i>B(${i}) <#if i == 3>Break!<#break></#if>A(${i})<#sep>, </#list>",
        "B(1) A(1), B(2) A(2), B(3) Break!",
    );
    assert_output_dm(
        &c,
        &loader,
        &listed,
        "<#list listed as i>B(${i}) <#if i == 3>Continue! <#continue></#if>A(${i})<#sep>, </#list>",
        "B(1) A(1), B(2) A(2), B(3) Continue! B(4) A(4), B(5) A(5)",
    );
}

/// Java testHash：列出哈希（TemplateHashModelEx2 / 非 Ex2）
#[test]
fn test_hash() {
    // Java 分别用 ImmutableMap（Ex2）与 NonEx2Hash 包装（非 Ex2）验证；
    // v1 哈希模型不区分 Ex2（keys() 即迭代源），验证一次
    test_hash_impl(listed_hash());
}

fn test_hash_impl(listed: TModel) {
    let (c, loader) = test_config();
    assert_output_dm(
        &c,
        &loader,
        &listed,
        "<#list listed as k, v>B(${k}=${v}) <#if k == 'c'>Break!<#break></#if>A(${k}=${v})<#sep>, </#list>",
        "B(a=1) A(a=1), B(b=2) A(b=2), B(c=3) Break!",
    );
    assert_output_dm(
        &c,
        &loader,
        &listed,
        "<#list listed as k, v>B(${k}=${v}) <#if k == 'c'>Continue! <#continue></#if>A(${k}=${v})<#sep>, </#list>",
        "B(a=1) A(a=1), B(b=2) A(b=2), B(c=3) Continue! B(d=4) A(d=4), B(e=5) A(e=5)",
    );
}

// Java NonEx2Hash 辅助类（隐藏 TemplateHashModelEx2 特性）：引擎无 Ex2 区分，
// 无对应物（非测试，忽略）
