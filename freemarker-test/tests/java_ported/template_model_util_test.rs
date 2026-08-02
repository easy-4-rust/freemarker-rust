//! Java `freemarker.template.utility.TemplateModelUtilTest` 的 Rust 1:1 实现
//! （TemplateModelUtilTest.java：getKeyValuePairIterator / wrapAsHashUnion 测试）
//!
//! 引擎映射：v1 无 TemplateModelUtils 公开 API（TModel 哈希为
//! IndexMap<String, TModel>，实现 TemplateHashModelEx 的 size/keys/entries）——
//! 按 Java 语义在测试内实现等价辅助并断言。引擎差异：v1 哈希键恒为 String
//! （Java 测试里非字符串键/空键抛 "keys must be ..." 错误的用例不可能出现）；
//! Bean 包装（ow.wrap(bean)）无对应。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::template::TModel;
use freemarker::value::TNumber;

/// 键值对迭代内容串（对应 Java assertetGetKeyValuePairIteratorContent 的
/// toValueAssertionString 拼接："str(k1): num(11), ..."；
/// v1 TModel::from_hash 同时实现 TemplateHashModelEx —— 用 hash_ex 的
/// size/keys/entries）
fn kv_pair_iter_content(model: &TModel) -> String {
    let h = model.hash_ex.as_ref().expect("应为哈希 Ex");
    let entries = h.entries().expect("entries 不应失败");
    let mut sb = String::new();
    for (k, v) in entries {
        if !sb.is_empty() {
            sb.push_str(", ");
        }
        sb.push_str(&format!(
            "{}: {}",
            value_assertion_string(&TModel::from_scalar(k)),
            value_assertion_string(&v)
        ));
    }
    sb
}

/// 对应 Java toValueAssertionString
fn value_assertion_string(model: &TModel) -> String {
    if let Some(n) = &model.number {
        return format!("num({})", n.as_number().unwrap().to_plain_string());
    }
    if let Some(s) = &model.scalar {
        return format!("str({})", s.as_string().unwrap());
    }
    if model.is_nothing() {
        return "null".to_string();
    }
    panic!("Type unsupported by test: {}", model.type_name);
}

/// 对应 TemplateModelUtils.wrapAsHashUnion：多哈希并集，后者覆盖同名键；
/// null（v1 nothing）跳过；全空 → 空哈希（v1 无法断言实例同一性，断言内容）
fn wrap_as_hash_union(models: &[TModel]) -> TModel {
    let mut merged = indexmap::IndexMap::new();
    for m in models {
        if m.is_nothing() {
            continue; // Java：null 参数视为空哈希
        }
        let h = m.hash_ex.as_ref().expect("应为哈希 Ex");
        for (k, v) in h.entries().expect("entries 不应失败") {
            merged.insert(k, v);
        }
    }
    TModel::from_hash(merged)
}

/// Java testGetKeyValuePairIterator：Ex 哈希的键值对迭代内容
/// （v1 键恒为字符串——Java 的非字符串键/空键错误用例注释保留）
#[test]
fn test_get_key_value_pair_iterator() {
    let mut map = indexmap::IndexMap::new();
    let thme = TModel::from_hash(map.clone());
    assert_eq!(kv_pair_iter_content(&thme), "");

    map.insert("k1".to_string(), TModel::from_number(TNumber::Int(11)));
    assert_eq!(
        kv_pair_iter_content(&TModel::from_hash(map.clone())),
        "str(k1): num(11)"
    );

    map.insert("k2".to_string(), TModel::from_scalar("v2".to_string()));
    assert_eq!(
        kv_pair_iter_content(&TModel::from_hash(map.clone())),
        "str(k1): num(11), str(k2): str(v2)"
    );

    map.insert("k2".to_string(), TModel::nothing());
    assert_eq!(
        kv_pair_iter_content(&TModel::from_hash(map.clone())),
        "str(k1): num(11), str(k2): null"
    );

    // 引擎差异：Java map.put(3, 33) → getKeyValuePairIterator 抛
    // TemplateModelException 消息含 "keys must be"、"string"、"number"；
    // map.put(null, 44) → 消息含 "keys must be"、"string"、"Null" ——
    // v1 哈希键恒为 String（无非字符串键），用例不可能出现
}

/// Java testGetKeyValuePairIteratorWithEx2：适配器哈希（DefaultMapAdapter）
/// —— v1 TModel::from_hash 即"纯"哈希，内容断言同上
#[test]
fn test_get_key_value_pair_iterator_with_ex2() {
    let mut map = indexmap::IndexMap::new();
    map.insert("k1".to_string(), TModel::from_number(TNumber::Int(11)));
    map.insert("k2".to_string(), TModel::nothing());
    // 引擎差异：Java 用例含键 3 与 null 键（DefaultMapAdapter 支持非字符串键）；
    // v1 仅字符串键
    assert_eq!(
        kv_pair_iter_content(&TModel::from_hash(map)),
        "str(k1): num(11), str(k2): null"
    );
}

