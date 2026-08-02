//! 对应 Java: ConcatenatedSequenceTest
//! Java `freemarker.core.ConcatenatedSequenceTest` 的 Rust 1:1 实现。
//!
//! 该 Java 类是 `AddConcatExpression.ConcatenatedSequence` 类的纯单元测试
//! （不渲染模板，直接调 iterator()/size()/get()/isEmpty()）。v1 的序列拼接
//! （`seq + seq`，eval.rs eval_add）即 Java ConcatenatedSequence 的对应实现，
//! 故以 FTL 等价验证：
//! - 迭代顺序：`<#list c as x>${x!'null'}</#list>`（v1 序列可重复迭代，两次一致）
//! - size()：`${c?size}`
//! - get(i)：`${c[i]!'null'}`；get(-1)/get(size)/get(size+1) Java 返回 null →
//!   v1 越界报错被 `!` 抑制，输出 "null"（引擎差异：v1 无 null 模型返回）
//! - isEmpty()：`${c?size == 0}`
//!
//! 引擎差异：
//! - Java 5 种 SeqFactory（SimpleSequence/ListAdapter/SequenceAndCollectionEx/
//!   iterable 包装/iterator 包装）→ v1 统一用 from_sequence（可重复）；
//!   iterator 包装的"不可重复"语义无法模拟。
//! - 含 null 元素的序列：v1 不支持 FTL `null` 字面量（`null` 按标识符解析报错），
//!   无法用 FTL 构造 null 元素 → 该用例在 test_with_segment_factory 中以注释保留。
//! - 越界/负下标取元素：v1 报 InvalidReference，`!` 默认值仅对括号/标识符目标
//!   抑制缺失错误 → 越界断言用 `(c[idx])!...` 括号写法对齐。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::Configuration;
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    // 引擎差异：Java 直接迭代模型（无循环变量回退）；v1 用 #list 模拟需关闭回退
    c.settings.fallback_on_null_loop_variable = false;
    (c, loader)
}

/// 断言拼接结果 —— 对应 Java assertConcatenationResult（repeatable 语义：v1 序列
/// 永远可重复，迭代两次；size/get/isEmpty 同 Java）
/// `prelude` 为可选 `<#assign s = ...>` 等前置定义；`expr` 为拼接表达式。
fn assert_concatenation_result(
    c: &Configuration,
    loader: &Arc<StringLoader>,
    prelude: &str,
    expr: &str,
    expected_items: &[Option<&str>],
) {
    let with_expr = |body: &str| format!("{prelude}<#assign c = {expr}>{body}");

    // 迭代（Java 第一次迭代；v1 两次一致）
    let expected_join = expected_items
        .iter()
        .map(|i| i.unwrap_or("null"))
        .collect::<Vec<_>>()
        .join(", ");
    let out = render_ftl(
        c,
        loader,
        &with_expr("<#list c as x>${x!'null'}<#sep>, </#list>"),
    );
    assert_eq!(out, expected_join, "expr: {expr}");
    // Java 第二次迭代（repeatable 时重新构造；v1 序列可重复 → 直接再迭代）
    let out = render_ftl(
        c,
        loader,
        &with_expr("<#list c as x>${x!'null'}<#sep>, </#list>"),
    );
    assert_eq!(out, expected_join, "expr: {expr} (2nd)");

    // size()
    let out = render_ftl(c, loader, &with_expr("${c?size}"));
    assert_eq!(out, expected_items.len().to_string(), "expr: {expr} ?size");

    // get(i)（Java seq.get(i)）
    for (i, item) in expected_items.iter().enumerate() {
        let out = render_ftl(c, loader, &with_expr(&format!("${{c[{i}]!'null'}}")));
        assert_eq!(out, item.unwrap_or("null"), "expr: {expr} get({i})");
    }

    // get(-1)/get(size)/get(size+1) → Java 返回 null
    // 引擎差异：v1 对负下标产生 InvalidReference 错误（正下标越界返回 null 模型），
    // 需用括号包裹 `(c[idx])!...` 才能被 `!` 默认值抑制（eval_lenient 仅对
    // 括号/标识符目标抑制缺失错误）→ 同为 "null"
    for idx in [
        -1i64,
        expected_items.len() as i64,
        expected_items.len() as i64 + 1,
    ] {
        let out = render_ftl(c, loader, &with_expr(&format!("${{(c[{idx}])!'null'}}")));
        assert_eq!(out, "null", "expr: {expr} get({idx})");
    }

    // isEmpty()（Java seq.isEmpty()）
    // 引擎差异：默认 boolean_format 下 ${布尔} 输出报错 → 用 ?c 转 "true"/"false"
    let out = render_ftl(c, loader, &with_expr("${(c?size == 0)?c}"));
    assert_eq!(
        out,
        if expected_items.is_empty() {
            "true"
        } else {
            "false"
        },
        "expr: {expr} isEmpty"
    );
}

/// Java testForSimpleSequences（SeqFactory = SimpleSequence）
#[test]
fn test_for_simple_sequences() {
    test_with_segment_factory();
}

/// Java testForListAdapter（SeqFactory = DefaultListAdapter）
#[test]
fn test_for_list_adapter() {
    // 引擎差异：Java 用 DefaultListAdapter.adapt(List)；v1 无 List 适配层，
    // 与 SimpleSequence 同等看待（结果行为一致）
    test_with_segment_factory();
}

