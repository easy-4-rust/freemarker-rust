# 测试覆盖率审计报告

- **日期**：2026-08-04
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97）
- **依赖**：`2026-08-01-testing-strategy-design.md`

---

# 测试覆盖率审计报告（cargo-llvm-cov）

> 日期：2026-08-04 ｜ 基线：Rust `f4de1bf` ｜ 命令：`cargo llvm-cov --workspace --exclude freemarker-pyo3 --summary-only`
> 技能依据：rust-java-migration-testing（覆盖率是信号，不是完成证明；不得通过排除难文件/加琐碎 getter 达标）

## 1. 总体结果

| 指标 | 数值 |
|---|---|
| 行覆盖率 | **85.10%**（45209 行，missed 6738；2026-08-04 markup 双槽实现后终值） |
| 分支覆盖率 | 81.85% |
| 区域覆盖率 | 85.14% |
| 测试总数 | 972 passed / 0 failed（workspace，exclude pyo3） |
| clippy / fmt | 0 warning / 干净 |

较迁移前基线（84.67% 行 / 897 tests）：+0.43pp 行覆盖、+79 测试（976）。

## 2. 覆盖范围与排除

- 覆盖对象：`freemarker/` 引擎 crate（含 freemarker-test 的测试驱动执行）
- 排除：`freemarker-pyo3`（Python 绑定，独立打包面）
- 低覆盖文件构成（146 个 <60% 文件，行数占比小但拖累行覆盖率）：

### A 类：锚点/文档/空壳文件（~120 个，行为为空，行多为 doc 注释与 `#[allow(dead_code)]` 锚点）
- `error/` 异常镜像 17 文件（构造器锚点，`TemplateError` 构造已委托、调用点走 impl 方法）
- `core/*_output_format.rs` 14 文件（Java 输出格式类锚点，escape 逻辑委托 utility）
- `template/` 空壳类型（SimpleScalar/SimpleNumber/simple_date/false/true_template_boolean_model/
  general_purpose_nothing/wrapping_template_model 等——Rust 统一由 `TModel` 承载，空壳为 Java 1:1 镜像）
- `xml/` 模型类锚点 11 文件（Java ext.dom 类对应物）

这些文件按技能"一文件一 Java 对象"要求保留镜像，行覆盖率低是设计使然；**不为覆盖率硬凑测试**（技能红线）。

### B 类：真实行为缺口（已补或待补）

| 模块 | 迁移前 | 迁移后 | 说明 |
|---|---|---|---|
| xml/node.rs | 59.36% | **67.31%** | 新增 xml_coverage.rs 11 测试（子元素/@attr/@@key/*/**/XPath 子集/节点内建/命名空间前缀） |
| xml/xml_dom_string_util.rs | 68.69% | **82.83%** | 转义/判定辅助随 node.rs 覆盖提升 |
| template/utility/to_canonical.rs | 5% | ~95% | 单元测试（?c 数字输出全分支） |
| cache/template_source.rs | 3% | ~90% | trait 实现测试 |
| logging_attempt_exception_reporter.rs | 2% | ~90% | 静默实现测试 |

## 3. 引擎缺口登记（测试揭示，按 Java 原语义实现）

**已实现（本审计前提交）：**
- `?absolute_template_name` 内建（BuiltIn.java absoluteTemplateNameBI）
- `.callerTemplateName` 特殊变量（BuiltinVariable.java CALLER_TEMPLATE_NAME）
- `?esc`/`?no_esc` markup 语义（autoEsc 下不二次转义）、`?markup_string`
- `?with_args`/`?with_args_last` 宏/函数支持（BuiltInsForCallables.java）
- `NewBuiltinClassResolver` 设置解析剥引号（SettingStringParser 语义）
- `CombinedMarkupOutputFormat` 组合格式（?c "HTML+XML"）
- `Environment` custom state API（getCustomState/setCustomState）
- 三层 auto include/import 分层（Configuration/Environment 继承合并）

**剩余（JVM 特有 API 面，保持 NOT_APPLICABLE）：**
- TemplateProcessingTracer、ThreadInterruptingSupport、DirectiveCallPlace
- ext.beans 26 测试类（JVM 反射，java-tests/NOT_APPLICABLE.md 已登记）
- ext.dom 3 测试类（org.w3c.dom/Jaxen XPath API 面）

**XML 边界缺口（本审计记录）：**
- `@@nodeName` 键未实现（Java AtAtKey；可用 `?node_name` 等价）
- 带默认命名空间文档上的 `//name` XPath 匹配差异（无命名空间文档正常）

## 4. 诚实结论

- 行覆盖率 85.19%，未达技能检查表的 ≥95% 门槛
- 差距构成：A 类锚点/文档文件 ~120 个（行为为空，行覆盖率低为镜像设计使然，
  按技能红线不硬凑测试）；B 类真实缺口已补主要模块（xml 67%、utility/cache 辅助 90%+）
- 引擎语义：Java 测试迁移 115/149 类（78%），34 类 NA（JVM 反射/DOM API，
  有登记证据）；测试揭示的引擎缺口已实现 8 项，剩余为 JVM 特有面
- 下一步建议：若需逼近 95%，可评估 (a) 将锚点文件标记为覆盖排除并记录原因
  （技能允许"文档化排除"），(b) 继续补 xml/node.rs 剩余分支与 format 模块测试

---

## 对应计划

- `docs/superpowers/plans/2026-08-04-coverage-test-completion.md`
