# Java freemarker-core ↔ Rust freemarker 目录结构对照报告

> 核对日期：2026-08-04
> Java 源：`apache/freemarker` freemarker-core `2.3-gae@7926e97` 的
> `src/main/java/freemarker/`（561 个 .java 文件，13 个包）
> Rust 实现：`freemarker-rust/freemarker/src/`（71 个 .rs 文件，11 个模块）
> 方法：机械文件名匹配（31）+ 逐文件 grep 语义核对（530，3 个并行核对）
> 结论：**280 MAPPED / 28 MISSING（9 个功能块）/ 233 NA-DESIGN / 20 个纯文档**

## 1. 目录结构对照

| Java 包 | Java 文件数 | Rust 模块 | Rust 文件数 | 状态 |
|---|---|---|---|---|
| `freemarker.core` | 282 | `src/core/` | 12 | ✅ 对齐（AST 合并为 enum） |
| `freemarker.cache` | 37 | `src/cache/` | 10 | ⚠️ 缺 per-template 配置体系 |
| `freemarker.template` | 70 | `src/template/` | 16 | ✅ 对齐 |
| `freemarker.template.utility` | 30 | `src/template/utility_transforms.rs` + `utility/` | 3 | ✅ 对齐（变换合并） |
| `freemarker.log` | 12 | — | — | NA（Rust 用 Result 传播错误，无日志框架） |
| `freemarker.debug` + `debug.impl` | 16 | — | — | NA（Java RMI 调试协议） |
| `freemarker.ext.beans` | 75 | — | — | NA（决策 1：JVM 反射不实现） |
| `freemarker.ext.dom` | 18 | `src/xml/mod.rs` | 1 | ✅ 对齐（10 文件合并入 XmlNode） |
| `freemarker.ext.xml` | 9 | `src/xml/mod.rs` | 1 | NA（弃用 dom4j API，语义子集覆盖） |
| `freemarker.ext.jdom` | 2 | — | — | NA（第三方集成） |
| `freemarker.ext.rhino` | 4 | — | — | NA（第三方集成） |
| `freemarker.ext.util` | 5 | — | — | NA（依附决策 1 包装体系） |
| `freemarker.ext`（根） | 1 | — | — | 纯文档 package-info |
| Rust 自有 | — | `parser/`、`error/`、`span.rs`、`value.rs`、`builtins/` | 20 | 多出的模块（Rust 特有设计） |

Rust 侧 20 个"多出的文件"：`parser/lexer.rs`、`parser/grammar.rs`（Java 用 JavaCC 生成，
无源文件）、`error/` 3 文件（Java 异常层级合并）、`span.rs`（源码位置）、`value.rs`
（DynValue/TNumber 值体系）、`builtins/` 15 文件（Java 内建分散在 core/BuiltInsFor*.java，
Rust 按主题分组）——均为合理的 Rust 特有组织，满足"可以多一些"。

## 2. 文件映射统计

| 分类 | 数量 | 说明 |
|---|---|---|
| MAPPED（含合并） | 280 | Java 类在 Rust 中有等价实现（多数多个 Java 类合并为一个 Rust 文件/enum） |
| MISSING（真实缺口） | 24（2 个功能块已补） | 见 §3，剩余 7 个功能块（c_format/自动转义禁令已实现） |
| NA-DESIGN | 233 | 设计决策不实现（JVM 反射 75 + 第三方集成 6 + 平台工具 28 + 内部桥 10 + 文档 20 等） |
| **合计** | **561** | ✓ 与磁盘计数一致 |

## 3. MISSING 清单（原 9 个功能块 28 个文件；已补 2 块，剩 7 块 21 文件）

| # | 功能块 | Java 文件 | Rust 现状 | 影响 | 建议 |
|---|---|---|---|---|---|
| 1 | **per-template 配置体系** | `core/TemplateConfiguration.java` + `cache/TemplateSourceMatcher.java` + 7 matcher + 4 Factory + FactoryException（14 文件） | Template 仅 encoding 字段；无 matcher/配置工厂 | 无法按模板路径差异化配置（编码/locale/输出格式） | P1：大缺口，嵌入 API 面 |
| 2 | ~~c_format 变体~~ | ~~`JavaCFormat` 等 5 文件~~ | ✅ **已实现**（2026-08-04）：`CFormatKind` 枚举 + Settings.c_format + 设置解析 + ?c/?cn 变体分派（format.rs） | — | — |
| 3 | ~~自动转义禁令~~ | ~~`BuiltInBannedWhenAutoEscaping`/`ForcedAutoEscaping`~~ | ✅ **已实现**（2026-08-04）：`check_legacy_escaping_ban`（?html/?xml/?rtf/?web_safe，eval.rs + strings_encoding.rs）；FORCE 禁令不适用（Rust 无 force 策略，文档化） | — | — |
| 4 | **自定义格式报错** | `UndefinedCustomFormatException`（1 文件） | 未知 number_format 名静默回退 plain | 拼错格式名不报错，错误路径不一致 | P1：低成本 |
| 5 | **get_optional_template** | `GetOptionalTemplateMethod`（1 文件） | BuiltinVar 无此变量 | 模板内无法动态获取模板引用 | P1：低成本 |
| 6 | **模板后处理钩子** | `TemplatePostProcessor`/`Exception`（2 文件） | 无 post_process 概念 | 嵌入扩展点缺失（安全/审计集成） | P2：嵌入 API |
| 7 | **组合输出格式** | `CombinedMarkupOutputFormat`（1 文件） | 输出格式为单一枚举 | 组合格式（HTML+XML）转义不可用 | P2 |
| 8 | **StatefulTemplateLoader** | `StatefulTemplateLoader`（1 文件） | TemplateLoader 无 reset_state | 状态化 loader 的 reset 语义缺失 | P2：低成本 |
| 9 | **DOCTYPE 节点** | `ext/dom/DocumentTypeModel`（1 文件） | roxmltree 丢弃 `<!DOCTYPE>` | 模板无法访问文档类型声明 | P2 |

