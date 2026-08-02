//! 对应 Java: ArithmeticEngineTest
//! Java `freemarker.core.ArithmeticEngineTest` 的 Rust 1:1 实现。
//!
//! 引擎差异总览：
//! - Java 直接测内部 API `BigDecimalEngine.compareNumbers`/`toNumber`
//!   （ArithmeticEngine.java:573-660 的 compareNumbers、:518-567 的 toNumber）；
//!   v1 的 `ArithmeticEngine` trait（core/arithmetic_engine.rs）仅暴露
//!   add/sub/mul/div/mod_op/negate，无 compareNumbers/toNumber → 六个
//!   Java 方法均 NOT_APPLICABLE（Java 内部 API 级测试）。
//! - 数值比较语义（scale 无关、无舍入毛刺）可经模板 `==`/`!=` 等价验证
//!   （v1 eval_compare 按 BigDecimal 数值比较，对照 Java ArithmeticEngine.compareNumbers）
//!   → 补一个 compare_numbers_via_template 翻译。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// Java compareNumbersZeroTest —— NOT_APPLICABLE：内部 API（BigDecimalEngine.compareNumbers）。
// NOT_APPLICABLE: compareNumbersZeroTest —— Java 内部 API BigDecimalEngine.compareNumbers
// Java compareNumbersNoRoundingGlitchTest —— NOT_APPLICABLE：同上。
// NOT_APPLICABLE: compareNumbersNoRoundingGlitchTest —— Java 内部 API BigDecimalEngine.compareNumbers

// Java compareNumbersSameTypeTest —— NOT_APPLICABLE：同上。
// NOT_APPLICABLE: compareNumbersSameTypeTest —— Java 内部 API BigDecimalEngine.compareNumbers

// Java compareNumbersScaleDoesNotMatterTest —— NOT_APPLICABLE：内部 API；
// 语义经 compare_numbers_via_template 等价翻译。
// NOT_APPLICABLE: compareNumbersScaleDoesNotMatterTest —— Java 内部 API BigDecimalEngine.compareNumbers

// Java compareNumbersInfinityTest —— NOT_APPLICABLE：内部 API。
// NOT_APPLICABLE: compareNumbersInfinityTest —— Java 内部 API BigDecimalEngine.compareNumbers

// Java toNumberTest —— NOT_APPLICABLE：内部 API（BigDecimalEngine.toNumber）。
// NOT_APPLICABLE: toNumberTest —— Java 内部 API BigDecimalEngine.toNumber

/// 补充翻译：通过模板 `==`/`!=` 验证 BigDecimalEngine 数值比较语义
/// （v1 eval_compare 按 BigDecimal 数值比较，Java ArithmeticEngine.compareNumbers 语义）：
/// - scale 无关（1.0 == 1、1.0 == 1.00、1 == 1.0）；
/// - 无舍入毛刺（1.1 == 1.1、0.1+0.2 == 0.3）。
#[test]
fn compare_numbers_via_template() {
    let (c, loader) = test_config();
    assert_output(&c, &loader, "${(1.1 == 1.1)?c}", "true");
    assert_output(&c, &loader, "${(1.0 == 1)?c}", "true");
    assert_output(&c, &loader, "${(1.0 == 1.00)?c}", "true");
    assert_output(&c, &loader, "${(1 == 1.0)?c}", "true");
    assert_output(&c, &loader, "${(0 == 0.0)?c}", "true");
    assert_output(&c, &loader, "${(0.1 + 0.2 == 0.3)?c}", "true");
    // 不相等方向（Java compareNumbers 返回 -1/1）
    assert_output(&c, &loader, "${(1.1 == 1.2)?c}", "false");
    assert_output(&c, &loader, "${(1 < 2)?c}", "true");
}
