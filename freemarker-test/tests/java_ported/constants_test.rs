//! Java `freemarker.template.utility.ConstantsTest` 的 Rust 1:1 实现
//! （ConstantsTest.java：Constants.EMPTY_HASH 的空哈希行为测试）
//!
//! 引擎映射：v1 `TModel::from_hash(空 IndexMap)` 对应 Constants.EMPTY_HASH
//! （空哈希；`?size`/`?keys`/`?values`/list 迭代均正常）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::template::TModel;

/// Java testEmptyHash：空哈希的迭代/键/值/大小
#[test]
fn test_empty_hash() {
    let (c, loader) = test_config();
    let mut dm_map = indexmap::IndexMap::new();
    dm_map.insert(
        "h".to_string(),
        TModel::from_hash(indexmap::IndexMap::new()),
    );
    let dm = TModel::from_hash(dm_map);

    // Java：addToDataModel("h", Constants.EMPTY_HASH) 后：
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "{<#list h as k ,v>x</#list>}", dm.clone()),
        "{}"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "{<#list h?keys as k>x</#list>}", dm.clone()),
        "{}"
    );
    assert_eq!(
        render_ftl_with_dm(&c, &loader, "{<#list h?values as k>x</#list>}", dm.clone()),
        "{}"
    );
    assert_eq!(render_ftl_with_dm(&c, &loader, "${h?size}", dm), "0");
}
