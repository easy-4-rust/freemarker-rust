# P1-P4 核心实现 — freemarker-rust 主体开发计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]` / `- [ ]` / `- [~]`) syntax for tracking.

**Goal:** 实现 FreeMarker 核心引擎的四个主要子系统——解析器（P1）、渲染引擎与基础指令（P2）、表达式与内建函数全量（P3）、配置/缓存/格式化（P4）。

**Architecture:** 手写递归下降解析器（5 状态词法 + 24 表达式产生式 + 13 指令产生式）→ Environment 指令栈渲染循环 → 183 内建函数注册表 → Configurable 设置继承链 + TemplateCache + OutputFormat 家族。

**Tech Stack:**
- 手写递归下降（不引 pest/nom）
- fancy-regex（正则环视/反向引用）
- bigdecimal（BigDecimalEngine 算术）
- chrono（日期格式化）
- roxmltree（XML 解析）

**Related Design Doc:** `docs/superpowers/specs/2026-08-01-parser-design.md`、`docs/superpowers/specs/2026-08-01-rendering-engine-design.md`、`docs/superpowers/specs/2026-08-02-builtins-design.md`、`docs/superpowers/specs/2026-08-01-config-cache-design.md`、`docs/superpowers/specs/2026-08-01-formatting-design.md`

---

## 全局约定

- **解析器**：手写递归下降，对应 FTL.jj 全部产生式（4,845 行 JavaCC 源）
- **渲染循环**：`while let Some(el) = stack.pop() { visit(el)? }` accept 模式
- **内建注册**：`builtins/mod.rs` 按名称查找 + 参数解析，183 个 BI 全量
- **设置继承**：`Option<T>` 表达"未设置"，父链向上查找
- **错误消息**：逐字对齐 Java 基线（`error/expected_messages/` 70 场景）

---

## 实施阶段总览

| Stage | 目标 | Task 数 |
|-------|------|---------|
| 1 | P1 词法器（5 状态 + 全 token） | 2 |
| 2 | P1 解析器（表达式 + 指令产生式） | 3 |
| 3 | P2 Environment 渲染引擎 | 3 |
| 4 | P2 基础指令 + 数据模型 | 4 |
| 5 | P3 表达式补全 + 内建函数全量 | 5 |
| 6 | P4 配置/缓存/格式化 | 5 |
| 7 | P4 错误消息对齐 + L3 接入 | 2 |

---

## Stage 1 — P1 词法器

### Task 1.1：词法器核心（5 状态）

**Files:**
- Create: `freemarker/src/parser/lexer.rs`（1,659 行，多状态词法扫描器）
- Create: `freemarker/src/parser/mod.rs`

- [x] **Step 1:** 实现 5 个词法状态（DEFAULT/IN_PAREN/NO_SPACE_EXPRESSION/NAMED_PARAMETER_EXPRESSION/NO_PARSE）
- [x] **Step 2:** 实现全部 token 类型（结构/指令/运算符/字面量/BUILT_IN/COMMENT 等）
- [x] **Step 3:** 实现字符串插值解析 + 转义规则
- [x] **Step 4:** Commit

---

### Task 1.2：词法测试 + 错误位置

**Files:**
- Modify: `freemarker/src/parser/lexer.rs`（测试模块）

- [x] **Step 1:** token 级快照测试（词法状态切换）
- [x] **Step 2:** 错误位置断言（行/列/期望清单）
- [x] **Step 3:** Commit

---

## Stage 2 — P1 解析器

### Task 2.1：表达式产生式（24 个）

**Files:**
- Create: `freemarker/src/parser/grammar.rs`（6,837 行，递归下降产生式）
- Create: `freemarker/src/core/expression.rs`（ExprKind enum）
- Create: `freemarker/src/core/expression/` 目录（26 个镜像文件）

