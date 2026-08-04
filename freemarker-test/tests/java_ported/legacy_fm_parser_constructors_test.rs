//! Java `freemarker.core.LegacyFMParserConstructorsTest` 的 Rust 1:1 实现
//! （对应 Java: LegacyFMParserConstructorsTest —— 遗留 FMParser 构造器 API：
//!   `new FMParser(source)` 模板解析入口与 `FMParser.createExpressionParser(source)`
//!   表达式解析入口的存在性/可用性）。
//!
//! NOT_APPLICABLE: 全部 2 个方法 —— Rust 引擎无 `FMParser` 类（JavaCC 生成的
//!   解析器，freemarker.core.FMParser），无对应构造器/静态工厂；Java 原文保留
//!   为注释。说明：两个方法验证的"解析入口可解析给定源码"行为由引擎的公开
//!   `freemarker::parser::parse`（模板）/`freemarker::parser::parse_expression`
//!   （表达式）承接，且已被本套件大量既有渲染测试覆盖（如 misc_error_messages、
//!   arithmetic_engine_test 等），此处仅登记构造器 API 本身无对应。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;

// ---------------------------------------------------------------------------
// Java 原文（LegacyFMParserConstructorsTest.java，@Test 方法体）：
// ---------------------------------------------------------------------------

// Java test1（Java 原文）：
//   FMParser parser = new FMParser("x");
//   parser.Root();
//   // 等价行为（引擎 API）：freemarker::parser::parse(&cfg, "adhoc", "x") 不抛错

// Java testCreateExpressionParser（Java 原文）：
//   FMParser parser = FMParser.createExpressionParser("x + y");
//   parser.Expression();
//   // 等价行为（引擎 API）：freemarker::parser::parse_expression(&cfg, "x + y") 不抛错
