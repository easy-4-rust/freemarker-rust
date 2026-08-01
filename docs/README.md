# freemarker-rust 迁移计划文档

> 目标：基于 [Apache FreeMarker](https://github.com/apache/freemarker)（2.3-gae 分支，commit `7926e97`，incompatibleImprovements 2.3.34 开发线）实现**语义功能一致**的 freemarker-rust。
> 核心迁移：`freemarker-core` → `freemarker-rust`（Rust 引擎）；`freemarker-jython25` → `freemarker-pyo3`（PyO3 0.29 Python 桥接）。
> 分析基线：codegraph 知识图谱（926 文件 / 11,798 节点 / 72,272 边 / 1,211 执行流），总览见工作区根 `freemarker-rust-迁移分析报告.md`。

## 文档导航

| # | 文档 | 内容 | 状态 |
|---|---|---|---|
| 01 | [项目概述与范围](./01-项目概述与范围.md) | 目标、版本基线、范围界定、非目标、成功标准 | ✅ |
| 02 | [架构设计](./02-架构设计.md) | workspace 布局、crate 划分、依赖选型、核心设计模式（角色槽位模型、指令栈） | ✅ |
| 03 | [解析器迁移设计](./03-解析器迁移设计.md) | FTL.jj（4,845 行）全量映射：词法状态机、24 表达式产生式、13 指令产生式、AST 草案 | ✅ |
| 04 | [渲染引擎与指令迁移设计](./04-渲染引擎与指令迁移设计.md) | Environment 指令栈 + accept 模式、表达式求值、指令全清单、作用域与命名空间 | ✅ |
| 05 | [内建函数迁移清单](./05-内建函数迁移清单.md) | 133 个内建函数全清单、按类分组、语义风险点 | ✅ |
| 06 | [数据模型与对象包装](./06-数据模型与对象包装.md) | TemplateModel trait 体系、Simple* 实现、ObjectWrapper/BeanWrapper | ✅ |
| 07 | [配置缓存与加载](./07-配置缓存与加载.md) | Configurable 设置项全表、TemplateLoader 家族、TemplateCache、名称规范化 | ✅ |
| 08 | [格式化与自动转义](./08-格式化与自动转义.md) | OutputFormat 体系、CFormat、日期/数字格式化、自动转义、空白处理 | ✅ |
| 09 | [错误处理与诊断](./09-错误处理与诊断.md) | 异常层级映射、错误消息逐字对齐、行号/指令栈上下文 | ✅ |
| 10 | [pyo3 集成设计](./10-pyo3集成设计.md) | freemarker-pyo3：wrap/unwrap 双向桥、GIL 策略、异常桥接、Python 测试套件 | ✅ |
| 11 | [测试与验证策略](./11-测试与验证策略.md) | 三层测试、testcases.xml 黄金套件移植、Java 对比测试、性能基准 | ✅ |
| 12 | [迁移路线图](./12-迁移路线图.md) | P0–P6 阶段任务分解、验收标准、工作量估算、依赖关系 | ✅ |

## 核心结论速览（来自图谱分析）

1. **渲染模型**：`Environment.visit` 指令栈 + `TemplateElement.accept(env) → Vec<下一指令>` 模式（`core/Environment.java:340/:367`，后者为手写内联优化版）——Rust 必须保留此语义。
2. **解析器**：JavaCC 7.0.12 生成 FMParser，语法源 `freemarker-core/src/main/javacc/freemarker/core/FTL.jj`（4,845 行）→ Rust 手写递归下降。
3. **内建函数**：30 个 `BuiltInsFor*.java` 文件、133 个 BI 静态类、7 类特化基类。
4. **jython25 本质**：本体仅 1 个版本适配器（48 行）；完整桥接实现在 `freemarker-jython20/ext/jython/`（13 文件）——pyo3 迁移的语义参考。
5. **黄金测试集**：`freemarker-jython25/src/test/resources/freemarker/test/templatesuite/testcases.xml`（100+ 模板）+ `expected/*.txt` 期望输出——语义一致性的最终判据。

## 计划文档编写约定

- 所有"源文件位置"均指 `/Users/wandl/workspaces/workspace-github/freemarker/freemarker-core/src/main/java/freemarker/` 下相对路径（除非注明）。
- `✅` = 已编写；`🔄` = 进行中；`⬜` = 待编写。
- 迁移基准版本：FreeMarker 2.3.x（incompatibleImprovements 支持到 2.3.34），Java 8+，JavaCC 7.0.12。