- [x] **Step 1:** 实现 24 个表达式产生式（算术/比较/逻辑/标识符/字符串/列表/哈希/范围/lambda/方法调用/dot/dynamic key/built-in 链/default-to/exists/parenthetical/not/unary）
- [x] **Step 2:** AST ExprKind enum sum type
- [x] **Step 3:** Commit — `refactor: core AST 拆分首批`

---

### Task 2.2：指令产生式（13 个）

**Files:**
- Modify: `freemarker/src/parser/grammar.rs`
- Create: `freemarker/src/core/template_element.rs`（ElementKind enum）
- Create: `freemarker/src/core/` 指令类镜像文件（38 个）

- [x] **Step 1:** 实现 13 个指令产生式 + FreemarkerDirective 内部分发
- [x] **Step 2:** ElementKind enum + 指令类拆分（if/list/assign/macro/function/include/visit/switch/attempt 等）
- [x] **Step 3:** Commit — `refactor: core 指令类拆分`

---

### Task 2.3：解析错误消息对齐

**Files:**
- Create: `freemarker/src/error/expected_messages/`（70 场景基线）

- [x] **Step 1:** 解析错误消息逐字对齐（`Parsing error in template...` 结构）
- [x] **Step 2:** 行/列/期望清单格式对齐
- [x] **Step 3:** Commit

---

## Stage 3 — P2 Environment 渲染引擎

### Task 3.1：Environment 核心（指令栈 + 作用域）

**Files:**
- Create: `freemarker/src/core/environment.rs`（3,357 行，渲染引擎核心）

- [x] **Step 1:** 实现指令栈（`Vec<Element>` + `process/visit/visit_many/replace_top`）
- [x] **Step 2:** 实现变量解析链（局部 -> 命名空间 -> 全局 -> 数据模型）
- [x] **Step 3:** 实现 `LocalContextStack`（循环变量/宏参数作用域链）
- [x] **Step 4:** 实现 `InvalidReferenceException` 语义
- [x] **Step 5:** Commit

---

### Task 3.2：Namespace + 上下文

**Files:**
- Create: `freemarker/src/core/configurable.rs`
- Create: `freemarker/src/core/exec.rs`（1,231 行，指令执行）
- Create: `freemarker/src/core/eval.rs`（1,218 行，表达式求值）

- [x] **Step 1:** 实现 `Namespace` / `LazilyInitializedNamespace`
- [x] **Step 2:** 实现 `LocalContextStack` + 作用域进出
- [x] **Step 3:** Commit

---

### Task 3.3：异常处理 + include

**Files:**
- Modify: `freemarker/src/core/environment.rs`
- Create: `freemarker/src/core/include.rs`

- [x] **Step 1:** 实现 `handle_error`（attempt 恢复/RETHROW/DEBUG 模式）
- [x] **Step 2:** 实现 `include`（模板加载与执行，带 locale/encoding/condition）
- [x] **Step 3:** Commit

---

## Stage 4 — P2 基础指令 + 数据模型

### Task 4.1：基础指令全量

**Files:**
- Create: `freemarker/src/core/` 指令实现文件（text/interpolation/if/list/assign/macro/function/nested/call/switch/attempt/recover/break/continue/return/stop/flush/trim/comment/setting/compress/escape/noescape/autoesc/noautoesc/outputformat）

- [x] **Step 1:** 实现 text/interpolation 基础指令
- [x] **Step 2:** 实现 if/list/assign 指令
- [x] **Step 3:** 实现 macro/function/nested/call 指令
- [x] **Step 4:** 实现 switch/attempt/recover/flow-control 指令
- [x] **Step 5:** Commit

---

### Task 4.2：基础表达式求值

**Files:**
- Modify: `freemarker/src/core/eval.rs`
- Modify: `freemarker/src/core/expression.rs`

- [x] **Step 1:** 实现算术/比较/逻辑/括号求值
- [x] **Step 2:** 实现标识符/字符串/列表/哈希/范围求值
- [x] **Step 3:** Commit