/// Java testForSequenceAndCollectionModelEx（SeqFactory = 序列+集合双角色模型）
#[test]
fn test_for_sequence_and_collection_model_ex() {
    // 引擎差异：Java 自定义 SequenceAndCollectionModelEx；v1 无双角色序列需求
    // （拼接结果与纯序列一致）→ 同 SimpleSequence 路径
    test_with_segment_factory();
}

/// Java testForCollectionsWrappingIterable（SeqFactory = SimpleCollection(iterable)）
#[test]
fn test_for_collections_wrapping_iterable() {
    // 引擎差异：Java 用 CollectionAndSequence 包装 SimpleCollection；v1 统一序列路径
    test_with_segment_factory();
}

/// Java testForCollectionsWrappingIterator（SeqFactory = SimpleCollection(iterator)，
/// isUnrepeatable=true）
#[test]
fn test_for_collections_wrapping_iterator() {
    // 引擎差异：Java 该工厂 isUnrepeatable=true（迭代一次后序列耗尽）；
    // v1 序列永远可重复 → 按可重复语义执行（与其余工厂相同）
    test_with_segment_factory();
}

/// Java testWithSegmentFactory —— 全部拼接组合
fn test_with_segment_factory() {
    let (c, loader) = cfg();

    // 对应 Java assertConcatenationResult 的各构造树（FTL 序列字面量等价；
    // null 元素用 FTL null 字面量）：
    let cases: Vec<(&str, &str, Vec<Option<&str>>)> = vec![
        ("", "[] + []", vec![]),
        ("", "[] + ['b']", vec![Some("b")]),
        ("", "['a'] + []", vec![Some("a")]),
        ("", "['a'] + ['b']", vec![Some("a"), Some("b")]),
        ("", "([] + []) + ([] + [])", vec![]),
        (
            "",
            "(['a', 'b'] + []) + ([] + [])",
            vec![Some("a"), Some("b")],
        ),
        (
            "",
            "([] + ['a', 'b']) + ([] + [])",
            vec![Some("a"), Some("b")],
        ),
        (
            "",
            "([] + []) + (['a', 'b'] + [])",
            vec![Some("a"), Some("b")],
        ),
        (
            "",
            "([] + []) + ([] + ['a', 'b'])",
            vec![Some("a"), Some("b")],
        ),
        (
            "",
            "(['a'] + ['b']) + ([] + [])",
            vec![Some("a"), Some("b")],
        ),
        (
            "",
            "([] + ['a']) + (['b'] + [])",
            vec![Some("a"), Some("b")],
        ),
        (
            "",
            "([] + []) + (['a'] + ['b'])",
            vec![Some("a"), Some("b")],
        ),
        (
            "",
            "((['a'] + ['b']) + ['c']) + ['d']",
            vec![Some("a"), Some("b"), Some("c"), Some("d")],
        ),
        (
            "",
            "(['a'] + ['b']) + (['c'] + ['d'])",
            vec![Some("a"), Some("b"), Some("c"), Some("d")],
        ),
        (
            "",
            "['a'] + (['b'] + (['c'] + ['d']))",
            vec![Some("a"), Some("b"), Some("c"), Some("d")],
        ),
        (
            "",
            "(['a', 'b'] + ['c', 'd']) + (['e', 'f'] + ['g', 'h'])",
            vec![
                Some("a"),
                Some("b"),
                Some("c"),
                Some("d"),
                Some("e"),
                Some("f"),
                Some("g"),
                Some("h"),
            ],
        ),
        (
            "",
            "['a', 'b', 'c'] + (['d', 'e'] + ['f', 'g', 'h'])",
            vec![
                Some("a"),
                Some("b"),
                Some("c"),
                Some("d"),
                Some("e"),
                Some("f"),
                Some("g"),
                Some("h"),
            ],
        ),
        (
            "",
            "(['a', 'b'] + ['c', 'd']) + ['e', 'f', 'g', 'h']",
            vec![
                Some("a"),
                Some("b"),
                Some("c"),
                Some("d"),
                Some("e"),
                Some("f"),
                Some("g"),
                Some("h"),
            ],
        ),
        // 同一段序列实例多次出现（Java abab = ab + ab；abab + abab）：
        (
            "<#assign s = ['a', 'b']>",
            "(s + s) + (s + s)",
            vec![
                Some("a"),
                Some("b"),
                Some("a"),
                Some("b"),
                Some("a"),
                Some("b"),
                Some("a"),
                Some("b"),
            ],
        ),
        // Java 还有 null 元素用例：
        // `new ConcatenatedSequence(new ConcatenatedSequence(create(null, "a"), create("b", null)), create(null))`
        // 期望 [null, "a", "b", null, null]。
        // 引擎差异：v1 不支持 FTL `null` 字面量（`null` 按标识符解析 → 未知变量
        // InvalidReference 报错），无法用 FTL 构造含 null 元素的序列 → 该用例不执行
        //（Java 语义见上方注释；引擎无 null 元素模型）。
    ];

    for (prelude, expr, expected) in cases {
        assert_concatenation_result(&c, &loader, prelude, expr, &expected);
    }
}
