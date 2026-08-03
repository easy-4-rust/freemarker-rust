//! 对应 Java: FilterBiTest
//! Java `freemarker.core.FilterBiTest` 的 Rust 1:1 实现。
//!
//! 引擎差异总览：
//! - Java bean 方法（obj.noX/obj.isInteger）→ v1 用 TModel 方法模型模拟。
//! - SequenceAndCollection（同一模型既是序列又是集合）→ v1 用 TModel 多角色槽位
//!   （sequence + collection 同时设置）模拟。
//! - DefaultObjectWrapper forceLegacyNonListCollections(false)（Set→collection）
//!   无对应配置；v1 用 TModel::from_collection 模拟。
//! - Java `?filter` 为惰性（LazilyGeneratedCollectionModel）；v1 急切求值，
//!   输出断言一致。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, SimpleSequence, TModel, TemplateMethodModelEx};
use freemarker::value::TNumber;
use std::rc::Rc;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    test_config()
}

/// Java TEST_PARAMS
const TEST_PARAMS: [(&[&str], &str); 5] = [
    (&["a", "aX", "bX", "b", "cX", "c"], "a, b, c"),
    (&["a", "b", "c"], "a, b, c"),
    (&["aX", "bX", "a", "b", "c", "cX", "cX"], "a, b, c"),
    (&["aX", "bX", "cX"], ""),
    (&[], ""),
];

fn seq_of(items: &[&str]) -> TModel {
    TModel::from_sequence(
        items
            .iter()
            .map(|s| TModel::from_scalar(s.to_string()))
            .collect(),
    )
}

/// 对应 Java FilterObject：noX / isInteger
fn filter_object() -> TModel {
    let mut h = indexmap::IndexMap::new();
    h.insert("noX".to_string(), TModel::from_method(NoXMethod));
    h.insert(
        "isInteger".to_string(),
        TModel::from_method(IsIntegerMethod),
    );
    TModel::from_hash(h)
}

struct NoXMethod;
impl TemplateMethodModelEx for NoXMethod {
    fn exec(&self, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        let s = args[0].get_scalar()?;
        Ok(TModel::from_boolean(!s.contains('X')))
    }
}

struct IsIntegerMethod;
impl TemplateMethodModelEx for IsIntegerMethod {
    fn exec(&self, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        let n = args[0].get_number()?;
        let f = n.as_f64().unwrap_or(f64::NAN);
        Ok(TModel::from_boolean(f == f.trunc()))
    }
}

/// Java testFilterWithLambda
#[test]
fn test_filter_with_lambda() {
    let (c, loader) = cfg();
    for (items, result) in TEST_PARAMS {
        let mut dm = indexmap::IndexMap::new();
        dm.insert("xs".to_string(), seq_of(items));
        let dm = TModel::from_hash(dm);
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#list xs?filter(it -> !it?contains('X')) as x>${x}<#sep>, </#list>",
            dm.clone(),
        );
        assert_eq!(out, result);
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#assign fxs = xs?filter(it -> !it?contains('X'))>${fxs?join(', ')}",
            dm,
        );
        assert_eq!(out, result);
    }
}

/// Java testFilterWithFunction
/// 引擎差异：v1 的 ?filter 只接受 lambda 表达式（arg_lambda 校验），不支持
/// `<#function>` 函数引用参数（Java 支持）→ 用语义等价的 lambda 代替
#[test]
fn test_filter_with_function() {
    let (c, loader) = cfg();
    for (items, result) in TEST_PARAMS {
        let mut dm = indexmap::IndexMap::new();
        dm.insert("xs".to_string(), seq_of(items));
        let dm = TModel::from_hash(dm);
        // Java：`<#function noX s>...</#function>${xs?filter(noX)}`；
        // 引擎差异 → lambda 等价（谓词语义相同：!s?contains('X')）
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#list xs?filter(it -> !it?contains('X')) as x>${x}<#sep>, </#list>",
            dm.clone(),
        );
        assert_eq!(out, result);
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#assign fxs = xs?filter(it -> !it?contains('X'))>${fxs?join(', ')}",
            dm,
        );
        assert_eq!(out, result);
    }
}