---

### Task 4.3：数据模型最小集

**Files:**
- Create: `freemarker/src/template/simple_scalar.rs`
- Create: `freemarker/src/template/simple_number.rs`
- Create: `freemarker/src/template/simple_boolean.rs`
- Create: `freemarker/src/template/simple_date.rs`
- Create: `freemarker/src/template/simple_hash.rs`
- Create: `freemarker/src/template/simple_list.rs`
- Create: `freemarker/src/template/simple_sequence.rs`
- Create: `freemarker/src/template/simple_collection.rs`

- [x] **Step 1:** 实现 SimpleScalar/Number/Boolean/Date
- [x] **Step 2:** 实现 SimpleHash/List/Sequence/Collection
- [x] **Step 3:** Commit

---

### Task 4.4：SimpleObjectWrapper + TemplateCache 最小版

**Files:**
- Create: `freemarker/src/template/simple_object_wrapper.rs`
- Create: `freemarker/src/cache/template_cache.rs`
- Create: `freemarker/src/cache/string_template_loader.rs`

- [x] **Step 1:** 实现 `SimpleObjectWrapper`（Rust 值 -> TModel）+ `DeepUnwrap`
- [x] **Step 2:** 实现 `StringLoader` + `TemplateCache` 最小版
- [x] **Step 3:** Commit

---

## Stage 5 — P3 表达式补全 + 内建函数全量

### Task 5.1：表达式补全

**Files:**
- Modify: `freemarker/src/core/expression.rs` + expression/ 子目录
- Create: `freemarker/src/core/arithmetic_engine.rs`

- [x] **Step 1:** 补全 DefaultTo/Exists/BuiltIn 链/MethodCall/Dot/DynamicKey/Lambda/NewBI/`+` concat 语义
- [x] **Step 2:** 实现 `BigDecimalEngine`（对照 ArithmeticEngine.java 逐行）+ `OptimizerUtil`
- [x] **Step 3:** Commit

---

### Task 5.2：内建函数——字符串族（31 个）

**Files:**
- Create: `freemarker/src/builtins/strings.rs`（1,075 行）
- Create: `freemarker/src/builtins/strings_encoding.rs`
- Create: `freemarker/src/builtins/strings_misc.rs`
- Create: `freemarker/src/builtins/strings_regexp.rs`

- [x] **Step 1:** 实现 BuiltInsForStringsBasic（31 个：cap_first/capitalize/lower_case/upper_case/contains/starts_with/ends_with/index_of/keep_after/keep_before/length/pad/split_/substring/trim/truncate/uncap_first/word_list 等）
- [x] **Step 2:** 实现字符串编码族（html/xhtml/xml/rtf/url/url_path/j_string/js_string/json_string/replace）
- [x] **Step 3:** 实现正则族（matches/replace_with_regexp/groups/wildcard_matches）
- [x] **Step 4:** UTF-16 语义专项测试
- [x] **Step 5:** Commit

---

### Task 5.3：内建函数——数字/日期/序列/哈希/节点/其他

**Files:**
- Create: `freemarker/src/builtins/numbers.rs`
- Create: `freemarker/src/builtins/dates.rs`
- Create: `freemarker/src/builtins/sequences.rs`（1,777 行）
- Create: `freemarker/src/builtins/hashes.rs`
- Create: `freemarker/src/builtins/nodes.rs`
- Create: `freemarker/src/builtins/existence.rs`
- Create: `freemarker/src/builtins/callables.rs`
- Create: `freemarker/src/builtins/lazy.rs`
- Create: `freemarker/src/builtins/loop_vars.rs`
- Create: `freemarker/src/builtins/multi.rs`
- Create: `freemarker/src/builtins/markup_outputs.rs`
- Create: `freemarker/src/builtins/format.rs`
- Create: `freemarker/src/builtins/iso_date_format.rs`
- Create: `freemarker/src/builtins/java_date_format.rs`
- Create: `freemarker/src/builtins/mod.rs`（注册表）

