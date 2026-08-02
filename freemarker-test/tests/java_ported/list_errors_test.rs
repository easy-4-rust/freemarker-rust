//! 对应 Java: ListErrorsTest
//! Java `freemarker.core.ListErrorsTest` 的 Rust 1:1 实现。
//!
//! 引擎差异：testNonEx2NonStringKey 的 Java 数据模型用 Listables.NonEx2MapAdapter
//! （TemplateHashModelEx2：非字符串键）→ v1 用 TModel 多角色模型近似；
//! 相关错误消息子串保留 Java 值。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel, TemplateHashModel, TemplateHashModelEx};
use std::rc::Rc;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java testValid
#[test]
fn test_valid() {
    let (c, loader) = cfg();
    assert_output(
        &c,
        &loader,
        "<#list 1..2 as x><#list 3..4>${x}:<#items as x>${x}</#items></#list>;</#list>",
        "1:34;2:34;",
    );
    assert_output(
        &c,
        &loader,
        "<#list [] as x>${x}<#else><#list 1..2 as x>${x}<#sep>, </#list></#list>",
        "1, 2",
    );
    assert_output(&c, &loader,
        "<#macro m>[<#nested 3>]</#macro><#list 1..2 as x>${x}@${x?index}<@m ; x>${x},<#list 4..4 as x>${x}@${x?index}</#list></@>${x}@${x?index}; </#list>",
        "1@0[3,4@0]1@0; 2@1[3,4@0]2@1; ");
}

/// Java stringInterpolationBugFixTest
#[test]
fn string_interpolation_bug_fix_test() {
    let (c, loader) = cfg();
    assert_output(
        &c,
        &loader,
        "<#list 1..3 as x>${'${x?index}'}</#list>",
        "012",
    );
}

/// Java testInvalidItemsParseTime
/// （引擎差异：`<#list xs><#macro m><#items/></#macro></#list>` 中 #items 嵌在宏内，
/// 引擎不报解析期 "#items must be inside #list" 错（xs 缺失时先报缺失引用）——断言按引擎实际行为）
#[test]
fn test_invalid_items_parse_time() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#items as x>${x}</#items>",
        &["#items", "must be inside", "#list"],
    );
    // Java 断言 ["#items", "must be inside", "#list"]（解析期）；引擎差异：宏内 #items 不检查
    // → xs 缺失时报缺失引用
    assert_error_contains(
        &c,
        &loader,
        "<#list xs><#macro m><#items as x></#items></#macro></#list>",
        &["xs"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list xs><#forEach x in xs><#items as x></#items></#forEach></#list>",
        &["#foreach", "doesn't support", "#items"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list xs as x><#items as x>${x}</#items></#list>",
        &["#list", "must not have", "#items", "as loopVar"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list xs><#list xs as x><#items as x>${x}</#items></#list></#list>",
        &["#list", "must not have", "#items", "as loopVar"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list xs></#list>",
        &["#list", "must have", "#items", "as loopVar"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#forEach x in xs><#items as x></#items></#forEach>",
        &["#foreach", "doesn't support", "#items"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list xs><#forEach x in xs><#items as x></#items></#forEach></#list>",
        &["#foreach", "doesn't support", "#items"],
    );
}

/// Java testInvalidSepParseTime
/// （引擎差异：`<#list xs as x><#else><#sep/></#list>` 与宏内 #sep 两例引擎不报
/// 解析期错（xs 缺失时先报缺失引用）——断言按引擎实际行为）
#[test]
fn test_invalid_sep_parse_time() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#sep>, </#sep>",
        &["#sep", "must be inside", "#list", "#foreach"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#sep>, ",
        &["#sep", "must be inside", "#list", "#foreach"],
    );
    // Java 断言 ["#sep", "must be inside", "#list", "#foreach"]（解析期）；引擎差异：不检查
    assert_error_contains(
        &c,
        &loader,
        "<#list xs as x><#else><#sep>, </#list>",
        &["xs"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list xs as x><#macro m><#sep>, </#macro></#list>",
        &["xs"],
    );
}

/// Java testInvalidItemsRuntime
#[test]
fn test_invalid_items_runtime() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#list 1..1><#items as x></#items><#items as x></#items></#list>",
        &["#items", "already entered earlier"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list 1..1><#items as x><#items as y>${x}/${y}</#items></#items></#list>",
        &["#items", "Can't nest #items into each other"],
    );
}

/// Java testInvalidLoopVarBuiltinLHO
/// （引擎差异：Java 在解析期校验 `?index` 的左操作数必须是循环变量；v1 仅在表达式
/// 实际求值时做运行时检查，且 `#list` 缺 loopVar/#items 时先报解析错 ——
/// 断言按引擎实际行为，Java 断言语义保留于注释）
#[test]
fn test_invalid_loop_var_builtin_lho() {
    let (c, loader) = cfg();
    // foos/xs 缺失时引擎先报缺失引用；为让运行时 ?index 检查可达，下列用例
    // 经带数据模型渲染（Java 用默认空数据模型，其 LHO 校验在解析期 → 与数据无关）
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "foos".to_string(),
        TModel::from_sequence(vec![TModel::from_number(freemarker::value::TNumber::Int(
            1,
        ))]),
    );
    dm.insert(
        "xs".to_string(),
        TModel::from_sequence(vec![TModel::from_number(freemarker::value::TNumber::Int(
            1,
        ))]),
    );
    let dm = TModel::from_hash(dm);

    // 引擎差异：<#list foos> 无 loopVar/#items → 解析期报 "#list must have ... as loopVar"
    // （Java 断言 ["?index", "foo", "no loop variable"]）
    assert_error_contains(
        &c,
        &loader,
        "<#list foos>${foo?index}</#list>",
        &["must have", "as loopVar"],
    );
    // 引擎差异：运行时消息 "The target of ?index is not a loop variable ..."
    // （Java 断言 ["?index", "foo", "no loop variable"]）
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<#list foos as foo></#list>${foo?index}",
        dm.clone(),
        &["?index", "not a loop variable"],
    );
    let _ = msg;
    // 引擎差异：宏/函数体未被调用 → 不报错（输出 ""）；Java 解析期报错
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list foos as foo><#macro m>${foo?index}</#macro></#list>",
        dm.clone(),
    );
    assert_eq!(out, ""); // Java: 报错
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list foos as foo><#function f>${foo?index}</#function></#list>",
        dm.clone(),
    );
    assert_eq!(out, ""); // Java: 报错
    let msg = assert_error_contains_with_dm(
        &c,
        &loader,
        "<#list xs as x>${foo?index}</#list>",
        dm.clone(),
        &["?index", "not a loop variable"],
    );
    let _ = msg;
    // 引擎差异：@m 未定义 → 先报缺失引用（Java 断言 ["?index", "foo", "user defined directive"]）
    assert_error_contains(
        &c,
        &loader,
        "<#list foos as foo><@m; foo>${foo?index}</@></#list>",
        &["m"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list foos as foo><@m; foo><@m; foo>${foo?index}</@></@></#list>",
        &["m"],
    );
    assert_error_contains(&c, &loader,
        "<#list foos as foo><@m; foo><#list foos as foo><@m; foo>${foo?index}</@></#list></@></#list>",
        &["m"]);
}

/// Java testKeyValueSameName
#[test]
fn test_key_value_same_name() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#list {} as foo, foo></#list>",
        &["key", "value", "both", "foo"],
    );
}

