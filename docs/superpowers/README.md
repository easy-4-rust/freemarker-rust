# freemarker-rust Superpowers 规格驱动开发体系

> 本目录遵循 [obra/superpowers](https://github.com/obra/superpowers) 执行方法层约定，
> 为 freemarker-rust 项目提供 plans（实施计划）与 specs（设计规格）的结构化管理。

## 目录结构

```
docs/superpowers/
├── README.md              # 本文件——约定与索引
├── plans/                 # 实施计划
│   └── YYYY-MM-DD-<kebab-name>.md
├── specs/                 # 设计规格（薄层映射 + 元数据）
│   └── YYYY-MM-DD-<kebab-name>-design.md
└── AUDIT-SUMMARY.md       # 历史计划审计总结
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

**定位**：specs 是**薄层映射 + 元数据补充**，不是重写。设计细节保留在已有
`docs/01-*.md` ~ `docs/12-*.md` 编号文档中。

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

## 与已有 01-12 编号文档的关系

| Superpowers 产物 | 对应已有文档 | 关系 |
|------------------|-------------|------|
| `specs/2026-08-01-project-overview-design.md` | `docs/01-项目概述与范围.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-architecture-design.md` | `docs/02-架构设计.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-parser-design.md` | `docs/03-解析器迁移设计.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-rendering-engine-design.md` | `docs/04-渲染引擎与指令迁移设计.md` | 薄层映射 + 元数据 |
| `specs/2026-08-02-builtins-design.md` | `docs/05-内建函数迁移清单.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-data-model-design.md` | `docs/06-数据模型与对象包装.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-config-cache-design.md` | `docs/07-配置缓存与加载.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-formatting-design.md` | `docs/08-格式化与自动转义.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-error-handling-design.md` | `docs/09-错误处理与诊断.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-pyo3-design.md` | `docs/10-pyo3集成设计.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-testing-strategy-design.md` | `docs/11-测试与验证策略.md` | 薄层映射 + 元数据 |
| `specs/2026-08-01-migration-roadmap-design.md` | `docs/12-迁移路线图.md` | 薄层映射 + 元数据 |

**原则**：修改设计时，先更新 `docs/XX-*.md` 源文档，再同步 specs 元数据。
specs 不复制源文档内容，只提供元数据和引用。

## 历史计划索引

| 计划文件 | 对应阶段 | 日期 | 核心交付 |
|---------|---------|------|---------|
| `2026-08-01-p0-skeleton-baseline.md` | P0 骨架与基线 | 2026-08-01 | workspace + 错误体系 + 基础类型 + L3 harness |
| `2026-08-01-p1-p4-core-implementation.md` | P1-P4 核心实现 | 2026-08-01~02 | 解析器 + 渲染引擎 + 内建函数 + 配置缓存格式化 |
| `2026-08-03-alpha0-production-readiness.md` | alpha.0 生产就绪 | 2026-08-03 | 治理门禁 + BLOCKED 清零 + 鲁棒性安全 |
| `2026-08-03-alpha1-governance-hardening.md` | alpha.1 治理收口 | 2026-08-03 | 内建 183/183 + golden 113/128 + pyo3 发布准备 |
| `2026-08-04-p5-pyo3-integration.md` | P5 pyo3 集成 | 2026-08-04 | pyo3 签名同步 + Python 绑定 |
| `2026-08-04-p6-polish-alignment.md` | P6 打磨与对齐 | 2026-08-04~05 | 文件拆分 + 语义补全 + 结构对齐 |
