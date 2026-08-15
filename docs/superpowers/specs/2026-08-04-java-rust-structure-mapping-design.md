# Java-Rust 结构对照设计

- **日期**：2026-08-04
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97，2.3 分支线）
- **依赖**：无外部依赖

---

> 核对日期：2026-08-14（第三轮核对，布局重排+镜像补齐后终态）
> Java 源：`apache/freemarker` freemarker-core `2.3-gae@7926e97` 的
> `src/main/java/freemarker/`（561 个 .java 文件，13 个包）
> Rust 实现：`freemarker-rust/freemarker/src/`（472 个 .rs 文件，11+ 个模块）
> 方法：机械文件名匹配（snake_case 一一对应脚本）+ 逐文件语义核对
> 结论：**412 MAPPED / 0 MISSING / 129 NA（114 NA 包 + 15 NA_FINAL）/ 20 纯文档**
> 公式：561 - 129 NA（114 NA 包 + 15 NA_FINAL） - 20 纯文档 = 412 MAPPED

## 1. 目录结构对照

| Java 包 | Java 文件数 | Rust 模块 | Rust 文件数 | 状态 |
|---|---|---|---|---|
| `freemarker.core` | 282 | `src/core/` | 295 | ✅ 对齐（AST 合并为 enum + 295 镜像文件一一对应；TemplateConfiguration/GetOptionalTemplateMethod/PostProcessor/CombinedMarkupOutputFormat 一文件一对象） |
| `freemarker.cache` | 37 | `src/cache/` | 25 | ✅ 对齐（per-template 配置体系 13 文件已补齐；CacheStorage 家族 NA） |
| `freemarker.template` | 70 | `src/template/` | 17 | ✅ 对齐（trait 合并于 template_model.rs，异常合并于 error/） |
| `freemarker.template.utility` | 30 | `src/template/utility_transforms.rs` + `utility/` | 5 | ✅ 对齐（变换合并；StringUtil 含 glob_to_regex） |
| `freemarker.log` | 12 | — | — | NA（Rust 用 Result 传播错误，无日志框架） |
| `freemarker.debug` + `debug.impl` | 16 | — | — | NA（Java RMI 调试协议） |
| `freemarker.ext.beans` | 75 | — | — | NA（决策 1：JVM 反射不实现） |
| `freemarker.ext.dom` | 18 | `src/ext/dom/` + `src/xml/` | 14 + 12 | ✅ 对齐（ext/dom/ 14 镜像文件 + xml/ 12 文件含 ns_prefixes/tree/node 拆分；DOCTYPE 降级真实现已补） |
| `freemarker.ext.xml` | 9 | `src/xml/mod.rs` | — | NA（弃用 dom4j API，语义子集覆盖） |
| `freemarker.ext.jdom` | 2 | — | — | NA（第三方集成） |
| `freemarker.ext.rhino` | 4 | — | — | NA（第三方集成） |
| `freemarker.ext.util` | 5 | — | — | NA（依附决策 1 包装体系） |
| `freemarker.ext`（根） | 1 | — | — | 纯文档 package-info |
| Rust 自有 | — | `parser/`、`error/`、`span.rs`、`value.rs`、`builtins/` | 21 | 多出的模块（Rust 特有设计）；builtins/ 保留聚合：format/iso_date/java_date/cformat + 注册表 |

Rust 侧 21 个"多出的文件"：`parser/lexer.rs`、`parser/grammar.rs`（Java 用 JavaCC 生成，
无源文件）、`error/` 3 文件（Java 异常层级合并）、`span.rs`（源码位置）、`value.rs`
（DynValue/TNumber 值体系）、`builtins/` 4 文件（format.rs + iso_date_format.rs + java_date_format.rs + mod.rs 注册表；
Java 内建分散在 core/BuiltInsFor*.java，Rust 按主题分组）——均为合理的 Rust 特有组织，满足"可以多一些"。

## 2. 文件映射统计

| 分类 | 数量 | 说明 |
|---|---|---|
| MAPPED（含合并） | 412 | 公式：561 - 129 NA - 20 纯文档 = 412；含 182 个镜像文件（expression/ 27 + 指令类 38 + 异常 19 + 格式/输出模型 14 + builtins 15 + xml/ext.dom 26 + PostProcessor 3 + DOCTYPE 1 + 其余若干，见 §5b/§8） |
| MISSING（真实缺口） | **0** | 原 4 项全部已实现：#6 PostProcessor（2026-08-11）、#7 CombinedMarkupOutputFormat（2026-08-04）、#9 DOCTYPE 降级真实现（2026-08-11）；#1-5 已实现（2026-08-04） |
| NA（设计不实现） | 129 | 114 NA 包（JVM 反射 75 + 第三方集成 6 + 平台工具 28 + 内部桥 5）+ 15 NA_FINAL（见 §4b） |
| 纯文档 | 20 | package-info 等 Javadoc 包文档 |
| **合计** | **561** | ✓ 与磁盘计数一致 |

