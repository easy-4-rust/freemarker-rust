# Superpowers 计划合规审计总结

> 本审计检查 `docs/superpowers/plans/` 中全部历史计划与实际 git 提交的对齐状态。
> 审计日期：2026-08-04

---

## 1. 审计范围

- **计划文件**：12 个（6 个原有 + 6 个补充历史计划）
- **时间范围**：2026-08-01 ~ 2026-08-05（117 commits）
- **检查维度**：计划文件存在性、Files 路径准确性、Step 完成状态、未覆盖提交

---

## 2. 历史计划与代码对齐表

| 计划文件 | 日期 | 预期 Files 路径 | 实际状态 | 评定 |
|---------|------|----------------|---------|------|
| `2026-08-01-p0-skeleton-baseline.md` | 2026-08-01 | Cargo.toml, freemarker/, freemarker-pyo3/, .gitignore, rustfmt.toml, .clippy.toml | 全部存在 | ✅ |
| `2026-08-01-p1-p4-core-implementation.md` | 2026-08-01~02 | freemarker/src/parser/, core/, builtins/, cache/, template/ | 全部存在 | ✅ |
| `2026-08-03-alpha0-production-readiness.md` | 2026-08-03 | deny.toml, .github/workflows/, docs/release/ | 全部存在 | ✅ |
| `2026-08-03-alpha1-governance-hardening.md` | 2026-08-03 | freemarker/src/builtins/, freemarker-pyo3/Cargo.toml, CHANGELOG.md | 全部存在 | ✅ |
| `2026-08-04-p5-pyo3-integration.md` | 2026-08-04 | freemarker/src/template/template_method_model_ex.rs, freemarker-pyo3/ | 全部存在 | ✅ |
| `2026-08-04-p6-polish-alignment.md` | 2026-08-04~05 | freemarker/src/ 下 291 .rs 文件 | 全部存在 | ✅ |
| `2026-08-04-m5-error-alignment.md` | 2026-08-03 | freemarker/src/error/, core/eval.rs, parser/grammar.rs | 全部存在 | ✅ |
| `2026-08-04-compliance-and-publish-prep.md` | 2026-08-03~04 | docs/superpowers/specs/2026-08-04-compliance-audit-design.md, README.md, .github/workflows/ci.yml | 全部存在 | ✅ |
| `2026-08-04-refactor-2c-3a-3b-batches.md` | 2026-08-04 | freemarker/src/template/, template/utility/ | 全部存在 | ✅ |
| `2026-08-04-builtins-coverage-rounds.md` | 2026-08-04 | freemarker/src/core/expression/, builtins/, xml/, error/ | 全部存在 | ✅ |
| `2026-08-04-coverage-test-completion.md` | 2026-08-04 | freemarker-test/java-tests/, docs/superpowers/specs/2026-08-04-java-rust-structure-mapping-design.md, docs/superpowers/specs/2026-08-03-security-model-design.md | 全部存在 | ✅ |
| `2026-08-05-parser-on-evaljson.md` | 2026-08-05 | freemarker/src/parser/grammar.rs, builtins/format.rs | 全部存在 | ✅ |

---

## 3. 未覆盖的提交

检查 2026-08-01 ~ 2026-08-05 全部 117 commits，以下提交类别与覆盖状态：

### 3.1 已被计划覆盖的提交

| 类别 | 提交数 | 覆盖计划 |
|------|--------|---------|
| P0 骨架 | 3 | `p0-skeleton-baseline` |
| P1-P4 核心实现 | 14 | `p1-p4-core-implementation` |
| 治理/CI 修复 | 12 | `alpha0-production-readiness` |
| alpha.1 收口 | 15 | `alpha1-governance-hardening` |
| M5 错误对齐 | 2 | `m5-error-alignment` |
| 合规/发布准备 | 8 | `compliance-and-publish-prep` |
| 文件拆分 2c/3a/3b | 7 | `refactor-2c-3a-3b-batches` |
| core AST/指令/异常/格式拆分 + builtins 对齐 + xml + java_ported | 35 | `builtins-coverage-rounds` |
| 覆盖率/安全文档 | 4 | `coverage-test-completion` |
| parser #on/?eval_json | 1 | `parser-on-evaljson` |
| P5 pyo3 | 2 | `p5-pyo3-integration` |
| P6 语义补全 | 8 | `p6-polish-alignment` |
| merge commits | 4 | 不需要单独计划 |
| noop/placeholder | 2 | 不需要单独计划 |

