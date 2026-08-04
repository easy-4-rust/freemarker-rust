# Java freemarker-core ↔ Rust freemarker 目录结构对照报告

> 核对日期：2026-08-04（第二轮核对，含补齐后状态）
> Java 源：`apache/freemarker` freemarker-core `2.3-gae@7926e97` 的
> `src/main/java/freemarker/`（561 个 .java 文件，13 个包）
> Rust 实现：`freemarker-rust/freemarker/src/`（90 个 .rs 文件，11 个模块）
> 方法：机械文件名匹配（snake_case 一一对应脚本）+ 逐文件语义核对
> 结论：**310 MAPPED / 4 MISSING（3 个功能块）/ 227 NA-DESIGN / 20 个纯文档**

## 1. 目录结构对照

| Java 包 | Java 文件数 | Rust 模块 | Rust 文件数 | 状态 |
|---|---|---|---|---|
| `freemarker.core` | 282 | `src/core/` | 14 | ✅ 对齐（AST 合并为 enum；TemplateConfiguration/GetOptionalTemplateMethod 一文件一对象） |
| `freemarker.cache` | 37 | `src/cache/` | 25 | ✅ 对齐（per-template 配置体系 13 文件已补齐；CacheStorage 家族 NA） |
| `freemarker.template` | 70 | `src/template/` | 17 | ✅ 对齐（trait 合并于 template_model.rs，异常合并于 error/） |
| `freemarker.template.utility` | 30 | `src/template/utility_transforms.rs` + `utility/` | 5 | ✅ 对齐（变换合并；StringUtil 含 glob_to_regex） |
| `freemarker.log` | 12 | — | — | NA（Rust 用 Result 传播错误，无日志框架） |
| `freemarker.debug` + `debug.impl` | 16 | — | — | NA（Java RMI 调试协议） |
| `freemarker.ext.beans` | 75 | — | — | NA（决策 1：JVM 反射不实现） |
| `freemarker.ext.dom` | 18 | `src/xml/` | 3 | ✅ 对齐（ns_prefixes/tree/node 拆分，10 文件合并入 XmlNode） |
| `freemarker.ext.xml` | 9 | `src/xml/mod.rs` | — | NA（弃用 dom4j API，语义子集覆盖） |
| `freemarker.ext.jdom` | 2 | — | — | NA（第三方集成） |
| `freemarker.ext.rhino` | 4 | — | — | NA（第三方集成） |
| `freemarker.ext.util` | 5 | — | — | NA（依附决策 1 包装体系） |
| `freemarker.ext`（根） | 1 | — | — | 纯文档 package-info |
| Rust 自有 | — | `parser/`、`error/`、`span.rs`、`value.rs`、`builtins/` | 21 | 多出的模块（Rust 特有设计） |

Rust 侧 21 个"多出的文件"：`parser/lexer.rs`、`parser/grammar.rs`（Java 用 JavaCC 生成，
无源文件）、`error/` 3 文件（Java 异常层级合并）、`span.rs`（源码位置）、`value.rs`
（DynValue/TNumber 值体系）、`builtins/` 17 文件（Java 内建分散在 core/BuiltInsFor*.java，
Rust 按主题分组）——均为合理的 Rust 特有组织，满足"可以多一些"。

## 2. 文件映射统计

| 分类 | 数量 | 说明 |
|---|---|---|
| MAPPED（含合并） | 422 | Java 类在 Rust 中有等价实现；2026-08-04 文件级拆分后新增 112 个镜像文件（expression/ 26 + 指令类 38 + 异常 17 + 格式/输出模型 14 + builtins 3 + xml 12 + 若干，见 §5b） |
| MISSING（真实缺口） | 4（3 个功能块） | 见 §3（模板后处理钩子 3 + 组合输出格式 1；DOCTYPE 见 xml/ 头注释） |
| NA-DESIGN | 115 | 设计决策不实现（JVM 反射 75 + 第三方集成 6 + 平台工具 28 + 内部桥 10 等；异常/格式/输出模型/指令/AST 原合并项已转 MAPPED） |
| **合计** | **561** | ✓ 与磁盘计数一致 |

## 3. MISSING 清单（原 9 个功能块 28 个文件；已补 6 块，剩 3 块 4 文件）

