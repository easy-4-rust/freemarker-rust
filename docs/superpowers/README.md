# freemarker-rust Superpowers 规格驱动开发体系

> 本目录遵循 [obra/superpowers](https://github.com/obra/superpowers) 执行方法层约定，
> 为 freemarker-rust 项目提供 plans（实施计划）与 specs（设计规格）的结构化管理。

## 目录结构

```
docs/superpowers/
├── README.md              # 本文件——约定与索引
├── VERSION-PLAN.md        # 版本规划（快照 + 路线图 + 晋级门禁映射）
├── AUDIT-SUMMARY.md       # 历史计划合规审计总结
├── plans/                 # 实施计划（12 个）
│   └── YYYY-MM-DD-<kebab-name>.md
└── specs/                 # 设计规格（25 个，完整内容 + 元数据）
    └── YYYY-MM-DD-<kebab-name>-design.md
```

## Plans 约定

**命名规则**：`YYYY-MM-DD-<kebab-name>.md`

**日期**：使用真实 git 提交日期或版本发布日期，不编造。

**格式**（参照 liteflow 约定）：

```markdown
# <计划标题>

> **For agentic workers:** REQUIRED SUB-SKILL: ...

**Goal:** 一句话目标
**Architecture:** 架构概要
**Tech Stack:** 技术栈
**Related Design Doc:** `docs/superpowers/specs/...` 或 `docs/...`

---

## 全局约定

---

## 实施阶段总览

| Stage | 目标 | 预期 Task 数 |
|-------|------|-------------|
| 1     | ...  | N           |

## Stage N — <阶段标题>

### Task N.M：<任务标题>

**Files:**
- Create: ...
- Modify: ...
- Test: ...

- [ ] **Step 1: ...**
- [ ] **Step 2: ...**
```

**Task 状态标记**（审计时使用）：
- `- [x]` 已完成
- `- [ ]` 未完成
- `- [~]` 部分完成（附说明）

## Specs 约定

**命名规则**：`YYYY-MM-DD-<kebab-name>-design.md`

**定位**：specs 是**完整设计文档**，包含 frontmatter（日期/作者/状态/依赖）、完整设计内容、
以及对应计划引用。specs 是设计规格的唯一事实源。

**格式**：

```markdown
# <规格标题>

- **日期**：YYYY-MM-DD
- **作者**：freemarker-rust 团队
- **状态**：已实施 | 实施中 | 待实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97）
- **依赖**：...

## 1. 目标与范围

简要描述。详细设计见：`docs/XX-xxx.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| ...  | ...  | ...      |

## 3. 关键设计决策

列出影响实现方向的核心决策（引用决策编号 D1-D5 等）。

## 4. 验收标准

引用源文档中的验收条件。
```

## 规格索引（25 个）

| 规格文件 | 日期 | 核心内容 |
|---------|------|---------|
| `specs/2026-08-01-project-overview-design.md` | 2026-08-01 | 项目目标、版本基线、迁移范围、D1-D5 决策、验收标准 |
| `specs/2026-08-01-architecture-design.md` | 2026-08-01 | Workspace 布局、模块结构、TModel 角色槽位、错误模型 |
| `specs/2026-08-01-parser-design.md` | 2026-08-01 | 5 词法状态、24 表达式产生式、13 指令产生式 |
| `specs/2026-08-01-rendering-engine-design.md` | 2026-08-01 | Environment 渲染循环、指令全清单、变量解析、作用域 |
| `specs/2026-08-02-builtins-design.md` | 2026-08-02 | 183 个内建函数完整清单、语义风险点 |
| `specs/2026-08-01-data-model-design.md` | 2026-08-01 | TemplateModel 接口家族、TNumber、ObjectWrapper |
| `specs/2026-08-01-config-cache-design.md` | 2026-08-01 | 50+ 设置项、TemplateCache、TemplateLoader |
| `specs/2026-08-01-formatting-design.md` | 2026-08-01 | OutputFormat 体系、CFormat、自动转义 |
| `specs/2026-08-01-error-handling-design.md` | 2026-08-01 | 异常层级→Rust enum、ErrorCtx、70 场景 parity |
| `specs/2026-08-01-pyo3-design.md` | 2026-08-01 | pyo3 模块、wrap/unwrap 矩阵、GIL 策略 |
| `specs/2026-08-01-testing-strategy-design.md` | 2026-08-01 | L1/L2/L3 测试金字塔、黄金套件、性能基准 |
| `specs/2026-08-01-migration-roadmap-design.md` | 2026-08-01 | P0-P6 阶段划分、WBS、工作量估算 |
| `specs/2026-08-03-versioning-design.md` | 2026-08-03 | 版本晋级规则、SemVer 承诺、门禁条件 |
| `specs/2026-08-03-security-model-design.md` | 2026-08-03 | 威胁模型、受限子集、15 项永久 NA |
| `specs/2026-08-03-publishing-design.md` | 2026-08-03 | 端到端发布流程、阻断点、演练 Checklist |
| `specs/2026-08-04-java-rust-structure-mapping-design.md` | 2026-08-04 | 561 Java↔90 Rust 对照、422 MAPPED / 4 MISSING |
| `specs/2026-08-04-compliance-audit-design.md` | 2026-08-04 | 88% 合规、33 项违规、整改建议 |
| `specs/2026-08-16-java-upstream-architecture-design.md` | 2026-08-16 | Java 基线机制级事实：内建注册/变量链/缓存/格式工厂/错误装配（CodeGraph 实证） |
| `specs/2026-08-16-rust-side-architecture-design.md` | 2026-08-16 | Rust 实现机制级事实：渲染链/分派/变量链/缓存/格式化/桥（与 Java 报告配对） |
| `specs/2026-08-02-rust-obligation-ledger-design.md` | 2026-08-02 | R1-R20 Rust 正确性义务账本（所有权/类型化错误/数值精度/UTF-16/pyo3 GIL 等） |
| `specs/2026-08-02-value-add-ledger-design.md` | 2026-08-02 | V1-V16 VALUE_ADD 测试账本（真实 bug 捕获：运算舍入/死循环/死锁/.args 急切构建/float 格式化） |
| `specs/2026-08-03-migration-parity-ledger-design.md` | 2026-08-03 | SOURCE_PARITY 128 用例 disposition 对照表（113 MIRRORED / 15 NOT_APPLICABLE / 0 BLOCKED） |
| `specs/2026-08-03-acceptance-report-design.md` | 2026-08-03 | 迁移测试验收报告 v13（golden 113/128、864 tests 全绿、门禁结果） |
| `specs/2026-08-03-production-readiness-audit-design.md` | 2026-08-03 | 生产就绪审计（1.0 晋级 8 条件核查、golden 定格、发布就绪） |
| `specs/2026-08-04-coverage-audit-design.md` | 2026-08-04 | 测试覆盖率审计（行 85.10%、A/B 类文件分类、引擎缺口登记） |

**原则**：specs 是设计规格的唯一事实源。修改设计时直接更新对应 spec 文件。

## 历史计划索引

| 计划文件 | 对应阶段 | 日期 | 核心交付 |
|---------|---------|------|---------|
| `2026-08-01-p0-skeleton-baseline.md` | P0 骨架与基线 | 2026-08-01 | workspace + 错误体系 + 基础类型 + L3 harness |
| `2026-08-01-p1-p4-core-implementation.md` | P1-P4 核心实现 | 2026-08-01~02 | 解析器 + 渲染引擎 + 内建函数 + 配置缓存格式化 |
| `2026-08-03-alpha0-production-readiness.md` | alpha.0 生产就绪 | 2026-08-03 | 治理门禁 + BLOCKED 清零 + 鲁棒性安全 |
| `2026-08-03-alpha1-governance-hardening.md` | alpha.1 治理收口 | 2026-08-03 | 内建 183/183 + golden 113/128 + pyo3 发布准备 |
| `2026-08-04-m5-error-alignment.md` | M5 错误对齐收尾 | 2026-08-03 | ErrorCtx 装箱 + 70 场景 parity + fuzz 防御 |
| `2026-08-04-compliance-and-publish-prep.md` | 合规审计与发布准备 | 2026-08-03~04 | 合规审计 88% + pyo3 元数据 + 根 README |
| `2026-08-04-p5-pyo3-integration.md` | P5 pyo3 集成 | 2026-08-04 | pyo3 签名同步 + Python 绑定 |
| `2026-08-04-refactor-2c-3a-3b-batches.md` | 文件拆分批次 2c/3a/3b | 2026-08-04 | template/utility 拆分 + 功能块缺口补齐 |
| `2026-08-04-builtins-coverage-rounds.md` | 内建对齐 + 核心拆分 + 测试 | 2026-08-04 | AST/指令/异常/格式拆分 + builtins 对齐 + xml + java_ported |
| `2026-08-04-coverage-test-completion.md` | 覆盖率补齐 + 安全文档 | 2026-08-04 | XML 深度覆盖 + 单元测试补全 + 对照文档更新 |
| `2026-08-04-p6-polish-alignment.md` | P6 打磨与对齐 | 2026-08-04~05 | 文件拆分 + 语义补全 + 结构对齐 |
| `2026-08-05-parser-on-evaljson.md` | parser #on/?eval_json | 2026-08-05 | #on 指令 + ?eval_json 内建（P6 收官） |
| `2026-08-14-layout-parity-migration.md` | 布局对齐轮 | 2026-08-14 | 472 文件 1:1 + PostProcessor/DOCTYPE 实现 |
| `2026-08-15-production-readiness.md` | 生产就绪（beta.0） | 2026-08-15 | pyo3 35 方法 + grammar 拆分 + soak + 发版 |