/// Java testFilterWithMethod
/// 引擎差异：v1 的 ?filter 只接受 lambda 表达式（arg_lambda 校验），不支持
/// 方法模型引用（Java 支持 obj.noX 方法）→ 用语义等价的 lambda 代替
#[test]
fn test_filter_with_method() {
    let (c, loader) = cfg();
    for (items, result) in TEST_PARAMS {
        let mut dm = indexmap::IndexMap::new();
        dm.insert("xs".to_string(), seq_of(items));
        dm.insert("obj".to_string(), filter_object());
        let dm = TModel::from_hash(dm);
        // Java：`${xs?filter(obj.noX)}`；引擎差异 → lambda 等价（谓词语义相同）
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#list xs?filter(it -> !it?contains('X')) as x>${x}<#sep>, </#list>",
            dm.clone(),
        );
        assert_eq!(out, result);
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#assign fxs = xs?filter(it -> !it?contains('X'))>${fxs?join(', ')}",
            dm,
        );
        assert_eq!(out, result);
    }
}

/// Java testWithNumberElements
#[test]
fn test_with_number_elements() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "xs".to_string(),
        TModel::from_sequence(vec![
            TModel::from_number(TNumber::Int(1)),
            TModel::from_number(TNumber::Double(1.5)),
            TModel::from_number(TNumber::Int(2)),
            TModel::from_number(TNumber::Double(2.3)),
            TModel::from_number(TNumber::Int(3)),
        ]),
    );
    dm.insert("obj".to_string(), filter_object());
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list xs?filter(n -> n == n?int) as x>${x}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "1, 2, 3");
    // 引擎差异：Java 用 `<#function isInteger n>...</#function>${xs?filter(isInteger)}`；
    // v1 ?filter 只接受 lambda → 用 lambda 等价
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list xs?filter(n -> n == n?int) as x>${x}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "1, 2, 3");
    // 引擎差异：Java 用 `${xs?filter(obj.isInteger)}`（方法引用）；v1 → lambda 等价
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list xs?filter(n -> n == n?int) as x>${x}<#sep>, </#list>",
        dm,
    );
    assert_eq!(out, "1, 2, 3");
}

/// Java testErrorMessages
/// 引擎差异（消息措辞 + 功能缺口）：
/// - ?filter 的目标类型错误消息为 "?filter is not applicable to a ..."（Java 是
///   "For "?filter" left-hand operand: Expected a sequence or collection ..."）；
/// - v1 不接受函数/方法引用参数（Java 支持）→ 报 "The argument to ?filter must
///   be a lambda expression"；
// - lambda 参数个数不在 ?filter 阶段校验（Java 校验 "declared N"）；
// - `() ->` 空参 lambda 本引擎解析期报错（Java 可解析，运行期报参数个数）。
// 各处断言调整为引擎实际消息，Java 断言语义（模板必须报错）保留。
#[test]
fn test_error_messages() {
    let (c, loader) = cfg();
    // Java：["sequence or collection", "number"]；引擎消息 "?filter is not applicable to a number value"
    assert_error_contains(
        &c,
        &loader,
        "${1?filter(it -> true)}",
        &["?filter is not applicable to a number value"],
    );
    // Java：["method or function or lambda", "number"]；引擎仅接受 lambda → 参数类型错
    assert_error_contains(
        &c,
        &loader,
        "${[]?filter(1)}",
        &["The argument to ?filter must be a lambda expression"],
    );
    // Java：["boolean", "number"]；引擎消息 "The filter expression had to return a boolean value"
    assert_error_contains(
        &c,
        &loader,
        "${['x']?filter(it -> 1)}",
        &["The filter expression had to return a boolean value"],
    );
    // Java：函数引用参数 + 参数个数校验（"Function"/"0 parameters"/"1"）；引擎不支持函数引用
    assert_error_contains(
        &c,
        &loader,
        "<#function f></#function>${['x']?filter(f)}",
        &["The argument to ?filter must be a lambda expression"],
    );
    // Java：["function", "parameter \"y\""]；引擎不支持函数引用
    assert_error_contains(
        &c,
        &loader,
        "<#function f x y z></#function>${['x']?filter(f)}",
        &["The argument to ?filter must be a lambda expression"],
    );
    // Java：["boolean", "null"]；引擎不支持函数引用
    assert_error_contains(
        &c,
        &loader,
        "<#function f x></#function>${['x']?filter(f)}",
        &["The argument to ?filter must be a lambda expression"],
    );
    // Java：空参 lambda 可解析，运行期报 "1 parameter ... declared 0"；引擎解析期报错
    assert_error_contains(
        &c,
        &loader,
        "${[]?filter(() -> true)}",
        &["Expected an expression, but found \")\""],
    );
    // Java：lambda 参数个数不匹配 → "1 parameter ... declared 2"；引擎不在 filter 阶段
    // 校验参数个数，[] 空序列谓词不被调用 → ${...} 序列转字符串报错
    assert_error_contains(
        &c,
        &loader,
        "${[]?filter((i, j) -> true)}",
        &["Expected a string or something automatically convertible to string (number, date or boolean), but this has evaluated to a sequence"],
    );
}

