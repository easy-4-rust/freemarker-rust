//! freemarker-rust 整体功能测试模块（对应 Java 侧 freemarker-test-utils +
//! templatesuite 的角色）：
//!
//! - `tests/golden.rs`：黄金套件 runner——读取 `tests/suite/manifest.json`
//!   （128 个 Java 用例）与 `tests/suite/cases/`，逐字节对照 Java expected
//!   输出（V3_GOLDEN_DIFF 证据）；no_output 用例 = 渲染成功不报错。
//! - `tests/common/`：套件辅助（assert/assertEquals/assertFails/noOutput
//!   断言指令 + 设置应用 + 数据模型构造，对应 Java TemplateTestCase）。
//! - `tests/suite/`：从 apache/freemarker templatesuite 复制的 128 个用例
//!   （模板与 expected 文件与 Java 原版逐字节一致）。
//!
//! freemarker / freemarker-pyo3 crate 内的 `#[cfg(test)]` 测试仅做局部/单元级
//! 验证；跨模块、端到端、与 Java oracle 的对照全部收敛在本模块。

#![allow(dead_code)]
