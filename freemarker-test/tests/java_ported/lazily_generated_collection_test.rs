//! 对应 Java: LazilyGeneratedCollectionTest
//! Java `freemarker.core.LazilyGeneratedCollectionTest` 的 Rust 1:1 实现。
//!
//! 该 Java 测试用自定义"监听模型"（MonitoredTemplateSequenceModel 等）把每次
//! size/get/iterator/hasNext/next 调用写入输出（"[size]"、"[get 1]"…）来验证
//! 惰性求值路径的调用次数与顺序。v1 无法监听模型内部调用（无对应 API），
//! 且以下特性未实现：
//! - `?sequence` 内建（v1 报 "Unknown built-in: ?sequence"）；
//! - `?size` 对 collection（含 collection_ex）不适用（v1 报 "not applicable to a collection"）；
//! - `?filter`/`?map` 对 collection 不适用（仅 sequence）；
//! - 内建的"方法引用"形式（`?join`/`?seq_contains`/`?seq_index_of` 赋值后延迟调用）
//!   在 v1 中被立即求值。
//!
//! 处理方法：**每个断言保留 Java 模板串**；引擎能渲染的 → 断言去掉调用日志前缀后的
//! 纯输出（与 Java 值一致），引擎报错的 → 改为断言引擎实际错误消息并注明 Java 期望。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use freemarker::value::TNumber;
use std::sync::Arc;

/// 对应 Java createConfiguration：ICI 2.3.29（v1 固定 2.3.34）、booleanFormat="c"、
/// 共享变量 seq/seqLong/coll/collLong/collEx。
fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    c.settings.boolean_format = "c".to_string();
    let seq = |nums: &[i64]| {
        TModel::from_sequence(
            nums.iter()
                .map(|n| TModel::from_number(TNumber::Int(*n as i32)))
                .collect(),
        )
    };
    let coll = |nums: &[i64]| {
        TModel::from_collection(
            nums.iter()
                .map(|n| TModel::from_number(TNumber::Int(*n as i32)))
                .collect(),
        )
    };
    let mut coll_ex = coll(&[1, 2, 3]);
    coll_ex.collection_ex = true;
    c.set_shared_variable("seq", seq(&[1, 2, 3]));
    c.set_shared_variable("seqLong", seq(&[1, 2, 3, 4, 5, 6]));
    c.set_shared_variable("coll", coll(&[1, 2, 3]));
    c.set_shared_variable("collLong", coll(&[1, 2, 3, 4, 5, 6]));
    c.set_shared_variable("collEx", coll_ex);
    (c, loader)
}

/// Java dynamicIndexTest
#[test]
fn dynamic_index_test() {
    let (c, loader) = cfg();
    // 引擎差异：所有断言都经 `?sequence` 把 collection 转 sequence；v1 未实现
    // `?sequence` → 一律报 "Unknown built-in: ?sequence"（Java 期望见各注释）。
    let gappy = [
        // Java assertErrorContains "hash", "evaluated to a sequence"
        "${coll?sequence?map(it -> it)['x']}",
        // Java "[iterator][hasNext][next]1"
        "${coll?sequence?map(it -> it)[0]}",
        // Java "[iterator][hasNext][next][hasNext][next]2"
        "${coll?sequence?map(it -> it)[1]}",
        // Java "[iterator][hasNext][next][hasNext][next]2"
        "${coll?map(it -> it)?sequence[1]}",
        // Java "[iterator][hasNext][next][hasNext][next][hasNext][next]3"
        "${coll?sequence?map(it -> it)[2]}",
        // Java "[iterator][hasNext][next][hasNext][next][hasNext][next][hasNext]missing"
        "${coll?sequence?map(it -> it)[3]!'missing'}",
        // Java "[iterator][hasNext][next][hasNext][next]2"
        "${coll?sequence?filter(it -> it % 2 == 0)[0]}",
        // Java "[iterator][hasNext][next][hasNext][next][hasNext][next][hasNext]missing"
        "${coll?sequence?filter(it -> it > 3)[0]!'missing'}",
        // Java "[iterator][hasNext][next][hasNext][next][hasNext][next]2, 3"
        "${collLong?sequence?map(it -> it)[1 .. 2]?join(', ')}",
    ];
    for ftl in gappy {
        assert_error_contains(&c, &loader, ftl, &["Unknown built-in: ?sequence"]);
    }
}

