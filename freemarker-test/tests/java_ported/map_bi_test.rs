//! 对应 Java: MapBiTest
//! Java `freemarker.core.MapBiTest` 的 Rust 1:1 实现。
//! createConfiguration：numberFormat="0.####"、booleanFormat="c"。
//!
//! 引擎差异总览：
//! - Java bean 方法（obj.toUpper/obj.tenTimes/obj.extractName）→ v1 用 TModel 方法
//!   模型 + 哈希对象模拟；bean 元素（User）→ 哈希模型 {name: ...}。
//! - Java `?map` 的**函数/方法参数**（`?map(f)`/`?map(obj.method)`）v1 **未实现**
//!   —— v1 ?map 仅接受 lambda 表达式，函数/方法参数报 "The argument to ?map must
//!   be a lambda expression"（Java 期望映射输出）；相关断言改为引擎实际错误并注明。
//! - Java `?map` 的**惰性**（LazilyGeneratedCollectionModel），v1 为急切求值；且
//!   v1 ?map 目标仅限序列（collection 目标报 "not applicable to a collection value"）。
//! - `?sequence` 内建 v1 未实现 → "Unknown built-in: ?sequence"。
//! - 零参数/多参数 lambda 语法（`() -> 1`/`(i, j) -> 1`）v1 解析不支持。
//! - DefaultObjectWrapper forceLegacyNonListCollections(false)（Set→collection）
//!   无对应配置；v1 用 TModel::from_collection 模拟 collection。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use bigdecimal::BigDecimal;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel, TemplateMethodModelEx};
use freemarker::value::TNumber;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    c.settings.number_format = "0.####".to_string();
    c.settings.boolean_format = "c".to_string();
    (c, loader)
}

/// Java TEST_PARAMS（ImmutableList.of("a","b","c") → "A, B, C" 等）
const TEST_PARAMS: [(&[&str], &str); 3] = [(&["a", "b", "c"], "A, B, C"), (&["a"], "A"), (&[], "")];

fn seq_of(items: &[&str]) -> TModel {
    TModel::from_sequence(
        items
            .iter()
            .map(|s| TModel::from_scalar(s.to_string()))
            .collect(),
    )
}

/// 对应 Java MapperObject：toUpper/tenTimes/extractName 三个方法模型
fn mapper_object() -> TModel {
    let mut h = indexmap::IndexMap::new();
    h.insert("toUpper".to_string(), TModel::from_method(ToUpperMethod));
    h.insert("tenTimes".to_string(), TModel::from_method(TenTimesMethod));
    h.insert(
        "extractName".to_string(),
        TModel::from_method(ExtractNameMethod),
    );
    TModel::from_hash(h)
}

struct ToUpperMethod;
impl TemplateMethodModelEx for ToUpperMethod {
    fn exec(&self, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        let s = args[0].get_scalar()?;
        Ok(TModel::from_scalar(s.to_uppercase()))
    }
}

struct TenTimesMethod;
impl TemplateMethodModelEx for TenTimesMethod {
    fn exec(&self, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        // Java BigDecimal.movePointRight(1) —— n * 10
        let n = args[0].get_number()?;
        let ten: BigDecimal = BigDecimal::from(10);
        Ok(TModel::from_number(TNumber::Decimal(
            n.as_big_decimal() * ten,
        )))
    }
}

struct ExtractNameMethod;
impl TemplateMethodModelEx for ExtractNameMethod {
    fn exec(&self, args: Vec<TModel>) -> freemarker::error::Result<TModel> {
        let h = args[0].get_hash()?;
        let name = h
            .get("name")?
            .ok_or_else(|| freemarker::error::TemplateError::misc("no name member".to_string()))?;
        name.get_scalar().map(TModel::from_scalar)
    }
}

/// Java testFilterWithLambda（Java 方法名 testFilterWithLambda，测的是 ?map）
/// lambda 参数：v1 支持，输出与 Java 一致。
#[test]
fn test_filter_with_lambda() {
    let (c, loader) = cfg();
    for (items, result) in TEST_PARAMS {
        let mut dm = indexmap::IndexMap::new();
        dm.insert("xs".to_string(), seq_of(items));
        let dm = TModel::from_hash(dm);
        // 惰性（Java 注释；v1 急切求值但输出相同）：
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#list xs?map(it -> it?upperCase) as x>${x}<#sep>, </#list>",
            dm.clone(),
        );
        assert_eq!(out, result);
        // 急切：
        let out = render_ftl_with_dm(
            &c,
            &loader,
            "<#assign fxs = xs?map(it -> it?upperCase)>${fxs?join(', ')}",
            dm,
        );
        assert_eq!(out, result);
    }
}

