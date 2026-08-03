# 合规审计摘要

> 完整报告：`docs/合规审计报告.md`
> 审计日期：2026-08-04

## 总体结论

**合规度 ~88%**。核心引擎无桩函数（`unimplemented!()`/`todo!()` 全无）、无 Java 依赖残留（Jackson/Spring/Reactor 全部替换）、命名 100% 合规（snake_case 文件 / PascalCase 类型 / snake_case 方法）。短板集中在 §1 一文件一对象的结构性违规。

## 违规分级

| 级别 | 数量 | 主要内容 |
|---|---|---|
| **blocker** | **5** | `xml/mod.rs` 含 3 类型；`pyo3/src/lib.rs` 含 2 pyclass；`template/template_model.rs` 16 个 trait 合一；`cache/template_loader.rs` 双 trait 合一；`lib.rs` 集中重导出（边缘） |
| **major** | **12** | `core/eval.rs`、`core/environment.rs`、`core/exec.rs` 等多类型；缺 `/// 对应 Java:` 注释 |
| **minor** | **13** | 伴生类型（`TzSetting`+`Settings` 等）—— 多为合理设计，仅需注释标记 |
| **info** | **3** | pyo3 prelude 通配、测试通配导入、serde 替换 Jackson（合规） |

## Top 5 必修项

1. **`xml/mod.rs`** — 拆分 `NsPrefixes`/`XmlTree`/`XmlNode` 到子文件（30 分钟）
2. **`template/template_model.rs`** — 拆 14 trait + 1 struct 为 `template_model/` 子目录（2 小时）
3. **`cache/template_loader.rs`** — 拆 `TemplateSource`/`TemplateLoader` 为两个文件（15 分钟）
4. **`pyo3/src/lib.rs`** — 把 `FmTemplate` 迁出；`FmConfiguration` 与 `#[pymodule]` 同留（pyo3 约束）（15 分钟）
5. **`cache/template_loader.rs::read_encoded`** 等辅助方法补 `/// 对应 Java:` 行号

## 灰区（已备案，无需修改）

- `t_model.rs` 11 角色槽位合并到单一 `TModel`（docs/02 §4.1 设计决策）
- `expression.rs`、`template_element.rs` 的 AST 节点合并（Rust enum 特性）
- `template_lookup_strategy.rs`、`template_name_format.rs`、`utility_transforms.rs` 的 trait + default 实现伴生
- 测试代码的 `use crate::util::*` 通配（测试豁免）

## 合规亮点

- 所有目录对齐 Java 包（`freemarker.core` → `core/`，`freemarker.template` → `template/` 等）
- 9 个 `simple_*.rs` 严格一文件一类型
- 所有 `cache/*_template_loader.rs` 严格执行一文件一对象
- 文档注释：所有公开类型均含 `/// 对应 Java: <class-path>` + 行号引用
- 依赖：完全用 `serde_json`/`chrono`/`thiserror`/`indexmap`/`roxmltree` 替代 Java 生态
- `mod.rs` 全部干净（仅 `mod` + `pub use`），除 `xml/mod.rs` 与 `pyo3/src/lib.rs`

## 修复工时预估

| 优先级 | 项数 | 预估工时 |
|---|---|---|
| P0 blocker | 5 | ~3 小时 |
| P1 major | 7 | ~4 小时 |
| P2 minor | 13 | ~2 小时 |
| P3 cosmetic | 2 | ~1 小时 |
| **合计** | **27** | **~10 小时** |