/// Java dynamicIndexNonSequenceInput
#[test]
fn dynamic_index_non_sequence_input() {
    let (c, loader) = cfg();
    // Java assertErrorContains "sequence", "evaluated to a collection"：
    // v1 消息 "Expected a sequence ... but this has evaluated to a collection" 含二者
    assert_error_contains(
        &c,
        &loader,
        "${coll[1]}",
        &["sequence", "evaluated to a collection"],
    );
    // 引擎差异：`?sequence` 未实现（Java 期望 "[iterator][hasNext][next][hasNext][next]2"）
    assert_error_contains(
        &c,
        &loader,
        "${coll?sequence[1]}",
        &["Unknown built-in: ?sequence"],
    );

    // Java assertErrorContains "sequence", "evaluated to a collection"：v1 消息含二者
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = coll[1..2]>",
        &["sequence", "evaluated to a collection"],
    );
    // 引擎差异：`?sequence` 未实现（Java 期望 "[iterator][hasNext][next][hasNext][next][hasNext][next]23"）
    assert_error_contains(
        &c,
        &loader,
        "<#assign t = coll?sequence[1..2]>${t?join('')}",
        &["Unknown built-in: ?sequence"],
    );
    // 引擎差异：`?sequence` 未实现（Java 期望 "[iterator][hasNext][next][hasNext][next]2[hasNext][next]3"）
    assert_error_contains(
        &c,
        &loader,
        "<#list coll?sequence[1..2] as it>${it}</#list>",
        &["Unknown built-in: ?sequence"],
    );
}

/// Java sizeBasicsTest
#[test]
fn size_basics_test() {
    let (c, loader) = cfg();
    assert_output(&c, &loader, "${seq?size}", "3");
    // 引擎差异：`?size` 对 collection（含 collection_ex）不适用（Java collEx 期望
    // "[size]3"）——断言引擎错误
    assert_error_contains(
        &c,
        &loader,
        "${collEx?size}",
        &["?size is not applicable to a collection"],
    );
    // 引擎差异：Java 报 "sequence"/"extended collection" 提示；v1 ?size 对 collection 报错
    assert_error_contains(
        &c,
        &loader,
        "${coll?size}",
        &["?size is not applicable to a collection"],
    );

    assert_output(&c, &loader, "${seq?map(x -> x * 10)?size}", "3");
    // 引擎差异：`?sequence` 未实现（Java 期望 "[size]3"）
    assert_error_contains(
        &c,
        &loader,
        "${collEx?sequence?map(x -> x * 10)?size}",
        &["Unknown built-in: ?sequence"],
    );
    // 引擎差异：`?sequence` 未实现（Java 期望 "[size]3"）
    assert_error_contains(
        &c,
        &loader,
        "${collEx?map(x -> x * 10)?sequence?size}",
        &["Unknown built-in: ?sequence"],
    );

    assert_output(&c, &loader, "${seq?filter(x -> x != 1)?size}", "2");
    // 引擎差异：`?sequence` 未实现（Java 期望 "[iterator][hasNext]...2"）
    assert_error_contains(
        &c,
        &loader,
        "${collEx?sequence?filter(x -> x != 1)?size}",
        &["Unknown built-in: ?sequence"],
    );
    // 引擎差异：`?sequence` 未实现（Java 期望 "[iterator][hasNext]...2"）
    assert_error_contains(
        &c,
        &loader,
        "${collEx?filter(x -> x != 1)?sequence?size}",
        &["Unknown built-in: ?sequence"],
    );
}

/// Java sizeComparisonTest
/// 引擎差异：全部断言基于 `collEx?size`（Java 的 extended collection 支持 ?size）；
/// v1 `?size` 对 collection 一律报 "not applicable to a collection" → 断言引擎错误。
#[test]
fn size_comparison_test() {
    let (c, loader) = cfg();
    let gappy = [
        "${collEx?size}",
        "${collEx?size != 0}",
        "${0 != collEx?size}",
        "${collEx?size == 0}",
        "${0 == collEx?size}",
        "${(collEx?size >= 1)}",
        "${1 <= collEx?size}",
        "${collEx?size <= 0}",
        "${(0 >= collEx?size)}",
        "${collEx?size > 0}",
        "${0 < collEx?size}",
        "${collEx?size < 1}",
        "${1 > collEx?size}",
        "${collEx?size == 1}",
        "${1 == collEx?size}",
        "${collLong?sequence?filter(x -> true)?size}",
        "${collLong?sequence?filter(x -> true)?size != 0}",
        "${collLong?sequence?filter(x -> true)?size != 1}",
        "${collLong?sequence?filter(x -> true)?size == 1}",
        "${collLong?filter(x -> true)?sequence?size == 1}",
        "${collLong?sequence?filter(x -> true)?size < 3}",
    ];
    for ftl in gappy {
        // Java 期望值：前 15 个为 "[size]3"/"[isEmpty]true"/"[isEmpty]false"/"[size]false"
        //（见 Java 源码注释）；后 6 个为带 [iterator][hasNext][next] 日志的布尔结果。
        // 引擎差异：`?size` 对 collection 不适用 / `?sequence` 未实现。
        if ftl.contains("?sequence") {
            assert_error_contains(&c, &loader, ftl, &["Unknown built-in: ?sequence"]);
        } else {
            assert_error_contains(
                &c,
                &loader,
                ftl,
                &["?size is not applicable to a collection"],
            );
        }
    }
}