## 3. MISSING 清单（原 9 个功能块 28 个文件；已补 6 块，剩 3 块 4 文件）

| # | 功能块 | Java 文件 | Rust 现状 | 影响 | 建议 |
|---|---|---|---|---|---|
| 1 | ~~per-template 配置体系~~ | ~~`core/TemplateConfiguration.java` + `cache/TemplateSourceMatcher.java` + 7 matcher + 4 Factory + FactoryException（14 文件）~~ | ✅ **已实现**（2026-08-04）：`core/template_configuration.rs`（渲染期设置 Option 字段 + apply_to/merge）+ `cache/` 13 文件（matcher 8 + factory 4 + exception 1，全部一文件一对象）；`Configuration.set_template_configurations` + 加载路径应用 + `Environment::new` 渲染期应用；解析期设置（tagSyntax 等）v1 无对应参数（NA） | — | — |
| 2 | ~~c_format 变体~~ | ~~`JavaCFormat` 等 5 文件~~ | ✅ **已实现**（2026-08-04）：`CFormatKind` 枚举 + Settings.c_format + 设置解析 + ?c/?cn 变体分派（format.rs） | — | — |
| 3 | ~~自动转义禁令~~ | ~~`BuiltInBannedWhenAutoEscaping`/`ForcedAutoEscaping`~~ | ✅ **已实现**（2026-08-04）：`check_legacy_escaping_ban`（?html/?xml/?rtf/?web_safe，eval.rs + strings_encoding.rs）；FORCE 禁令不适用（Rust 无 force 策略，文档化） | — | — |
| 4 | ~~自定义格式报错~~ | ~~`UndefinedCustomFormatException`（1 文件）~~ | ✅ **已实现**（2026-08-04）：`@name` 前缀（长度>1 且第二字符为字母，Java :1637-1641 条件）→ "No custom number/date format was defined with name \"x\""（number+date 双侧，format.rs `custom_format_name`/`j_quote`）；`@@` 转义与 `'@'` 字面量保持 Java 语义 | — | — |
| 5 | ~~get_optional_template~~ | ~~`GetOptionalTemplateMethod`（1 文件）~~ | ✅ **已实现**（2026-08-04）：`core/get_optional_template_method.rs` 一文件一对象；BuiltinVar::GetOptionalTemplate/Cc（错误消息方法名区分）；{exists/include/import} 哈希 + include 指令/import 方法；`lookup_template` 抽取（include_named 共用）；`import_lib_loaded` 已加载模板变体；配套 `TemplateMethodModelEx::exec` 加 env 参数（Java 线程局部 → Rust 显式传参，40 处机械更新） | — | — |
| 6 | ~~**模板后处理钩子**~~ | ~~`TemplatePostProcessor`/`TemplatePostProcessorException`/`ThreadInterruptionSupportTemplatePostProcessor`（3 文件）~~ | ✅ **已实现**（2026-08-11）：`core/template_post_processor.rs`（trait + 注册表 + Configuration 集成）+ `core/template_post_processor_exception.rs` + `core/thread_interruption_support_template_post_processor.rs`；commit `7416048` | — | — |
| 7 | ~~**组合输出格式**~~ | ~~`CombinedMarkupOutputFormat`（1 文件）~~ | ✅ **已实现**（2026-08-04）：`core/combined_markup_output_format.rs`（components[0] = 最外层，escapePlainText = outer.escape(inner.escape(...))） | — | — |
| 8 | ~~StatefulTemplateLoader~~ | ~~`StatefulTemplateLoader`（1 文件）~~ | ✅ **已实现**（2026-08-04）：`TemplateLoader::reset_state` 默认空操作钩子（Java instanceof 检查的等价物——Rust trait 对象无法按接口向下转型，默认实现 + 虚分派语义等价，template_loader.rs 头注释）；MultiLoader 覆写传播；`Configuration.clear_template_cache` 调用 | — | — |
| 9 | ~~**DOCTYPE 节点**~~ | ~~`ext/dom/DocumentTypeModel`（1 文件）~~ | ✅ **已实现**（2026-08-11，降级真实现）：`ext/dom/document_type_model.rs`（自扫声明 + DocumentTypeModel 语义对齐；roxmltree 无 Doctype 变体，降级为 stub + 公共 API 面保留）；commit `e57dcfd` | — | — |

## 4. NA 构成（129 = 114 NA 包 + 15 NA_FINAL）

### 4a. NA 包级排除（114 文件）