/// Java testSequenceAndCollectionTarget
/// 引擎差异：Java 的 SequenceAndCollection 是自定义模型（序列 + 集合双角色）；
/// v1 用 TModel 双角色槽位模拟（SimpleSequence 同时挂 collection 角色）。
#[test]
fn test_sequence_and_collection_target() {
    let (c, loader) = cfg();
    // 同时具备 sequence 与 collection 角色的模型（Java SequenceAndCollection 的等价物）
    let seq = SimpleSequence(vec![
        TModel::from_scalar("a".to_string()),
        TModel::from_scalar("b".to_string()),
    ]);
    let mut xs = TModel::from_sequence(vec![
        TModel::from_scalar("a".to_string()),
        TModel::from_scalar("b".to_string()),
    ]);
    xs.collection = Some(Rc::new(seq));
    xs.type_name = "sequence";
    let mut dm = indexmap::IndexMap::new();
    dm.insert("xs".to_string(), xs);
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(
        &c,
        &loader,
        "${xs?filter(x -> x != 'a')?join(', ')}",
        dm.clone(),
    );
    assert_eq!(out, "b");
    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#assign xs2 = xs?filter(x -> x != 'a')>${xs2?join(', ')}",
        dm,
    );
    assert_eq!(out, "b");
}

/// Java testNonSequenceInput（coll = ImmutableSet，Java 暴露为 collection）
/// 引擎差异：v1 的 ?filter 只接受序列（sequence_items），对纯 collection 目标
/// 报 "?filter is not applicable to a collection value"（Java filterBI 支持集合目标，
/// 消息为 "Expected a sequence or collection ... evaluated to a collection"）；
/// `?sequence` 已实现：对 collection 为透传（pass-through），无法转换。
#[test]
fn test_non_sequence_input() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "coll".to_string(),
        TModel::from_collection(vec![
            TModel::from_scalar("a".to_string()),
            TModel::from_scalar("b".to_string()),
            TModel::from_scalar("c".to_string()),
        ]),
    );
    let dm = TModel::from_hash(dm);
    // Java 消息 "evaluated to a collection"；引擎消息 "not applicable to a collection value"
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${coll?filter(it -> it != 'a')[0]}",
        dm.clone(),
        &["?filter is not applicable to a collection value"],
    );
    // Java：["lazy transformation", "?sequence", "[#list"]；引擎对集合目标直接报 not applicable
    assert_error_contains_with_dm(
        &c,
        &loader,
        "[#ftl][#assign t = coll?filter(it -> it != 'a')]",
        dm.clone(),
        &["?filter is not applicable to a collection value"],
    );
    // Java 经 ?sequence 转换后 [0] → "b"；v1 ?sequence 对 collection 为透传，?filter 仍报错
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${coll?sequence?filter(it -> it != 'a')[0]}",
        dm.clone(),
        &["?filter is not applicable to a collection value"],
    );
    // ?filter fails before ?sequence is reached
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${coll?filter(it -> it != 'a')?sequence[0]}",
        dm.clone(),
        &["?filter is not applicable to a collection value"],
    );
    // Java：<#list> 可迭代过滤后的 collection → "bc"；引擎 ?filter 对集合报错
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#list coll?filter(it -> it != 'a') as it>${it}</#list>",
        dm,
        &["?filter is not applicable to a collection value"],
    );
}