- [x] **Step 1:** 实现数字族（abs/round/floor/ceiling/c/is_infinite/is_nan/is_nan_or_infinite/byte/double/float/int/long/short/big_decimal/number_to_date/is_date_like）
- [x] **Step 2:** 实现日期族（date/time/datetime/iso_*_hz/java_*_hz/date_if_unknown/time_if_unknown/datetime_if_unknown）
- [x] **Step 3:** 实现序列族（first/last/reverse/sort/sort_by/seq_contains/seq_index_of/seq_last_index_of/chunk/join/min/max/filter/map/flatMap/group_by/count/weak_groups/take_while/drop_while/some/every/none/interleave/zip/subsequence/index_of）
- [x] **Step 4:** 实现哈希族（keys/values/size/contains_key/values/merge/aggregate)
- [x] **Step 5:** 实现节点族（node_name/node_namespace/parent/children/root/ancestors/node_type/is_node/copy/next_sibling/previous_sibling）
- [x] **Step 6:** 实现其他族（exists/is_*/has_*/api/has_api/new/web_safe/eval/is_string/is_number/is_boolean/is_date/is_method/is_sequence/is_hash/is_collection/is_enumerable/is_indexable/is_directive/is_transform/is_macro/is_hash_ex/is_markup_output/is_string_like/is_number_like/is_boolean_like/is_date_like/is_enumerable/is_sequence_or_collection）
- [x] **Step 7:** 实现 format/iso_date/java_date 格式化内建
- [x] **Step 8:** 183 BI 全注册（编译期清单核对）
- [x] **Step 9:** Commit

---

### Task 5.4：正则适配 + CFormat 最小集

**Files:**
- Create: `freemarker/src/regexp.rs`
- Create: `freemarker/src/core/cformat.rs`（CFormatKind 枚举）

- [x] **Step 1:** `fancy-regex` 适配（反向引用/环视）+ `RegexpHelper` 等价
- [x] **Step 2:** 实现 `?string/?c/?cn` 基础（CFormat 最小集：Legacy）
- [x] **Step 3:** Commit

---

### Task 5.5：循环变量内建

**Files:**
- Modify: `freemarker/src/builtins/loop_vars.rs`

- [x] **Step 1:** 实现循环变量内建（counter/index/has_next/item_cycle...）与 `LocalContext` 联动
- [x] **Step 2:** Commit

---

## Stage 6 — P4 配置/缓存/格式化

### Task 6.1：Configurable 设置项全表

**Files:**
- Create: `freemarker/src/core/configurable.rs`（317 行）
- Modify: `freemarker/src/template/configuration.rs`

- [x] **Step 1:** 实现 `Settings` 结构体（全部设置项 + `Option<T>` 继承语义）
- [x] **Step 2:** 实现设置继承合并 + 字符串设置 API
- [x] **Step 3:** Commit

---

### Task 6.2：TemplateConfiguration + matcher 链

**Files:**
- Create: `freemarker/src/core/template_configuration.rs`
- Create: `freemarker/src/cache/template_source_matcher.rs` 及 8 个 matcher 文件
- Create: `freemarker/src/cache/template_configuration_factory.rs` 及 4 个 factory 文件
- Create: `freemarker/src/cache/template_configuration_factory_exception.rs`

- [x] **Step 1:** 实现 `TemplateConfiguration`（渲染期设置 Option 字段 + apply_to/merge）
- [x] **Step 2:** 实现 matcher 链（FirstMatch/Merging/Conditional + 5 类 matcher：And/Or/Not/FileExtension/FileNameGlob/PathGlob/PathRegex/TemplateSource）
- [x] **Step 3:** 实现 Configuration.set_template_configurations + 加载路径应用
- [x] **Step 4:** Commit — `refactor: cache/ matcher + factory 一文件一对象`

---

### Task 6.3：TemplateLoader 全家族