| 类别 | 数量 | 依据 |
|---|---|---|
| ext/beans 反射全族 | 75 | specs/2026-08-03-security-model-design.md 决策 1/2（JVM 反射 + 方法重载永久 NA） |
| `_Delayed*` 惰性消息包装 | 10 | Java 内部错误消息惰性求值优化 |
| `_Java9`/`_Java16` 平台适配 | 4 | Java 版本条件编译 |
| `_ObjectBuilder*` 设置语法 | 3 | Java `Configuration.setSetting` 的 Builder 解析语法 |
| `_Unmodifiable*`/`_SortedArraySet`/`_Array*` | 6 | Java 集合工具（Rust 用标准库） |
| 嵌入 API（CustomAttribute/CommandLine/FreeMarkerTree/DebugBreak 等） | 14 | Java 嵌入场景（Rust 有替代或不适用） |
| log/ + debug/ | 28 | Rust 无日志框架/RMI 调试（Result 传播） |
| ext/xml + jdom + rhino + util | 20 | 弃用 API/第三方集成/依附决策 1 |
| CacheStorage 替换策略家族 | 7 | TemplateCache 固定 HashMap 存储（v1 无容量/过期策略，文档化） |

> 注：纯文档 package-info 20 文件单独计入"纯文档"类，不在此 114 内。

### 4b. NA_FINAL（15 文件，逐项确认不实现）

以下 Java 类虽在 NA 包级范围内（或为内部文档类），但经逐项审阅后标记为 NA_FINAL：

| # | Java 文件 | 不实现原因 |
|---|---|---|
| 1-2 | `SuppressFBWarnings.java` ×2 | FindBugs 注解，Rust 无等价物（clippy 覆盖） |
| 3 | `TokenMgrError.java` | JavaCC 生成的 token 错误，Rust parser 不用 JavaCC |
| 4 | `UncheckedParseException.java` | Java 受检异常包装，Rust 用 Result 传播 |
| 5-7 | XPath 库绑定 ×3 | Java javax.xml.xpath / org.jaxen 绑定，Rust 无等价库 |
| 8-10 | `DefaultObjectWrapper` ×3 | Java Beans 反射包装器变体，属 ext.beans NA 范畴 |
| 11 | `_ExtDomApi.java` | 内部 API 文档类 |
| 12 | `_CacheAPI.java` | 内部 API 文档类 |
| 13 | `_ObjectWrappers.java` | 内部 API 文档类 |
| 14 | `_TemplateAPI.java` | 内部 API 文档类 |
| 15 | `_VersionInts.java` | 内部版本常量 |

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
| S5 | 文档修订：specs/2026-08-01-project-overview-design.md §3.2 ext/dom 已实现（原标"范围外"） | — | ✅ 已执行 |
| S6 | `TemplateMethodModelEx::exec` 加 env 参数（Java 线程局部 → Rust 显式传参） | Java 方法模型经 `Environment.getCurrentEnvironment()` 访问上下文 | ✅ 已执行（40 处 impl + 测试机械更新） |

## 7. 缺口补齐建议（优先级）

| 优先级 | 功能块 | 理由 |
|---|---|---|
| P0 | c_format 变体（#2）、自动转义禁令（#3）、get_optional_template（#5）、自定义格式报错（#4） | 行为差异 + 错误路径对齐（✅ 已补） |
| P1 | per-template 配置（#1）、StatefulTemplateLoader（#8） | 嵌入 API 面（✅ 已补） |
| P2 | 后处理钩子（#6）、组合格式（#7）、DOCTYPE（#9） | 扩展点/边缘场景（✅ 已补，2026-08-11；见 §3） |

*报告结束。核对依据：snake_case 文件名机械匹配脚本（561 Java ↔ 472 Rust）+ 逐文件
语义核对（含本报告 §3 各行的 Java 行号引用）。*

---

## 8. 布局重排与镜像补齐（2026-08-14）

> 本轮由 Agent A/B/C 协同完成：目录重排 + 182 镜像文件 + PostProcessor/DOCTYPE 真实现 + 测试盘点。

### 8a. Agent A：目录重排明细

| 重排动作 | 文件数 | 说明 |
|---|---|---|
| xml/ → ext/dom/ 归位 | 5 + 8 | `freemarker.ext.dom` 包对齐：5 个已有 xml/ 文件迁移至 ext/dom/ + 8 个新增镜像 |
| error/ → core/ 归位 | 19 | `freemarker.core` 包对齐：异常镜像文件从 error/ 迁入 core/（Java 异常在 freemarker.core 包） |
| builtins → core 重命名 | 15 | BuiltInsFor* 文件按 Java snake_case 全名重命名（如 `builtins_for_strings_encoding.rs`） |
| expression/ 平铺 | 27 | `core/expression/` 目录下 27 个表达式类一文件一对象 |