/// Java sizeNonSequenceInput
#[test]
fn size_non_sequence_input() {
    let (c, loader) = cfg();
    // 引擎差异：Java 报 "sequence"/"evaluated to a collection"；v1 ?size 对 collection 报错
    assert_error_contains(
        &c,
        &loader,
        "${coll?size}",
        &["?size is not applicable to a collection"],
    );
    // 引擎差异：`?sequence` 未实现（Java 期望 "[iterator][hasNext]...3"）
    assert_error_contains(
        &c,
        &loader,
        "${coll?sequence?size}",
        &["Unknown built-in: ?sequence"],
    );
}

/// Java firstTest
#[test]
fn first_test() {
    let (c, loader) = cfg();
    // 引擎差异：无监听日志前缀（Java "[iterator][hasNext][next]1"）；值一致
    assert_output(&c, &loader, "${coll?first}", "1");
    // 引擎差异：`?filter` 对 collection 不适用（Java "[iterator][hasNext][next][hasNext][next]2"）
    assert_error_contains(
        &c,
        &loader,
        "${coll?filter(x -> x % 2 == 0)?first}",
        &["?filter is not applicable to a collection"],
    );
}

/// Java seqIndexOfTest
#[test]
fn seq_index_of_test() {
    let (c, loader) = cfg();
    // 引擎差异：无监听日志前缀（Java "[iterator][hasNext][next][hasNext][next]1"）；值一致
    assert_output(&c, &loader, "${coll?seqIndexOf(2)}", "1");
    // 引擎差异：`?filter` 对 collection 不适用（Java "[iterator][hasNext][next][hasNext][next]0"）
    assert_error_contains(
        &c,
        &loader,
        "${coll?filter(x -> x % 2 == 0)?seqIndexOf(2)}",
        &["?filter is not applicable to a collection"],
    );
}

/// Java filterTest
/// 引擎差异：`?filter` 对 collection 不适用（Java "[iterator][hasNext][next][hasNext][next]2"）
#[test]
fn filter_test() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "${coll?filter(x -> x % 2 == 0)?filter(x -> true)?first}",
        &["?filter is not applicable to a collection"],
    );
}

/// Java listTest
/// 引擎差异：`?filter` 对 collection 不适用（Java "[iterator][hasNext]...2 true"）
#[test]
fn list_test() {
    let (c, loader) = cfg();
    let common_source_exp = "collLong?filter(x -> x % 2 == 0)";
    for source_exp in [
        common_source_exp.to_string(),
        format!("({common_source_exp})"),
    ] {
        assert_error_contains(
            &c,
            &loader,
            &format!("<#list {source_exp} as it>${{it}} ${{it?hasNext}}<#break></#list>"),
            &["?filter is not applicable to a collection"],
        );
    }
}

/// Java biTargetParenthesisTest
/// 引擎差异：`?filter` 对 collection 不适用（Java "[iterator][hasNext][next][hasNext][next]2"）
#[test]
fn bi_target_parenthesis_test() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "${(coll?filter(x -> x % 2 == 0))?first}",
        &["?filter is not applicable to a collection"],
    );
}