**Files:**
- Create: `freemarker/src/cache/file_template_loader.rs`
- Create: `freemarker/src/cache/class_template_loader.rs`
- `freemarker/src/cache/url_template_loader.rs`
- Create: `freemarker/src/cache/byte_array_template_loader.rs`
- Create: `freemarker/src/cache/multi_template_loader.rs`
- Create: `freemarker/src/cache/template_loader.rs`
- Create: `freemarker/src/cache/template_source.rs`
- Create: `freemarker/src/cache/stateful_template_loader.rs`

- [x] **Step 1:** 实现 File/String/Class/URL/ByteArray/Multi loader
- [x] **Step 2:** 实现粘性 loader + StatefulTemplateLoader（reset_state 钩子）
- [x] **Step 3:** Commit

---

### Task 6.4：TemplateCache 完整 + CacheStorage

**Files:**
- Modify: `freemarker/src/cache/template_cache.rs`
- Create: `freemarker/src/cache/cache_storage.rs`
- Create: `freemarker/src/cache/strong_cache_storage.rs`
- Create: `freemarker/src/cache/mru_cache_storage.rs`
- Create: `freemarker/src/cache/soft_cache_storage.rs`
- Create: `freemarker/src/cache/null_cache_storage.rs`
- Create: `freemarker/src/cache/template_lookup_strategy.rs`
- Create: `freemarker/src/cache/template_lookup_context.rs`
- Create: `freemarker/src/cache/template_lookup_result.rs`

- [x] **Step 1:** 实现 TemplateCache 完整版（TemplateKey/delay/负查找/localizedLookup/acquisition/名称规范化）
- [x] **Step 2:** 实现 CacheStorage：Strong/MRU（Weak 软段）
- [x] **Step 3:** Commit

---

### Task 6.5：OutputFormat 家族 + CFormat 五种

**Files:**
- Create: `freemarker/src/core/output_format.rs`（OutputFormatKind enum）
- Create: `freemarker/src/core/html_output_format.rs` 等 10 个输出格式文件
- Create: `freemarker/src/core/combined_markup_output_format.rs`
- Create: `freemarker/src/core/common_markup_output_format.rs`
- Modify: `freemarker/src/core/cformat.rs`

- [x] **Step 1:** 实现 OutputFormat 全家族（HTML/XML/XHTML/JavaScript/JSON/CSS/RTF/PlainText/Undefined）
- [x] **Step 2:** 实现 `?esc/?no_esc` + autoEscaping 矩阵
- [x] **Step 3:** 实现 CFormat 五种（Legacy/JSON/Java/JS/JSOrJSON/XSC）+ `?cn`
- [x] **Step 4:** Commit

---

## Stage 7 — P4 错误消息对齐 + L3 接入

### Task 7.1：错误消息基线

**Files:**
- Modify: `freemarker/src/error/error_ctx.rs`
- Modify: `freemarker/src/error/template_error.rs`

- [x] **Step 1:** 错误消息逐字对齐（Java 基线全量 diff）
- [x] **Step 2:** 指令栈转储格式对齐
- [x] **Step 3:** Commit

---

### Task 7.2：L3 对比 harness 接入

**Files:**
- Modify: `freemarker-test/tests/golden.rs`
- Modify: `scripts/`

- [x] **Step 1:** 接入 L3 对比 harness（Java 侧跑控制流用例 -> Rust diff）
- [x] **Step 2:** 首批 golden 套件通过
- [x] **Step 3:** Commit

---

## 实际完成状态

- **日期**：2026-08-01 ~ 2026-08-02
- **关键里程碑**：
  - 2026-08-02：P4 + 内建函数 + L3 harness + CI 完成
  - BLOCKED 35 -> 20 -> 5 -> 0
  - golden 套件 71 PASS（2026-08-02）-> 82 PASS
- **验收**：`cargo test --workspace` 通过；golden 套件可运行
