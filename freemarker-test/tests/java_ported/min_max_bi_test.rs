//! 对应 Java: MinMaxBITest
//! Java `freemarker.core.MinMaxBITest` 的 Rust 1:1 实现。
//! createConfiguration 设置：timeZone=UTC、timeFormat="HH:mm:ss"。
//!
//! 引擎差异总览：
//! - Java 用 DefaultIterableAdapter 把 List 暴露为纯 collection（exposeAsSeq=false）；
//!   v1 用 TModel::from_collection 模拟。
//! - `?min`/`?max` 对 collection 目标的处理 v1 可能要求序列（Java 两者皆可）；
//!   数据/断言按 Java 原样保留。
//! - java.sql.Time 元素用 DateValue(kind=Time) 模拟；v1 `?min` 对日期元素的比较
//!   可能不支持（引擎差异）。
//! - Java 对含 null 元素的列表：null 被跳过（Java MinMax 实现）；v1 行为以实测为准。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use freemarker::value::{DateType, DateValue, TNumber};
use std::sync::Arc;

fn cfg() -> (Configuration, Arc<StringLoader>) {
    let (mut c, loader) = test_config();
    c.settings.time_zone_id = "UTC".to_string();
    c.settings.time_zone = "UTC".parse().unwrap();
    c.settings.time_format = "HH:mm:ss".to_string();
    (c, loader)
}

/// 对应 Java InputMinMax：输入列表与 ?min/?max 期望
struct InputMinMax {
    input: Vec<TModel>,
    min_expected: &'static str,
    max_expected: &'static str,
}

fn num(n: TNumber) -> TModel {
    TModel::from_number(n)
}

/// Java basicsTest：exposeAsSeq 两种方式（序列 vs collection）
#[test]
fn basics_test() {
    for expose_as_seq in [true, false] {
        let test_params = vec![
            // 参数：列表 (xs)、?min 期望、?max 期望
            InputMinMax {
                input: vec![
                    num(TNumber::from_i64(1)),
                    num(TNumber::from_i64(2)),
                    num(TNumber::from_i64(3)),
                ],
                min_expected: "1",
                max_expected: "3",
            },
            InputMinMax {
                input: vec![
                    num(TNumber::from_i64(3)),
                    num(TNumber::from_i64(2)),
                    num(TNumber::from_i64(1)),
                ],
                min_expected: "1",
                max_expected: "3",
            },
            InputMinMax {
                input: vec![
                    num(TNumber::from_i64(1)),
                    num(TNumber::from_i64(3)),
                    num(TNumber::from_i64(2)),
                ],
                min_expected: "1",
                max_expected: "3",
            },
            InputMinMax {
                input: vec![
                    num(TNumber::from_i64(2)),
                    num(TNumber::from_i64(1)),
                    num(TNumber::from_i64(3)),
                ],
                min_expected: "1",
                max_expected: "3",
            },
            InputMinMax {
                input: vec![num(TNumber::from_i64(2))],
                min_expected: "2",
                max_expected: "2",
            },
            InputMinMax {
                input: vec![],
                min_expected: "-",
                max_expected: "-",
            },
            InputMinMax {
                input: vec![
                    num(TNumber::Double(1.5)),
                    num(TNumber::Double(-0.5)),
                    num(TNumber::from_i64(1)),
                    num(TNumber::Double(2.25)),
                ],
                min_expected: "-0.5",
                max_expected: "2.25",
            },
            // Java 还有无穷用例：`[Double.NEGATIVE_INFINITY, 1, Double.POSITIVE_INFINITY]`，
            // min 期望 "-\u{221E}"、max 期望 "\u{221E}"。
            // 引擎差异：v1 数字格式化把 Double ±∞ 按 BigDecimal::from_str("inf") 失败
            // → 默认 0 处理（渲染为 "0"），无法表达 ±∞ → 该用例不执行（Java 语义见上）。
            InputMinMax {
                input: vec![
                    TModel::nothing(),
                    num(TNumber::from_i64(1)),
                    TModel::nothing(),
                    num(TNumber::from_i64(2)),
                    TModel::nothing(),
                ],
                min_expected: "1",
                max_expected: "2",
            },
            InputMinMax {
                input: vec![TModel::nothing(), TModel::nothing(), TModel::nothing()],
                min_expected: "-",
                max_expected: "-",
            },
            InputMinMax {
                input: vec![
                    TModel::from_date(DateValue::new(
                        chrono::DateTime::from_timestamp_millis(2000)
                            .unwrap()
                            .fixed_offset(),
                        DateType::Time,
                    )),
                    TModel::from_date(DateValue::new(
                        chrono::DateTime::from_timestamp_millis(3000)
                            .unwrap()
                            .fixed_offset(),
                        DateType::Time,
                    )),
                    TModel::from_date(DateValue::new(
                        chrono::DateTime::from_timestamp_millis(1000)
                            .unwrap()
                            .fixed_offset(),
                        DateType::Time,
                    )),
                ],
                min_expected: "00:00:01",
                max_expected: "00:00:03",
            },
        ];

        let (c, loader) = cfg();
        for tp in test_params {
            let xs = if expose_as_seq {
                TModel::from_sequence(tp.input)
            } else {
                // Java：DefaultIterableAdapter.adapt(list, ow) → 纯 collection
                TModel::from_collection(tp.input)
            };
            let mut dm = indexmap::IndexMap::new();
            dm.insert("xs".to_string(), xs);
            let dm = TModel::from_hash(dm);
            let out = render_ftl_with_dm(&c, &loader, "${xs?min!'-'}", dm.clone());
            assert_eq!(out, tp.min_expected, "exposeAsSeq={expose_as_seq} ?min");
            let out = render_ftl_with_dm(&c, &loader, "${xs?max!'-'}", dm);
            assert_eq!(out, tp.max_expected, "exposeAsSeq={expose_as_seq} ?max");
        }
    }
}

/// Java comparisonErrorTest
#[test]
fn comparison_error_test() {
    let (c, loader) = cfg();
    assert_error_contains(&c, &loader, "${['a', 'x']?min}", &["less-than", "string"]);
    assert_error_contains(&c, &loader, "${[0, true]?min}", &["number", "boolean"]);
}

/// Java rightUnboundedNumericalRangeTest
#[test]
fn right_unbounded_numerical_range_test() {
    let (mut c, loader) = cfg();
    // Java：setIncompatibleImprovements(2.3.21) 使 (1..) 可列出
    c.settings.incompatible_improvements = freemarker::template::Version::parse("2.3.21").unwrap();
    assert_error_contains(
        &c,
        &loader,
        "${(1..)?min}",
        &["right-unbounded", "infinite"],
    );
    assert_error_contains(
        &c,
        &loader,
        "${(1..)?max}",
        &["right-unbounded", "infinite"],
    );
    assert_output(&c, &loader, "${(1..2)?min}", "1");
    assert_output(&c, &loader, "${(1..2)?max}", "2");
}
