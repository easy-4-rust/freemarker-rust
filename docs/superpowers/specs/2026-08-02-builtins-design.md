# 内建函数迁移设计

- **日期**：2026-08-02
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（BuiltInsFor*.java 30 文件，183 个 BI 静态类）
- **依赖**：表达式求值（P2）、数据模型（06 文档）

## 1. 目标与范围

将 Java 183 个内建函数全量迁移为 Rust 实现：按 17 个文件分组（strings/numbers/dates/sequences/hashes/nodes/existence/callables/lazy/loop_vars/multi/markup_outputs/format/iso_date/java_date/strings_encoding/strings_misc/strings_regexp）。

详细设计见：`docs/05-内建函数迁移清单.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 内建函数迁移清单 | `docs/05-内建函数迁移清单.md` | 183 个 BI 完整清单（按源文件分组 17 组）、每个 BI 的语义要点（对照 Java 逐项复刻）、特殊基类映射、UTF-16 语义注意事项 |

## 3. 关键设计决策

- **注册表模式**：`builtins/mod.rs` 按名称查找 + 参数解析（`&'static str -> BuiltInKind`）
- **按主题分组**：Java core/ 包分散，Rust 按主题分组（strings/numbers/dates/...）
- **UTF-16 语义**：Java 用 UTF-16 code unit，Rust 需对齐（`length`/`substring`/`pad`/`truncate`）
- **183/183 全覆盖**：含最后补齐的 eval_json/is_date_like/next_sibling/previous_sibling/web_safe

## 4. 验收标准

1. string-builtins1/2、regexps、encoding-builtins、list 系列、boolean-formatting、type-builtins 逐字节通过
2. 183 BI 全注册（编译期清单核对）
3. golden 套件 MIRRORED 113/128（88%）

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 5）
- `docs/superpowers/plans/2026-08-03-alpha1-governance-hardening.md`（Task B1：最后 5 个 BI 补齐）