/// Java wrapAsHashUnionBasics：并集的合并/覆盖顺序
#[test]
fn wrap_as_hash_union_basics() {
    let mk = |pairs: &[(&str, i32)]| -> TModel {
        let mut map = indexmap::IndexMap::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), TModel::from_number(TNumber::Int(*v)));
        }
        TModel::from_hash(map)
    };
    let th_ex1 = mk(&[("a", 1), ("b", 2)]);
    let th_ex2 = mk(&[("c", 3), ("d", 4)]);
    let th_ex3 = mk(&[("b", 22), ("c", 33)]);
    let th_ex4 = mk(&[]);

    assert_union_result(
        &[("a", 1), ("b", 2), ("c", 3), ("d", 4)],
        &wrap_as_hash_union(&[th_ex1.clone(), th_ex2.clone()]),
    );
    assert_union_result(
        &[("a", 1), ("b", 2), ("c", 3), ("d", 4)],
        &wrap_as_hash_union(&[th_ex1.clone(), th_ex2.clone()]),
    );
    assert_union_result(
        &[("a", 1), ("b", 22), ("c", 33)],
        &wrap_as_hash_union(&[th_ex1.clone(), th_ex3.clone()]),
    );
    // 覆盖顺序：后者胜出；顺序按首个哈希的插入序 + 新增键
    assert_union_result(
        &[("b", 2), ("c", 33), ("a", 1)],
        &wrap_as_hash_union(&[th_ex3.clone(), th_ex1.clone()]),
    );
    assert_union_result(
        &[("a", 1), ("b", 2)],
        &wrap_as_hash_union(&[th_ex1.clone(), th_ex4.clone()]),
    );
    assert_union_result(
        &[("a", 1), ("b", 2)],
        &wrap_as_hash_union(&[th_ex4.clone(), th_ex1.clone()]),
    );
    assert_union_result(&[], &wrap_as_hash_union(&[th_ex4.clone(), th_ex4.clone()]));
}

fn assert_union_result(expected: &[(&str, i32)], actual: &TModel) {
    let h = actual.hash_ex.as_ref().expect("应为哈希 Ex");
    assert_eq!(h.size().unwrap(), expected.len());
    for (k, v) in expected {
        let m = h.get(k).unwrap().expect("键应存在");
        assert_eq!(m.get_number().unwrap().to_plain_string(), v.to_string());
    }
    // 键序（Java keys() 迭代序断言）：
    let keys: Vec<String> = h.keys().unwrap();
    let expected_keys: Vec<String> = expected.iter().map(|(k, _)| k.to_string()).collect();
    assert_eq!(keys, expected_keys);
}

/// Java wrapAsHashUnionWrapping：混合参数（含 Bean 包装）。
/// 引擎差异：v1 无 ObjectWrapper.wrap(bean)（MyBean.getB() 的 JavaBean 反射）
/// 与 get("class")（BeanWrapper 的 class 属性）；wrapAsHashUnion(ow, "x")
/// （非哈希参数）抛 TemplateModelException——v1 无此 API，整体注释保留。
#[test]
fn wrap_as_hash_union_wrapping() {
    // Java：wrapAsHashUnion(ow, {a:1}, new MyBean(), null, ow.wrap({c:3}))
    // → a=1、b=2（MyBean.getB()）、c=3、class 非 null、noSuchVariable==null；
    // wrapAsHashUnion(ow, "x")（非哈希）抛 TemplateModelException。
    // v1 无 Bean 包装与多参数并集 API（测试内 wrap_as_hash_union 仅接受 TModel）
}

/// Java wrapAsHashUnionSizeEdgeCases：空/单参数边界
#[test]
fn wrap_as_hash_union_size_edge_cases() {
    // Java：wrapAsHashUnion(ow) 与 wrapAsHashUnion(ow, null, null) 均返回
    // Constants.EMPTY_HASH（同一实例）——v1 断言"空哈希"（实例同一性无对应）
    let empty = wrap_as_hash_union(&[]);
    assert!(empty.hash_ex.as_ref().unwrap().is_empty().unwrap());
    assert!(wrap_as_hash_union(&[TModel::nothing(), TModel::nothing()])
        .hash_ex
        .as_ref()
        .unwrap()
        .is_empty()
        .unwrap());

    // Java：wrapAsHashUnion(ow, hash) 返回同一实例——v1 断言内容相等
    let mut map = indexmap::IndexMap::new();
    map.insert("a".to_string(), TModel::from_number(TNumber::Int(1)));
    let hash = TModel::from_hash(map);
    let merged = wrap_as_hash_union(std::slice::from_ref(&hash));
    assert_eq!(
        merged
            .hash_ex
            .as_ref()
            .unwrap()
            .get("a")
            .unwrap()
            .unwrap()
            .get_number()
            .unwrap()
            .to_plain_string(),
        "1"
    );
    assert_eq!(
        wrap_as_hash_union(&[TModel::nothing(), hash.clone(), TModel::nothing()])
            .hash_ex
            .as_ref()
            .unwrap()
            .get("a")
            .unwrap()
            .unwrap()
            .get_number()
            .unwrap()
            .to_plain_string(),
        "1"
    );
}