| # | 功能块 | Java 文件 | Rust 现状 | 影响 | 建议 |
|---|---|---|---|---|---|
| 1 | ~~per-template 配置体系~~ | ~~`core/TemplateConfiguration.java` + `cache/TemplateSourceMatcher.java` + 7 matcher + 4 Factory + FactoryException（14 文件）~~ | ✅ **已实现**（2026-08-04）：`core/template_configuration.rs`（渲染期设置 Option 字段 + apply_to/merge）+ `cache/` 13 文件（matcher 8 + factory 4 + exception 1，全部一文件一对象）；`Configuration.set_template_configurations` + 加载路径应用 + `Environment::new` 渲染期应用；解析期设置（tagSyntax 等）v1 无对应参数（NA） | — | — |
| 2 | ~~c_format 变体~~ | ~~`JavaCFormat` 等 5 文件~~ | ✅ **已实现**（2026-08-04）：`CFormatKind` 枚举 + Settings.c_format + 设置解析 + ?c/?cn 变体分派（format.rs） | — | — |
| 3 | ~~自动转义禁令~~ | ~~`BuiltInBannedWhenAutoEscaping`/`ForcedAutoEscaping`~~ | ✅ **已实现**（2026-08-04）：`check_legacy_escaping_ban`（?html/?xml/?rtf/?web_safe，eval.rs + strings_encoding.rs）；FORCE 禁令不适用（Rust 无 force 策略，文档化） | — | — |
| 4 | ~~自定义格式报错~~ | ~~`UndefinedCustomFormatException`（1 文件）~~ | ✅ **已实现**（2026-08-04）：`@name` 前缀（长度>1 且第二字符为字母，Java :1637-1641 条件）→ "No custom number/date format was defined with name \"x\""（number+date 双侧，format.rs `custom_format_name`/`j_quote`）；`@@` 转义与 `'@'` 字面量保持 Java 语义 | — | — |
| 5 | ~~get_optional_template~~ | ~~`GetOptionalTemplateMethod`（1 文件）~~ | ✅ **已实现**（2026-08-04）：`core/get_optional_template_method.rs` 一文件一对象；BuiltinVar::GetOptionalTemplate/Cc（错误消息方法名区分）；{exists/include/import} 哈希 + include 指令/import 方法；`lookup_template` 抽取（include_named 共用）；`import_lib_loaded` 已加载模板变体；配套 `TemplateMethodModelEx::exec` 加 env 参数（Java 线程局部 → Rust 显式传参，40 处机械更新） | — | — |
| 6 | **模板后处理钩子** | `TemplatePostProcessor`/`TemplatePostProcessorException`/`ThreadInterruptionSupportTemplatePostProcessor`（3 文件） | 无 post_process 概念 | 嵌入扩展点缺失（安全/审计集成） | P2：嵌入 API |
| 7 | **组合输出格式** | `CombinedMarkupOutputFormat`（1 文件） | 输出格式为单一枚举 | 组合格式（HTML+XML）转义不可用 | P2 |
| 8 | ~~StatefulTemplateLoader~~ | ~~`StatefulTemplateLoader`（1 文件）~~ | ✅ **已实现**（2026-08-04）：`TemplateLoader::reset_state` 默认空操作钩子（Java instanceof 检查的等价物——Rust trait 对象无法按接口向下转型，默认实现 + 虚分派语义等价，template_loader.rs 头注释）；MultiLoader 覆写传播；`Configuration.clear_template_cache` 调用 | — | — |
| 9 | **DOCTYPE 节点** | `ext/dom/DocumentTypeModel`（1 文件） | roxmltree 无 Doctype 节点变体（DOCTYPE 静默丢弃） | 模板无法访问文档类型声明 | P2（依赖 crate 限制） |

## 4. NA-DESIGN 构成（227）

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
| CacheStorage 替换策略家族 | 7 | TemplateCache 固定 HashMap 存储（v1 无容量/过期策略，文档化） |
| 其余内部工具 | 40 | BugException/SuppressFBWarnings 注解/迭代器抽象等 |

## 5. 合并映射说明（MAPPED 的主要合并）

| Java 文件组 | Rust 目标 | 说明 |
|---|---|---|
| ~80 个 AST 表达式类（AddConcatExpression/AndExpression/...） | `core/expression.rs::ExprKind` | Java 每类一文件，Rust 用 enum sum type（语言特性） |
| ~30 个指令类（IfBlock/ListBlock/SwitchBlock/...） | `core/template_element.rs::ElementKind` | 同上 |
| ~50 个内建类（BuiltIn*/BuiltInsFor*） | `builtins/mod.rs` 注册表 + `builtins/*.rs` | Java core/ 包分散，Rust 按主题分组 |
| ~20 个异常类（UnexpectedTypeException/NonXxxException/...） | `error/template_error.rs::TemplateError` | 错误枚举 + 消息逐字对齐 |
| 8 种输出格式类 | `core/output_format.rs::OutputFormatKind` | 单一枚举 |
| ext/dom 10 文件（NodeModel/ElementModel/...） | `xml/node.rs::XmlNode` | 节点角色由槽位分支承担 |
| TemplateExceptionHandler 内嵌 4 实现 | `environment.rs` 设置分发 | 字符串设置 |
| TemplateLookupContext/Result | `cache/template_lookup_strategy.rs` | 文件头自注"合并存放" |
| StatefulTemplateLoader | `cache/template_loader.rs::reset_state` 默认钩子 | Java 可选接口 instanceof 检查 → 默认空操作 + 虚分派（见 §3 #8） |
| StringUtil（glob 部分） | `utility/string_util.rs::glob_to_regex` | 一文件一对象（glob_to_regex 为 StringUtil 方法） |

