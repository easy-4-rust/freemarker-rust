# 渲染引擎与指令迁移设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（Environment.java 3,709 行）
- **依赖**：解析器（P1）、数据模型（06 文档）

## 1. 目标与范围

将 Java Environment 渲染引擎迁移为 Rust 实现：指令栈 accept 模式、变量解析链、46 个指令类、基础表达式求值。

详细设计见：`docs/04-渲染引擎与指令迁移设计.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 渲染引擎与指令迁移设计 | `docs/04-渲染引擎与指令迁移设计.md` | Environment 总览（3,709 行 -> Rust）、渲染循环伪代码、指令栈 accept 模式、变量解析链（局部->命名空间->全局->数据模型）、46 个指令清单、Namespace/LazilyInitializedNamespace、数据模型最小集、SimpleObjectWrapper、TemplateCache 最小版 |

## 3. 关键设计决策

- **渲染循环**：`while let Some(el) = stack.pop() { visit(el)? }` accept 模式
- **指令栈**：`Vec<Element>` 替代 Java `TemplateElement[]`
- **变量解析链**：局部 -> 命名空间 -> 全局 -> 数据模型
- **异常处理**：`handle_error` 对应 Java `handleTemplateException`

## 4. 验收标准

1. 控制流黄金用例逐字节通过
2. 作用域用例通过
3. 流控错误消息一致
4. 性能冒烟通过

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 3-4）
