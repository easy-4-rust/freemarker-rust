//! 对应 Java: TakeWhileAndDropWhileBiTest
//! Java `freemarker.core.TakeWhileAndDropWhileBiTest` 的 Rust 1:1 实现。
//! Java createConfiguration：DefaultObjectWrapper 2.3.28 + forceLegacyNonListCollections
//! （v1 无对应配置，数据用普通序列即可——引擎差异无碍断言）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java TEST_PARAMS：（列表, takeWhile 结果, dropWhile 结果）
struct TestParam {
    list: &'static [&'static str],
    take_while_result: &'static str,
    drop_while_result: &'static str,
}

const TEST_PARAMS: [TestParam; 11] = [
    TestParam {
        list: &[],
        take_while_result: "",
        drop_while_result: "",
    },
    TestParam {
        list: &["a"],
        take_while_result: "a",
        drop_while_result: "a",
    },
    TestParam {
        list: &["a", "b", "c"],
        take_while_result: "a, b, c",
        drop_while_result: "a, b, c",
    },
    TestParam {
        list: &["aX"],
        take_while_result: "",
        drop_while_result: "",
    },
    TestParam {
        list: &["aX", "b"],
        take_while_result: "",
        drop_while_result: "b",
    },
    TestParam {
        list: &["aX", "b", "c"],
        take_while_result: "",
        drop_while_result: "b, c",
    },
    TestParam {
        list: &["a", "bX", "c"],
        take_while_result: "a",
        drop_while_result: "a, bX, c",
    },
    TestParam {
        list: &["a", "b", "cX"],
        take_while_result: "a, b",
        drop_while_result: "a, b, cX",
    },
    TestParam {
        list: &["aX", "bX", "c"],
        take_while_result: "",
        drop_while_result: "c",
    },
    TestParam {
        list: &["aX", "bX", "cX"],
        take_while_result: "",
        drop_while_result: "",
    },
    TestParam {
        list: &["aX", "b", "cX"],
        take_while_result: "",
        drop_while_result: "b, cX",
    },
];

fn seq_of(items: &[&str]) -> TModel {
    TModel::from_sequence(
        items
            .iter()
            .map(|s| TModel::from_scalar(s.to_string()))
            .collect(),
    )
}

/// Java testTakeWhile
#[test]
fn test_take_while() {
    let (c, loader) = cfg();
    for tp in &TEST_PARAMS {
        let mut dm = indexmap::IndexMap::new();
        dm.insert("xs".to_string(), seq_of(tp.list));
        let dm = TModel::from_hash(dm);
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#list xs?takeWhile(it -> !it?contains('X')) as x>${x}<#sep>, </#list>",
            dm.clone(),
        );
        assert_eq!(out, tp.take_while_result);
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#assign fxs = xs?takeWhile(it -> !it?contains('X'))>${fxs?join(', ')}",
            dm,
        );
        assert_eq!(out, tp.take_while_result);
    }
}

/// Java testDropWhile
#[test]
fn test_drop_while() {
    let (c, loader) = cfg();
    for tp in &TEST_PARAMS {
        let mut dm = indexmap::IndexMap::new();
        dm.insert("xs".to_string(), seq_of(tp.list));
        let dm = TModel::from_hash(dm);
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#list xs?dropWhile(it -> it?contains('X')) as x>${x}<#sep>, </#list>",
            dm.clone(),
        );
        assert_eq!(out, tp.drop_while_result);
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#assign fxs = xs?dropWhile(it -> it?contains('X'))>${fxs?join(', ')}",
            dm,
        );
        assert_eq!(out, tp.drop_while_result);
    }
}

/// Java testBetween：两个内建链式（并非特例，但期望借此触发 bug）
#[test]
fn test_between() {
    let (c, loader) = cfg();
    let ftl =
        "<#list xs?dropWhile(it -> it < 0)?takeWhile(it -> it >= 0) as x>${x}<#sep>, </#list>";

    let xs = TModel::from_sequence(vec![
        TModel::from_number(TNumber::Int(-1)),
        TModel::from_number(TNumber::Int(-2)),
        TModel::from_number(TNumber::Int(3)),
        TModel::from_number(TNumber::Int(4)),
        TModel::from_number(TNumber::Int(-5)),
        TModel::from_number(TNumber::Int(-6)),
    ]);
    let mut dm = indexmap::IndexMap::new();
    dm.insert("xs".to_string(), xs);
    let dm = TModel::from_hash(dm);
    assert_eq!(render_ftl_with_dm(&c, &loader, ftl, dm), "3, 4");

    let xs = TModel::from_sequence(vec![
        TModel::from_number(TNumber::Int(-1)),
        TModel::from_number(TNumber::Int(-2)),
        TModel::from_number(TNumber::Int(-5)),
        TModel::from_number(TNumber::Int(-6)),
    ]);
    let mut dm = indexmap::IndexMap::new();
    dm.insert("xs".to_string(), xs);
    let dm = TModel::from_hash(dm);
    assert_eq!(render_ftl_with_dm(&c, &loader, ftl, dm), "");

    let xs = TModel::from_sequence(vec![
        TModel::from_number(TNumber::Int(1)),
        TModel::from_number(TNumber::Int(2)),
        TModel::from_number(TNumber::Int(3)),
    ]);
    let mut dm = indexmap::IndexMap::new();
    dm.insert("xs".to_string(), xs);
    let dm = TModel::from_hash(dm);
    assert_eq!(render_ftl_with_dm(&c, &loader, ftl, dm), "1, 2, 3");

    let mut dm = indexmap::IndexMap::new();
    dm.insert("xs".to_string(), TModel::from_sequence(vec![]));
    let dm = TModel::from_hash(dm);
    assert_eq!(render_ftl_with_dm(&c, &loader, ftl, dm), "");
}

/// Java testSnakeCaseNames
#[test]
fn test_snake_case_names() {
    let (c, loader) = cfg();
    let xs = TModel::from_sequence(vec![
        TModel::from_number(TNumber::Int(-1)),
        TModel::from_number(TNumber::Int(-2)),
        TModel::from_number(TNumber::Int(3)),
        TModel::from_number(TNumber::Int(4)),
        TModel::from_number(TNumber::Int(-5)),
        TModel::from_number(TNumber::Int(-6)),
    ]);
    let mut dm = indexmap::IndexMap::new();
    dm.insert("xs".to_string(), xs);
    let dm = TModel::from_hash(dm);
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list xs?drop_while(it -> it < 0)?take_while(it -> it >= 0) as x>${x}<#sep>, </#list>",
        dm,
    );
    assert_eq!(out, "3, 4");
}