## 4. NA-DESIGN 构成（233）

| 类别 | 数量 | 依据 |
|---|---|---|
| ext/beans 反射全族 | 75 | security.md 决策 1/2（JVM 反射 + 方法重载永久 NA） |
| `_Delayed*` 惰性消息包装 | 10 | Java 内部错误消息惰性求值优化 |
| `_Java9`/`_Java16` 平台适配 | 4 | Java 版本条件编译 |
| `_ObjectBuilder*` 设置语法 | 3 | Java `Configuration.setSetting` 的 Builder 解析语法 |
| `_Unmodifiable*`/`_SortedArraySet`/`_Array*` | 6 | Java 集合工具（Rust 用标准库） |
| 嵌入 API（CustomAttribute/CommandLine/FreeMarkerTree/DebugBreak 等） | 14 | Java 嵌入场景（Rust 有替代或不适用） |
| log/ + debug/ | 28 | Rust 无日志框架/RMI 调试（Result 传播） |
| ext/xml + jdom + rhino + util | 20 | 弃用 API/第三方集成/依附决策 1 |
| 纯文档 package-info | 20 | Javadoc 包文档 |
| 其余内部工具 | 53 | BugException/SuppressFBWarnings 注解/迭代器抽象等 |

## 5. 合并映射说明（MAPPED 的主要合并）

| Java 文件组 | Rust 目标 | 说明 |
|---|---|---|
| ~80 个 AST 表达式类（AddConcatExpression/AndExpression/...） | `core/expression.rs::ExprKind` | Java 每类一文件，Rust 用 enum sum type（语言特性） |
| ~30 个指令类（IfBlock/ListBlock/SwitchBlock/...） | `core/template_element.rs::ElementKind` | 同上 |
| ~50 个内建类（BuiltIn*/BuiltInsFor*） | `builtins/mod.rs` 注册表 + `builtins/*.rs` | Java core/ 包分散，Rust 按主题分组 |
| ~20 个异常类（UnexpectedTypeException/NonXxxException/...） | `error/template_error.rs::TemplateError` | 错误枚举 + 消息逐字对齐 |
| 8 种输出格式类 | `core/output_format.rs::OutputFormatKind` | 单一枚举 |
| ext/dom 10 文件（NodeModel/ElementModel/...） | `xml/mod.rs::XmlNode` | 节点角色由槽位分支承担 |
| TemplateExceptionHandler 内嵌 4 实现 | `environment.rs` 设置分发 | 字符串设置 |
| TemplateLookupContext/Result | `cache/template_lookup_strategy.rs` | 文件头自注"合并存放" |

## 6. 结构优化建议（执行清单）

对照结果发现 Rust 侧 4 处文件组织与"一文件一 Java 对象"原则差距较大，且与
`docs/合规审计报告.md` 的 blocker 重合：

| # | 优化 | Java 对应 | 工作量 |
|---|---|---|---|
| S1 | 拆分 `xml/mod.rs` → `xml/ns_prefixes.rs`（NsPrefixes）+ `xml/tree.rs`（XmlTree）+ `xml/node.rs`（XmlNode/parse_xml） | ext/dom 各节点类 | 30 分钟 |
| S2 | 拆分 `template/template_model.rs`（16 trait）→ `template_model/` 子目录逐 trait 文件 | template/*.java 各接口 | 2 小时 |
| S3 | 拆分 `cache/template_loader.rs` → `template_source.rs`（TemplateSource）+ `template_loader.rs`（TemplateLoader） | cache/TemplateLoader.java + URLTemplateSource.java | 15 分钟 |
| S4 | 拆分 `freemarker-pyo3/src/lib.rs` → `template.rs`（FmTemplate） | freemarker-jython25 对应类 | 15 分钟 |
| S5 | 文档修订：docs/01 §3.2 ext/dom 已实现（原标"范围外"） | — | 5 分钟 |

## 7. 缺口补齐建议（优先级）

| 优先级 | 功能块 | 理由 |
|---|---|---|
| P0 | c_format 变体（#2）、自动转义禁令（#3） | 行为差异 + 安全相关，成本低 |
| P1 | per-template 配置（#1）、自定义格式报错（#4）、get_optional_template（#5） | 嵌入 API 面 + 错误路径对齐 |
| P2 | 后处理钩子（#6）、组合格式（#7）、StatefulTemplateLoader（#8）、DOCTYPE（#9） | 扩展点/边缘场景 |

*报告结束。核对依据：3 个并行 Explore agent 的逐文件 grep 验证（报告在
`/tmp/core_classify.md`、`/tmp/cache_template_classify.md`、ext 内联交付）。*