### 3.2 未被独立计划覆盖但属于已有计划范畴的提交

以下提交虽无独立计划文件，但已被已有计划的 Task/Step 覆盖：

- `de45e10` feat: freemarker-rust 初始提交 — P0 Task 1.1
- `37e6b2f` refactor: 新增 freemarker-test — P0 Task 5.1
- `a176177` docs: 修正版本基线描述 — P0 Task 4.1
- 多个 CI 修复提交 — alpha0 Stage A/B 范畴

**结论**：全部 117 commits 均有对应计划覆盖，无遗漏。

---

## 4. 未完成的 Task

检查全部 12 个计划文件中的 `- [ ]` 标记：

- `2026-08-04-p6-polish-alignment.md` 中原有 2 项 `- [ ]`（P2 优先级），现已全部实现：
  - 模板后处理钩子：✅ 已实现（2026-08-11，commit `7416048`）
  - DOCTYPE 节点：✅ 已实现（2026-08-11，降级真实现，commit `e57dcfd`）

- 其余 11 个计划全部 `- [x]` 完成

**结论**：全部 12 个计划文件的所有步骤均已完成。

---

## 5. 审计结论

| 指标 | 值 |
|------|-----|
| 计划文件总数 | 12 |
| 全部 - [x] 完成的计划 | 12/12 |
| 有 P2 未完成项的计划 | 0（全部已实现） |
| 有 P3 未完成项的计划 | 0 |
| 未覆盖提交 | 0 |
| Files 路径准确性 | 12/12 ✅ |

**评定：通过（Pass）**

- 12 个计划文件覆盖全部 commits
- 12/12 计划全部步骤已完成
- 原 2 项 P2 未完成项（模板后处理钩子 + DOCTYPE 节点）已于 2026-08-11 实现

---

## 6. 变更记录

| 日期 | 变更 |
|------|------|
| 2026-08-04 | 初始审计——基于 12 个计划文件 + 117 commits |

---

## 7. 布局合规复审（2026-08-14）

> 重跑命令：
> ```bash
> python3 audit_migration_layout.py --java-package-root .../freemarker --rust-root freemarker --retain-segments 2 --json
> ```

### 审计脚本数字

| 指标 | 值 |
|------|-----|
| strict_migration_blockers | **244** |
| findings 总数 | 244 |

### findings 分类

| rule | 数量 | 残留说明 |
|---|---|---|
| `missing_object_file` | 163 | 114 NA 包文件 + 15 NA_FINAL + 20 纯文档 + 14 个聚合枚举内部类（ExprKind/ElementKind/TemplateError/OutputFormatKind 的变体不单独建文件，审计脚本按 Java 文件计缺失） |
| `non_snake_case_file` | 34 | 已审例外：34 个前导下划线文件（Java `_Xxx` 内部类忠实映射，如 `_CacheAPI`/`_ExtDomApi`/`_ObjectWrappers`/`_TemplateAPI`/`_VersionInts` 等），审计脚本 non-snake 启发式误报 |
| `multiple_public_objects` | 32 | 聚合枚举既有设计：ExprKind（26 变体）/ ElementKind（~30 变体）/ TemplateError（~20 变体）/ OutputFormatKind（9 变体）等，一个文件多个 pub enum/struct 是 Rust sum type 惯用法 |
| `rust_file_over_800_lines` | 13 | 预存文件登记：13 个 >800 行文件为迁移前核心实现（eval.rs/grammar.rs/node.rs/environment.rs/settings.rs 等），非本轮新增 |
| `rust_file_over_500_lines` | 5 | 同上，5 个 500-800 行文件 |
| `misplaced_object_file` | 2 | ext/jdom / ext/xml 同名误报：审计脚本将 Java `freemarker.ext.jdom.NodeModel` 和 `freemarker.ext.xml.NodeModel` 误匹配到 Rust `xml/node.rs`（实际对应 `freemarker.ext.dom.NodeModel`），非真正错位 |

### 结论

244 个 findings 均为已知残留，无新增 blocker：
- 163 missing：NA/文档/聚合枚举内部类（设计使然）
- 34 non-snake：下划线前缀已审例外
- 32 multiple_public：聚合枚举既有设计
- 13 + 5 over_lines：预存核心文件
- 2 misplaced：脚本误报
