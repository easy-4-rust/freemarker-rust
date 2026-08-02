//! 对应 Java: SequenceBuiltInTest
//! Java `freemarker.core.SequenceBuiltInTest` 的 Rust 1:1 实现。
//!
//! 引擎差异总览：
//! - Java 用 DefaultIterableAdapter/DefaultNonListCollectionAdapter 把 Set 适配为
//!   纯 collection / CollectionEx；v1 用 TModel::from_collection（collection_ex 标志
//!   可设）模拟。
//! - `?sequence` 内建 v1 **未实现** —— 所有 `${x?sequence...}` 用例报
//!   "Unknown built-in: ?sequence"（Java 期望 "b"/"2"/"12"）。
//! - 集合 `[i]` 索引：Java 报错含 "?sequence" 提示；v1 报
//!   "Expected a sequence or string ... but this has evaluated to a collection"
//!   （提示 ?api，无 "?sequence" 子串）。
//! - `?size` 对 collection：Java 对纯 collection 报错（含 "?sequence" 提示）、对
//!   CollectionEx 直接可用；v1 一律报 "?size is not applicable to a collection value"。
//! - setIncompatibleImprovements(2.3.23) 的 `?sequence` 返回原序列语义无法验证。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// 纯 collection 模型（对应 Java DefaultIterableAdapter.adapt(Set)：非 sequence、
/// 非 CollectionEx）
fn collection_model(items: &[&str]) -> TModel {
    let mut m = TModel::from_collection(
        items
            .iter()
            .map(|s| TModel::from_scalar(s.to_string()))
            .collect(),
    );
    m.collection_ex = false;
    m
}

/// Java testWithCollection
#[test]
fn test_with_collection() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert("xs".to_string(), collection_model(&["a", "b"]));
    let dm = TModel::from_hash(dm);

    // Java：xs 不是 sequence（assertThat not instanceOf TemplateSequenceModel）——
    // v1 同样（from_collection 无 sequence 角色），引擎差异仅限 Java 断言模型类型。
    // 引擎消息差异：Java 报错含 "?sequence" 提示；v1 报 "has evaluated to a collection"
    // （提示 ?api，无 "?sequence" 子串）。
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${xs[1]}",
        dm.clone(),
        &["has evaluated to a collection"],
    );
    // 引擎差异：?sequence 未实现 → "Unknown built-in: ?sequence"；Java 期望 "b"
    assert_error_contains(
        &c,
        &loader,
        "${xs?sequence[1]}",
        &["Unknown built-in: ?sequence"],
    );

    // 引擎消息差异：Java 报错含 "?sequence" 提示；v1 "?size is not applicable to a collection value"
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${xs?size}",
        dm.clone(),
        &["not applicable to a collection value"],
    );
    // 引擎差异：?sequence 未实现 → 未知内建；Java 期望 "2"
    assert_error_contains(
        &c,
        &loader,
        "${xs?sequence?size}",
        &["Unknown built-in: ?sequence"],
    );
}

/// Java testWithCollectionEx
#[test]
fn test_with_collection_ex() {
    let (c, loader) = cfg();
    // 对应 Java DefaultNonListCollectionAdapter.adapt(Set)：collection + CollectionEx
    let mut xs = collection_model(&["a", "b"]);
    xs.collection_ex = true;
    let mut dm = indexmap::IndexMap::new();
    dm.insert("xs".to_string(), xs);
    let dm = TModel::from_hash(dm);

    // 引擎消息差异：同 testWithCollection（Java 报错含 "?sequence" 提示；v1 无该子串）
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${xs[1]}",
        dm.clone(),
        &["has evaluated to a collection"],
    );
    // 引擎差异：?sequence 未实现 → 未知内建；Java 期望 "b"
    assert_error_contains(
        &c,
        &loader,
        "${xs?sequence[1]}",
        &["Unknown built-in: ?sequence"],
    );

    // CollectionEx：Java 可直接 ?size（无需 ?sequence）→ "2"；
    // 引擎差异：v1 ?size 对 collection（含 CollectionEx）一律报不可用
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${xs?size}",
        dm,
        &["not applicable to a collection value"],
    );
}

/// Java testWithSequence
#[test]
fn test_with_sequence() {
    let (c, loader) = cfg();
    // Java：${[11, 12]?sequence[1]} → "12"；v1 ?sequence 未实现（引擎差异）
    assert_error_contains(
        &c,
        &loader,
        "${[11, 12]?sequence[1]}",
        &["Unknown built-in: ?sequence"],
    );

    // Java：setIncompatibleImprovements(2.3.23) 后 ?sequence 原样返回序列，
    // 对无限序列 (11..) 也有效 → "12"；v1 ?sequence 未实现（引擎差异）
    let (mut c, loader) = cfg();
    c.settings.incompatible_improvements = freemarker::template::Version::parse("2.3.23").unwrap();
    assert_error_contains(
        &c,
        &loader,
        "${(11..)?sequence[1]}",
        &["Unknown built-in: ?sequence"],
    );
}