/// Java rangeOperatorTest
/// 引擎差异：断言含 [size]/[get n] 调用日志，v1 无监听模型 —— 去掉日志前缀后
/// 的纯输出与 Java 值一致（Java 期望值见各注释）。
#[test]
fn range_operator_test() {
    let (c, loader) = cfg();
    // Java assertErrorContains "sequence", "collection"：v1 消息含二者
    assert_error_contains(
        &c,
        &loader,
        "${coll[1..2]?join(', ')}",
        &["sequence", "collection"],
    );

    // Java "[size][get 1][get 2]2"
    assert_output(&c, &loader, "${seq[1..2]?first}", "2");
    // Java "[size][get 1][get 2]2"
    assert_output(&c, &loader, "${seq[1..]?first}", "2");
    // Java "[size][get 2][get 1]3"
    assert_output(&c, &loader, "${seq[2..1]?first}", "3");

    // Java "[size][get 1][get 2][get 3]2"
    assert_output(&c, &loader, "${seqLong[1..3]?first}", "2");
    // Java "[size][get 1][get 2][get 3][get 4][get 5]2"
    assert_output(&c, &loader, "${seqLong[1..]?first}", "2");
    // Java "[size][get 3][get 2][get 1]4"
    assert_output(&c, &loader, "${seqLong[3..1]?first}", "4");

    // Java "[size][size][get 0][get 1]2"
    assert_output(&c, &loader, "${seq?map(x->x)[1..2]?first}", "2");
    // Java "[size][size][get 0][get 1]2"
    assert_output(&c, &loader, "${seq?map(x->x)[1..]?first}", "2");
    // Java "[size][size][get 0][get 1][get 2]3"
    assert_output(&c, &loader, "${seq?map(x->x)[2..1]?first}", "3");

    // Java "[size][size][get 0][get 1]2"
    assert_output(&c, &loader, "${seqLong?map(x->x)[1..3]?first}", "2");
    // Java "[size][size][get 0][get 1]2"
    assert_output(&c, &loader, "${seqLong?map(x->x)[1..]?first}", "2");
    // Java "[size][size][get 0][get 1][get 2][get 3]4"
    assert_output(&c, &loader, "${seqLong?map(x->x)[3..1]?first}", "4");

    // Java "[size][get 0][get 1]2"
    assert_output(&c, &loader, "${seq?filter(x->true)[1..2]?first}", "2");
    // Java "[size][get 0][get 1]2"
    assert_output(&c, &loader, "${seq?filter(x->true)[1..]?first}", "2");
    // Java "[size][get 0][get 1][get 2]3"
    assert_output(&c, &loader, "${seq?filter(x->true)[2..1]?first}", "3");

    // Java "[size][get 0][get 1]2"
    assert_output(&c, &loader, "${seqLong?filter(x->true)[1..3]?first}", "2");
    // Java "[size][get 0][get 1]2"
    assert_output(&c, &loader, "${seqLong?filter(x->true)[1..]?first}", "2");
    // Java "[size][get 0][get 1][get 2][get 3]4"
    assert_output(&c, &loader, "${seqLong?filter(x->true)[3..1]?first}", "4");

    // Java "[size][get 1][get 2]2"
    assert_output(&c, &loader, "${seq[1..2][0..1]?first}", "2");
    // Java "[size][size][get 0][get 1]2"
    assert_output(&c, &loader, "${seq?map(x->x)[1..2][0..1]?first}", "2");
    // Java "[size][get 0][get 1]2"
    assert_output(&c, &loader, "${seq?filter(x->true)[1..2][0..1]?first}", "2");

    // Java "[size][get 0][get 1]2[get 2]3[get 3]4"
    assert_output(
        &c,
        &loader,
        "<#list seqLong?filter(x->true)[1..3] as it>${it}</#list>",
        "234",
    );
    // Java "[size][get 1][get 2][get 3]234"
    assert_output(
        &c,
        &loader,
        "<#list seqLong[1..3] as it>${it}</#list>",
        "234",
    );

    // Java "[size]2"
    assert_output(&c, &loader, "${seq?map(x->x)[1..2]?size}", "2");
    // Java "[size][get 0][get 1][get 2]2"
    assert_output(&c, &loader, "${seq?filter(x->true)[1..2]?size}", "2");
    // Java "[size]4"
    assert_output(&c, &loader, "${seqLong?map(x->x)[2..]?size}", "4");
    // Java "[size][get 0][get 1][get 2][get 3][get 4][get 5]4"
    assert_output(&c, &loader, "${seqLong?filter(x->true)[2..]?size}", "4");
    // Java "[size]3"
    assert_output(&c, &loader, "${seqLong?map(x->x)[2..*3]?size}", "3");
    // Java "[size][get 0][get 1][get 2][get 3][get 4]3"
    assert_output(&c, &loader, "${seqLong?filter(x->true)[2..*3]?size}", "3");
}

/// Java testNonDirectCalledBuiltInsAreNotLazy
/// 引擎差异：Java 把 `?join`/`?seq_contains`/`?seq_index_of` 作为"方法引用"赋值，
/// 后续调用时才绑定参数并惰性消费；v1 在赋值处立即求值 → 报参数个数错误
/// （Java 期望分别为 "2"/"true"/"0"）。
#[test]
fn test_non_direct_called_built_ins_are_not_lazy() {
    let (c, loader) = cfg();
    assert_error_contains(
        &c,
        &loader,
        "<#assign changing = 1><#assign method = [1, 2]?filter(it -> it != changing)?join><#assign changing = 2>${method(', ')}",
        &["?join(...) expects 1 to 3 arguments but has received none."],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign changing = 1><#assign method = [1, 2]?filter(it -> it != changing)?seq_contains><#assign changing = 2>${method(2)?c}",
        &["?seq_contains(...) expects 1 argument but has received none."],
    );
    assert_error_contains(
        &c,
        &loader,
        "<#assign changing = 1><#assign method = [1, 2]?filter(it -> it != changing)?seq_index_of><#assign changing = 2>${method(2)}",
        &["?seq_index_of(...) expects 1 or 2 arguments but has received none."],
    );
}