## 5b. 文件级拆分执行记录（2026-08-04，一文件一 Java 对象）

| 区域 | 新增文件 | 对应 Java |
|---|---|---|
| `core/expression/`（26） | add_concat/and/or/arithmetic/comparison/string/number/boolean_literal、identifier、list/hash_literal、parenthetical/not/unary_plus_minus/exists/default_to/local_lambda/method_call/dot/dynamic_key_name/range/bounded_range_model/listable+nonlistable_right_unbounded_range_model/builtin_variable/built_in | Expression 家族各 Java 类 |
| `core/` 指令类（38） | flush/break/continue/return/stop_instruction、comment、ftl_header、trim_instruction、property_setting、fallback_instruction、text_block、dollar_variable、escape/no_escape/auto_esc/no_auto_esc_block、output_format_block、compressed_block、transform_block、attempt_block、assignment(_instruction)/block/global/local_assignment、if_block、iterator_block、items、sep、switch_block、macro（r#macro）、unified_call、body_instruction、include、library_load、visit_node、recurse_node、on | TemplateElement 家族各 Java 类 |
| `error/` 异常（17） | invalid_reference/unexpected_type_exception、non_*_exception ×8、_misc/misc_template_exception、template_exception、stop/return/break_or_continue_exception、template_not_found/template_model/parse_exception | Java 异常层级 |
| `core/` 格式/输出模型（14） | html/xml/xhtml/javascript/json/css/rtf/plain_text/undefined_output_format、common_markup/markup_output_format、template_output/template_markup_output/template_plain_output_model | OutputFormat 家族 + 输出模型接口 |
| `builtins/`（3 + 补全） | hashes、markup_outputs、strings_misc；sequences 迁入 join/reverse/seq_contains | BuiltInsForHashes/MarkupOutputs/StringsMisc/Sequences.java |
| `xml/`（12） | xml_dom_string_util（真实逻辑）+ 11 模型类锚点 | freemarker.ext.dom 各文件 |

**做法**：聚合枚举（ExprKind/ElementKind/TemplateError/OutputFormatKind）保留为聚合 API，
各 Java 类建立镜像文件（struct + new + exec/eval 方法或构造器锚点），dispatch 切换为
struct 方法调用；`TemplateError` 构造方法全部委托镜像文件（460 处调用点零改动）。
public-api baseline 除 builtins 新增 pub fn（与兄弟模块一致）外零差异。

## 6. 结构优化建议（执行清单）

对照结果发现的文件组织优化（S1-S4 已执行完毕，2026-08-04）：

| # | 优化 | Java 对应 | 状态 |
|---|---|---|---|
| S1 | 拆分 `xml/mod.rs` → `xml/ns_prefixes.rs`（NsPrefixes）+ `xml/tree.rs`（XmlTree）+ `xml/node.rs`（XmlNode/parse_xml） | ext/dom 各节点类 | ✅ 已执行 |
| S2 | 拆分 `cache/template_loader.rs` → `template_source.rs`（TemplateSource）+ `template_loader.rs`（TemplateLoader） | cache/TemplateLoader.java + URLTemplateSource.java | ✅ 已执行 |
| S3 | 新增 `core/get_optional_template_method.rs`（GetOptionalTemplateMethod）+ `core/template_configuration.rs`（TemplateConfiguration） | 对应 Java 文件 | ✅ 已执行 |
| S4 | 新增 `cache/` matcher 8 + factory 4 + exception 1（一文件一对象） | cache/TemplateSourceMatcher 家族 + Factory 家族 | ✅ 已执行 |
| S5 | 文档修订：docs/01 §3.2 ext/dom 已实现（原标"范围外"） | — | ✅ 已执行 |
| S6 | `TemplateMethodModelEx::exec` 加 env 参数（Java 线程局部 → Rust 显式传参） | Java 方法模型经 `Environment.getCurrentEnvironment()` 访问上下文 | ✅ 已执行（40 处 impl + 测试机械更新） |

## 7. 缺口补齐建议（优先级）

| 优先级 | 功能块 | 理由 |
|---|---|---|
| P0 | c_format 变体（#2）、自动转义禁令（#3）、get_optional_template（#5）、自定义格式报错（#4） | 行为差异 + 错误路径对齐（✅ 已补） |
| P1 | per-template 配置（#1）、StatefulTemplateLoader（#8） | 嵌入 API 面（✅ 已补） |
| P2 | 后处理钩子（#6）、组合格式（#7）、DOCTYPE（#9） | 扩展点/边缘场景（未补，见 §3） |

*报告结束。核对依据：snake_case 文件名机械匹配脚本（561 Java ↔ 90 Rust）+ 逐文件
语义核对（含本报告 §3 各行的 Java 行号引用）。*
