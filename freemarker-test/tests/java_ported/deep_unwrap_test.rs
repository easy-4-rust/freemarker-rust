//! Java `freemarker.template.utility.DeepUnwrapTest` 的 Rust 1:1 实现
//! （DeepUnwrapTest.java：DeepUnwrap.unwrap 的哈希往返/纯模型复制测试）
//!
//! 引擎映射：v1 `ObjectWrapper::unwrap`（simple_object_wrapper.rs：
//!   scalar → number → boolean → date → hash → sequence/collection → nothing，
//!   对应 Java DeepUnwrap.java:100-176）。
//! 引擎差异：
//! - Java `unwrap(wrap(map))` 断言返回**同一实例**（DefaultObjectWrapper 的
//!   Map 适配器直接返回原 map）——v1 TModel 是值类型，unwrap 恒产新 DynValue，
//!   断言内容相等；
//! - Java 用例含非字符串键（3）与 null 键——v1 DynValue::Map 键恒为 String；
//! - "纯 TemplateHashModelEx2 复制"（unwrap 结果不等于原 map）——v1 无
//!   "适配器 vs 纯模型"之分（from_hash 恒为纯哈希）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::template::{DynValue, ObjectWrapper, SIMPLE_WRAPPER};

/// 对应 Java DeepUnwrap.unwrap(model)：v1 经 ObjectWrapper::unwrap
fn unwrap(model: &freemarker::template::TModel) -> DynValue {
    SIMPLE_WRAPPER.unwrap(model).expect("unwrap 不应失败")
}

/// Java testHashEx2Unwrapping：哈希的 unwrap 往返
#[test]
fn test_hash_ex2_unwrapping() {
    let map: Vec<(String, DynValue)> = vec![
        ("k1".to_string(), DynValue::Str("v1".to_string())),
        ("k2".to_string(), DynValue::Null),
    ];
    // 引擎差异：Java map 含键 3（Integer）与 null 键（LinkedHashMap 支持非
    // 字符串键）；v1 DynValue::Map 键恒为 String——仅字符串键用例可对齐
    let model = SIMPLE_WRAPPER
        .wrap(&DynValue::Map(map.clone()))
        .expect("包装不应失败")
        .expect("非 null");

    let unwrapped = unwrap(&model);
    match &unwrapped {
        DynValue::Map(pairs) => {
            // Java：assertSame(map, unwrapped) —— v1 值类型，断言内容与顺序
            assert_eq!(pairs.len(), 2);
            let get = |k: &str| pairs.iter().find(|(key, _)| key == k).map(|(_, v)| v);
            assert_eq!(get("k1"), Some(&DynValue::Str("v1".to_string())));
            assert_eq!(get("k2"), Some(&DynValue::Null));
            // Java：键序保持（"k1"、"k2"、3、null）——v1 字符串键序一致
            let keys: Vec<&String> = pairs.iter().map(|(k, _)| k).collect();
            assert_eq!(keys, vec!["k1", "k2"]);
        }
        other => panic!("期望 Map，得到 {other:?}"),
    }

    // Java：unwrap(纯 TemplateHashModelEx2) 返回新对象（不等同原 map）——
    // v1 from_hash 恒为纯哈希，unwrap 恒产新 DynValue（内容相等），
    // "同一实例"断言无对应
    let pure = freemarker::template::TModel::from_hash(
        map.iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    SIMPLE_WRAPPER
                        .wrap(v)
                        .unwrap()
                        .unwrap_or_else(freemarker::template::TModel::nothing),
                )
            })
            .collect(),
    );
    match unwrap(&pure) {
        DynValue::Map(pairs) => {
            let get = |k: &str| pairs.iter().find(|(key, _)| key == k).map(|(_, v)| v);
            assert_eq!(get("k1"), Some(&DynValue::Str("v1".to_string())));
            assert_eq!(get("k2"), Some(&DynValue::Null));
        }
        other => panic!("期望 Map，得到 {other:?}"),
    }
}
