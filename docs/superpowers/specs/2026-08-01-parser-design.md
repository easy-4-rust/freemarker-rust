# 解析器迁移设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（FTL.jj 4,845 行，JavaCC 7.0.12）
- **依赖**：无外部解析器生成器（手写递归下降）

## 1. 目标与范围

将 JavaCC 生成的 FMParser 迁移为 Rust 手写递归下降解析器：5 个词法状态、24 个表达式产生式、13 个指令产生式。

详细设计见：`docs/03-解析器迁移设计.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 解析器迁移设计 | `docs/03-解析器迁移设计.md` | 词法设计（5 状态 + 全 token 清单）、表达式产生式（24 个）、指令产生式（13 个 + FreemarkerDirective 分发）、AST 通用属性、字符串插值解析、空白剥离、解析错误消息 |

## 3. 关键设计决策

- **手写递归下降**：不引 pest/nom——FreeMarker 词法有 5 个状态且指令/表达式上下文交织
- **5 个词法状态**：DEFAULT/IN_PAREN/NO_SPACE_EXPRESSION/NAMED_PARAMETER_EXPRESSION/NO_PARSE
- **错误消息逐字对齐**：行/列/期望清单格式与 Java 基线一致

## 4. 验收标准

1. 套件全部模板可解析
2. 错误消息行/列/期望清单对齐
3. `[#ftl]`/注释/方括号语法/插值/lambda/范围边界用例通过

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 1-2）
