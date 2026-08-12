# 错误处理与诊断设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（TemplateException.java + 30+ Non*Exception.java）
- **依赖**：架构设计（02 文档）

## 1. 目标与范围

将 Java 异常层级迁移为 Rust 统一错误模型：TemplateError enum + ErrorCtx + 30+ 异常类镜像文件。

详细设计见：`docs/09-错误处理与诊断.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 错误处理与诊断 | `docs/09-错误处理与诊断.md` | 异常层级映射（TemplateException -> 30+ 子类）、TemplateError enum（统一错误模型）、ErrorCtx（模板名/行/列/期望值）、_ErrorDescriptionBuilder、_MessageUtil、错误消息逐字对齐策略 |

## 3. 关键设计决策

- **TemplateError 统一枚举**：Java 30+ 异常类 -> Rust 单一 enum
- **ErrorCtx 消息上下文**：模板名/行/列/期望值
- **460 处调用点零改动**：TemplateError 构造方法统一入口
- **FlowKind 流控枚举**：RETURN/BREAK/CONTINUE/STOP

## 4. 验收标准

1. 错误消息逐字对齐（Java 基线全量 diff）
2. 指令栈转储格式对齐
3. 70 场景 expected_messages 基线通过

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p0-skeleton-baseline.md`（Stage 2）
- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 7 Task 7.1）