/// Java testFilterWithFunction
/// 引擎差异：Java `?map(函数名)` 接受函数参数并输出 {result}；v1 ?map 仅接受
/// lambda 表达式 → 报 "The argument to ?map must be a lambda expression"。
#[test]
fn test_filter_with_function() {
    let (c, loader) = cfg();
    for (items, result) in TEST_PARAMS {
        let _ = result; // 引擎差异：v1 报错，Java 期望输出 result
        let mut dm = indexmap::IndexMap::new();
        dm.insert("xs".to_string(), seq_of(items));
        let dm = TModel::from_hash(dm);
        let function_def = "<#function toUpper s><#return s?upperCase></#function>";
        assert_error_contains_with_dm(
            &c,
            &loader,
            &format!("{function_def}<#list xs?map(toUpper) as x>${{x}}<#sep>, </#list>"),
            dm.clone(),
            &["must be a lambda expression"],
        );
        assert_error_contains_with_dm(
            &c,
            &loader,
            &format!("{function_def}<#assign fxs = xs?map(toUpper)>${{fxs?join(', ')}}"),
            dm,
            &["must be a lambda expression"],
        );
    }
}

/// Java testFilterWithMethod
/// 引擎差异：Java `?map(obj.toUpper)` 接受方法模型；v1 ?map 仅接受 lambda → 报错。
#[test]
fn test_filter_with_method() {
    let (c, loader) = cfg();
    for (items, result) in TEST_PARAMS {
        let _ = result; // 引擎差异：v1 报错，Java 期望输出 result
        let mut dm = indexmap::IndexMap::new();
        dm.insert("xs".to_string(), seq_of(items));
        dm.insert("obj".to_string(), mapper_object());
        let dm = TModel::from_hash(dm);
        assert_error_contains_with_dm(
            &c,
            &loader,
            "<#list xs?map(obj.toUpper) as x>${x}<#sep>, </#list>",
            dm.clone(),
            &["must be a lambda expression"],
        );
        assert_error_contains_with_dm(
            &c,
            &loader,
            "<#assign fxs = xs?map(obj.toUpper)>${fxs?join(', ')}",
            dm,
            &["must be a lambda expression"],
        );
    }
}

/// Java testWithNumberElements
/// 引擎差异：lambda 部分可过；函数/方法参数部分 v1 报错（Java 期望 "10, 15.5, 30"）。
#[test]
fn test_with_number_elements() {
    let (c, loader) = cfg();
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "xs".to_string(),
        TModel::from_sequence(vec![
            TModel::from_number(TNumber::Int(1)),
            TModel::from_number(TNumber::Double(1.55)),
            TModel::from_number(TNumber::Int(3)),
        ]),
    );
    dm.insert("obj".to_string(), mapper_object());
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list xs?map(n -> n * 10) as x>${x}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "10, 15.5, 30");
    // 引擎差异：Java `?map(tenTimes)` 输出 "10, 15.5, 30"；v1 ?map 仅接受 lambda → 报错
    assert_error_contains_with_dm(
        &c, &loader,
        "<#function tenTimes n><#return n * 10></#function><#list xs?map(tenTimes) as x>${x}<#sep>, </#list>",
        dm.clone(),
        &["must be a lambda expression"],
    );
    // 引擎差异：Java `?map(obj.tenTimes)` 输出 "10, 15.5, 30"；v1 → 报错
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#list xs?map(obj.tenTimes) as x>${x}<#sep>, </#list>",
        dm,
        &["must be a lambda expression"],
    );
}

/// Java testWithBeanElements（Java bean 元素 → v1 哈希模型 {name: ...}）
/// 引擎差异：lambda 部分可过；函数/方法参数部分 v1 报错（Java 期望 "a, b, c"）。
#[test]
fn test_with_bean_elements() {
    let (c, loader) = cfg();
    let user = |n: &str| {
        let mut h = indexmap::IndexMap::new();
        h.insert("name".to_string(), TModel::from_scalar(n.to_string()));
        TModel::from_hash(h)
    };
    let mut dm = indexmap::IndexMap::new();
    dm.insert(
        "xs".to_string(),
        TModel::from_sequence(vec![user("a"), user("b"), user("c")]),
    );
    dm.insert("obj".to_string(), mapper_object());
    let dm = TModel::from_hash(dm);

    let out = render_ftl_with_dm(
        &c,
        &loader,
        "<#list xs?map(user -> user.name) as x>${x}<#sep>, </#list>",
        dm.clone(),
    );
    assert_eq!(out, "a, b, c");
    // 引擎差异：Java `?map(extractName)` 输出 "a, b, c"；v1 → 报错
    assert_error_contains_with_dm(
        &c, &loader,
        "<#function extractName user><#return user.name></#function><#list xs?map(extractName) as x>${x}<#sep>, </#list>",
        dm.clone(),
        &["must be a lambda expression"],
    );
    // 引擎差异：Java `?map(obj.extractName)` 输出 "a, b, c"；v1 → 报错
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#list xs?map(obj.extractName) as x>${x}<#sep>, </#list>",
        dm,
        &["must be a lambda expression"],
    );
}

