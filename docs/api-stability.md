# API 稳定性承诺

> 本文档定义 freemarker-rust 在各版本阶段的公共 API 稳定性策略。

---

## 1. 当前阶段（0.x）

### 1.1 策略

在 `0.x` 阶段（`0.1.0-alpha.1` ~ `0.1.0`），公共 API **不保证稳定**。
任何 minor 版本都可能包含 breaking change。

### 1.2 API 基线门禁

尽管 0.x 不承诺稳定，项目通过 `cargo public-api` CI 门禁使**任何 API 变更显式化**：

- **基线文件**：`docs/release/api-baseline.txt`（6054 项公共 API 条目）
- **CI 检查**：每次 PR 运行 `cargo public-api --diff against baseline`，diff 非零即阻断
- **变更流程**：任何基线变更必须在 PR 描述中注明原因，并经 reviewer 确认

这意味着：即使 API 可以变，每次变更都是**有意为之**，不会因意外提交引入漂移。

### 1.3 冻结窗口声明

自 **2026-08-15 `0.1.0-beta.0`** 起，至 **`0.1.0` 正式发布**前，
公共 API 基线 **diff = 0**（冻结）。

在冻结窗口内：
- 不接受公共 API 变更的 PR（除非修复 blocker issue）
- 内部实现变更（非公共 API）不受限制
- 文档/测试/CI 变更不受限制

---

## 2. 1.0 后 SemVer 承诺预览

`1.0.0` 发布后，项目遵循 [Semantic Versioning 2.0.0](https://semver.org/)：

### 2.1 版本号语义

| 变更类型 | 版本号变化 | 示例 |
|----------|-----------|------|
| **Patch**（bug 修复、文档、内部优化） | `1.0.x` | `1.0.0` → `1.0.1` |
| **Minor**（新增功能、新增公共 API、非破坏性变更） | `1.x.0` | `1.0.0` → `1.1.0` |
| **Major**（破坏性变更、移除公共 API、语义变化） | `x.0.0` | `1.0.0` → `2.0.0` |

### 2.2 破坏性变更定义

以下变更视为 **breaking change**，需要大版本号递增：

- 移除或重命名公共类型、函数、方法、trait、枚举变体
- 修改公共函数签名（参数类型、返回类型、trait bound）
- 修改公共枚举的变体集合（新增变体若枚举标记 `#[non_exhaustive]` 则非破坏性）
- 修改公共结构体的字段可见性（`pub` → 非 `pub`）
- 提升 MSRV（最低支持 Rust 版本）
- 修改现有行为语义（相同输入产生不同输出）

以下变更 **不视为** breaking change：

- 新增公共 API（函数、类型、trait）
- 在 `#[non_exhaustive]` 枚举/结构体中新增变体/字段
- 内部实现优化（行为不变）
- 文档变更
- Clippy/fmt 修复

### 2.3 破坏性变更流程

1. **CHANGELOG 记录**：所有破坏性变更必须在 `CHANGELOG.md` 的 `Breaking Changes` 小节记录
2. **基线评审**：`cargo public-api` diff 必须经 reviewer 确认
3. **迁移指南**：破坏性变更附带迁移说明（在 CHANGELOG 或用户指南中）
4. **至少 1 个 beta 版本**：破坏性变更在正式发布前至少经历 1 个 beta 版本

---

## 3. 公共 API 范围

### 3.1 构成

公共 API 由以下部分构成（以 `cargo public-api` 输出为准）：

- `freemarker` crate 中所有 `pub` 类型、函数、trait、常量
- 通过 `freemarker::template::*`、`freemarker::value::*`、`freemarker::error::*` 等路径导出的条目
- `freemarker::parser::parse`（模板解析入口）
- `freemarker::xml::parse_xml`（XML 解析入口）

### 3.2 不构成

以下不视为公共 API 的一部分（即使技术上可访问）：

- `#[doc(hidden)]` 标记的条目
- `pub(crate)` 条目
- 测试辅助模块（`freemarker-test` crate）
- `examples/` 目录中的示例代码
- 内部模块路径（`freemarker::core::*` 中未通过 `freemarker` 根重导出的条目）

### 3.3 版本敏感 API

以下 API 对版本行为敏感，变更需特别注意：

- `Settings.incompatible_improvements`：ICI 版本化行为
- `Settings.classic_compatible`：经典兼容模式
- `Settings.template_exception_handler`：异常处理策略
- `NewBuiltinClassResolver`：`?new` 类解析策略

---

## 4. 参考

| 文档 | 内容 |
|------|------|
| [`superpowers/specs/2026-08-03-versioning-design.md`](superpowers/specs/2026-08-03-versioning-design.md) | 版本治理完整设计 |
| [`superpowers/VERSION-PLAN.md`](superpowers/VERSION-PLAN.md) | 版本路线图与晋级门禁 |
| [`release/api-baseline.txt`](release/api-baseline.txt) | 当前公共 API 基线（6054 项） |
| [Semantic Versioning 2.0.0](https://semver.org/) | SemVer 规范 |
