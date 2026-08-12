# 配置缓存与加载设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（Configuration.java 3,877 行 + cache/ 37 文件）
- **依赖**：数据模型（06 文档）、格式化（08 文档）

## 1. 目标与范围

将 Java Configuration/Configurable/TemplateConfiguration + cache/ 迁移为 Rust 实现：设置继承链、TemplateConfiguration matcher 链、TemplateLoader 全家族、TemplateCache 完整版、CacheStorage 家族。

详细设计见：`docs/07-配置缓存与加载.md`

## 2. 设计来源

| 文档 | 路径 | 核心内容 |
|------|------|----------|
| 配置缓存与加载 | `docs/07-配置缓存与加载.md` | 配置继承链（Configurable/TemplateConfiguration/Environment/Configuration）、Settings 结构体（Option<T> 继承语义）、TemplateConfiguration matcher 链（FirstMatch/Merging/Conditional + 5 类 matcher）、TemplateLoader 全家族、TemplateCache 完整版、CacheStorage 家族 |

## 3. 关键设计决策

- **设置继承**：`Option<T>` 表达"未设置"，父链向上查找
- **SettingsSnapshot**：继承链解析后的不可变快照，渲染期读取零锁
- **per-template 配置**：TemplateConfiguration 按路径/属性条件匹配的模板级配置
- **CacheStorage**：Strong/MRU（Weak 软段），v1 无容量/过期策略

## 4. 验收标准

1. dateformat/encoding/whitespace 用例通过
2. `?c/?cn` 五格式一致
3. 缓存时序用例通过
4. 并发冒烟

## 5. 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 6）
- `docs/superpowers/plans/2026-08-04-p6-polish-alignment.md`（Task 4.1：per-template 配置补齐）