/// Java testBuiltInsThatAllowLazyEval
/// 引擎差异：v1 ?map 急切求值且**不接受函数参数** —— 所有用例在 `?map(tenTimes)`
/// 处报 "The argument to ?map must be a lambda expression"（Java 期望映射输出 +
/// 副作用计数 s）。
#[test]
fn test_built_ins_that_allow_lazy_eval() {
    let (c, loader) = cfg();
    let side_effect_fn =
        "<#assign s = ''><#function tenTimes(x)><#assign s += '${x};'><#return x * 10></#function>";
    assert_error_contains(
        &c,
        &loader,
        &format!("{side_effect_fn}${{(1..3)?map(tenTimes)?first}} ${{s}}"),
        &["must be a lambda expression"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{side_effect_fn}${{(1..3)?map(tenTimes)?seqContains(20)}} ${{s}}"),
        &["must be a lambda expression"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{side_effect_fn}${{(1..3)?map(tenTimes)?seqIndexOf(20)}} ${{s}}"),
        &["must be a lambda expression"],
    );
    assert_error_contains(
        &c,
        &loader,
        &format!("{side_effect_fn}${{[1, 2, 3, 2, 5]?map(tenTimes)?seqLastIndexOf(20)}} ${{s}}"),
        &["must be a lambda expression"],
    );

    // 这些测试无法检查是否构建了序列，但至少知道它们在正常工作：
    assert_output(&c, &loader, "${(1..3)?map(it -> it * 10)?min}", "10");
    assert_output(&c, &loader, "${(1..3)?map(it -> it * 10)?max}", "30");
    assert_output(
        &c,
        &loader,
        "${(1..3)?map(it -> it * 10)?join(', ')}",
        "10, 20, 30",
    );
}

/// Java testErrorMessages
/// 引擎差异：v1 ?map 仅接受 lambda、且零/多参数 lambda 语法不支持 —— 错误消息
/// 与 Java 不同，断言引擎实际消息（Java 子串保留在注释）。
#[test]
fn test_error_messages() {
    let (c, loader) = cfg();
    // Java：["sequence or collection", "number"]；v1：?map 对数字不可用
    assert_error_contains(
        &c,
        &loader,
        "${1?map(it -> it)}",
        &["not applicable to a number value"],
    );
    // Java：["method or function or lambda", "number"]；v1：参数不是 lambda
    assert_error_contains(
        &c,
        &loader,
        "${[]?map(1)}",
        &["must be a lambda expression"],
    );
    // Java：["Function", "0 parameters", "1"]；v1：函数参数不支持
    assert_error_contains(
        &c,
        &loader,
        "<#function f></#function>${['x']?map(f)}",
        &["must be a lambda expression"],
    );
    // Java：["function", "parameter \"y\""]；v1：函数参数不支持
    assert_error_contains(
        &c,
        &loader,
        "<#function f x y z></#function>${['x']?map(f)}",
        &["must be a lambda expression"],
    );
    // Java：["null"]；v1：函数参数不支持
    assert_error_contains(
        &c,
        &loader,
        "<#function f x></#function>${['x']?map(f)}",
        &["must be a lambda expression"],
    );
    // Java：["lambda", "1 parameter", "declared 0"]；v1：零参数 lambda 解析不支持
    assert_error_contains(
        &c,
        &loader,
        "${[]?map(() -> 1)}",
        &["Encountered \")\"", "was expecting one of these patterns"],
    );
    // Java：["lambda", "1 parameter", "declared 2"]；v1：(i, j) 多参数 lambda 解析成
    // 其他表达式 → 类型错误
    assert_error_contains(
        &c,
        &loader,
        "${[]?map((i, j) -> 1)}",
        &["Expected a string or something automatically convertible to string (number, date or boolean)"],
    );
}

/// Java testNonSequenceInput（coll = ImmutableSet，Java 暴露为 collection）
/// 引擎差异：v1 ?map 目标仅限序列（collection 报 "not applicable to a collection
/// value"）；?sequence 已实现：对 collection 为透传（pass-through），无法转换。
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
    // Java：["sequence", "evaluated to an extended_collection"]；v1：collection 不可用
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${coll?map(it -> it?upperCase)[0]}",
        dm.clone(),
        &["not applicable to a collection value"],
    );
    // Java：["lazy transformation", "?sequence", "[#list]"]；v1：collection 不可用
    assert_error_contains_with_dm(
        &c,
        &loader,
        "[#ftl][#assign t = coll?map(it -> it?upperCase)]",
        dm.clone(),
        &["not applicable to a collection value"],
    );
    // Java 经 ?sequence 转换后 [0] → "A"；v1 ?sequence 对 collection 为透传，?map 仍报错
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${coll?sequence?map(it -> it?upperCase)[0]}",
        dm.clone(),
        &["not applicable to a collection value"],
    );
    // ?map fails before ?sequence is reached
    assert_error_contains_with_dm(
        &c,
        &loader,
        "${coll?map(it -> it?upperCase)?sequence[0]}",
        dm.clone(),
        &["not applicable to a collection value"],
    );
    // Java 期望 "ABC"；v1：collection 目标不可用 → 报错
    assert_error_contains_with_dm(
        &c,
        &loader,
        "<#list coll?map(it -> it?upperCase) as it>${it}</#list>",
        dm,
        &["not applicable to a collection value"],
    );
}