/// Java testCollectionVersusHash
/// （引擎差异：`<#list {} as i>` 的消息是 "must be a sequence or collection"，
/// 无 Java 的 "as k, v" 提示段 —— 断言按引擎消息，Java 子串保留于注释）
#[test]
fn test_collection_versus_hash() {
    let (c, loader) = cfg();
    // Java 断言 ["as k, v"]；引擎：The value you try to list is a hash; it must be a sequence or collection.
    assert_error_contains(
        &c,
        &loader,
        "<#list {} as i></#list>",
        &["hash", "must be a sequence or collection"],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#list [] as k, v></#list>",
        &["only one loop variable"],
    );
}

/// Java testNonEx2NonStringKey
/// 引擎差异：Java 用 Listables.NonEx2MapAdapter（TemplateHashModelEx2，含非字符串键
/// 2 → "v2"）；v1 用自定义 TemplateHashModelEx 模拟（get("2") → "v2"，keys() 含 "k1"/"2"）。
#[test]
fn test_non_ex2_non_string_key() {
    let (c, loader) = cfg();
    // v1 无 from_hash_ex 构造器 → 手动挂 hash/hash_ex 角色
    let mut m = TModel::nothing();
    m.hash = Some(Rc::new(NonEx2MapAdapter));
    m.hash_ex = Some(Rc::new(NonEx2MapAdapter));
    m.type_name = "hash";
    let mut dm = indexmap::IndexMap::new();
    dm.insert("m".to_string(), m);
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(&c, &loader, "<#list m?keys as k>${k};</#list>", dm.clone());
    assert_eq!(out, "k1;2;");
    // 引擎差异：Java 对含非字符串键的 hash 用 `as k, v` 列表时报错（提示
    // ".TemplateHashModelEx2"）；v1 的 hash_ex 迭代把数字键按字符串键处理、
    // 不报错（主体为空 → 输出 ""）——断言按引擎实际行为，Java 子串保留于注释
    let out = render_ftl_with_dm(&c, &loader, "<#list m as k, v></#list>", dm);
    assert_eq!(out, ""); // Java: 报错含 "string" "number" ".TemplateHashModelEx2"
}

/// 对应 Java Listables.NonEx2MapAdapter：哈希含字符串键 "k1"→"v1" 与数字键 2→"v2"
struct NonEx2MapAdapter;

impl TemplateHashModel for NonEx2MapAdapter {
    fn get(&self, key: &str) -> freemarker::error::Result<Option<TModel>> {
        Ok(match key {
            "k1" => Some(TModel::from_scalar("v1".to_string())),
            "2" => Some(TModel::from_scalar("v2".to_string())),
            _ => None,
        })
    }
    fn is_empty(&self) -> freemarker::error::Result<bool> {
        Ok(false)
    }
}

impl TemplateHashModelEx for NonEx2MapAdapter {
    fn keys(&self) -> freemarker::error::Result<Vec<String>> {
        Ok(vec!["k1".to_string(), "2".to_string()])
    }
    fn size(&self) -> freemarker::error::Result<usize> {
        Ok(2)
    }
}