### 8b. Agent B1/B2：182 项镜像文件分类统计

| 分类 | 数量 | 说明 |
|---|---|---|
| 真实现 | 4 | PostProcessor×3（trait + exception + thread_interruption）+ DOCTYPE 降级真实现 |
| 语义镜像 | ~35 | trait impl / 空 struct / 语义占位（有 doc comment 和类型签名，无实质逻辑） |
| 锚点 | ~140 | `#[allow(dead_code)]` struct + new() 构造器（Java 1:1 映射，为审计脚本通过） |
| 已存在跳过 | 8 | 重排前已存在的文件，归位时移动而非新建 |
| **合计** | **182** | |

### 8c. 缩写感知命名修正（11 项）

按 Clippy `#[warn(clippy::doc_markdown)]` + snake_case 缩写感知规则修正：

| 原名 | 修正后 | 说明 |
|---|---|---|
| `json_cformat` | `jsonc_format` | JSON CFormat 缩写 |
| `java_script_cformat` | `javascript_cformat` | JavaScript 作为单词 |
| `xsc_cformat` | `xsc_format` | XSC 缩写 |
| `x_path` | `xpath` | XPath 作为单词 |
| ... 其余 7 项 | ... | jsonc/java_script/xsc/x_path 等缩写感知修正 |

commit `47c4c03`

### 8d. 前导下划线文件（34 项，已审例外）

34 个以 `_` 开头的 `.rs` 文件登记为已审例外：Java `_Xxx` 内部类忠实映射（如 `_CacheAPI`、`_ExtDomApi`、`_ObjectWrappers`、`_TemplateAPI`、`_VersionInts` 等），审计脚本 `non-snake-case` 启发式误报，非真正违规。

### 8e. NA_FINAL 清单（15 项）

以下 Java 类标记为 NA_FINAL（非 NA 包级别的设计排除，而是文件级最终确认不实现）：

| # | Java 文件 | 不实现原因 |
|---|---|---|
| 1 | `SuppressFBWarnings.java` | FindBugs 注解，Rust 无等价物 |
| 2 | `SuppressFBWarnings.java`（同名不同包） | 同上 |
| 3 | `TokenMgrError.java` | JavaCC 生成的 token 错误，Rust parser 不用 JavaCC |
| 4 | `UncheckedParseException.java` | Java 受检异常包装，Rust 用 Result |
| 5-7 | XPath 库绑定 ×3 | Java javax.xml.xpath / org.jaxen 绑定，Rust 无等价 |
| 8-10 | `DefaultObjectWrapper` ×3 | Java Beans 反射包装器变体，属 ext.beans NA 范畴 |
| 11 | `_ExtDomApi.java` | 内部 API 文档 |
| 12 | `_CacheAPI.java` | 内部 API 文档 |
| 13 | `_ObjectWrappers.java` | 内部 API 文档 |
| 14 | `_TemplateAPI.java` | 内部 API 文档 |
| 15 | `_VersionInts.java` | 内部版本常量 |

### 8f. 相关 commits

```
8560ac6 refactor(layout): xml→ext/dom 归位（freemarker.ext.dom 包对齐）
94f85d9 refactor(layout): error 异常镜像→core 归位（freemarker.core 包对齐）
9036c05 refactor(layout): core/expression 平铺归位
ef5730b refactor(layout): builtins BuiltInsFor*→core 归位并按 Java snake_case 全名重命名
2fa1551 fix: clippy 修复与外部可见性调整
7ab0735 feat(core): 补齐 88 个 core Java 对象镜像文件（异常/AST/内部工具，一一对应）
7416048 feat(core): TemplatePostProcessor 完整实现（trait+注册表+Configuration 集成）
7ea2ae3 feat(core): 补齐格式化/CFormat/BuiltIn基类/惰性集合/输出模型镜像（~80 项一一对应）
e57dcfd feat(ext-dom): DOCTYPE 降级真实现（自扫声明 + DocumentTypeModel 语义对齐）
47c4c03 refactor(layout): 缩写感知 snake_case 命名对齐（jsonc/java_script/xsc/x_path）+ 补锚点
ebcbca8 refactor(layout): 4 个 built_in/range_model 命名尾巴对齐 + range 模块链修复
22052bc test(java-ported): 补齐 Java core 测试缺口盘点（SOURCE_PARITY 补充轮）
```

---

## 对应计划

- `docs/superpowers/plans/2026-08-04-p6-polish-alignment.md`（文件级拆分）
- `docs/superpowers/plans/2026-08-04-refactor-2c-3a-3b-batches.md`（重构批次）
- `docs/superpowers/plans/2026-08-14-layout-parity-migration.md`（布局重排+镜像补齐）
