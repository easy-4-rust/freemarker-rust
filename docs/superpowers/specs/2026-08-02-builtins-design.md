# 内建函数迁移设计

- **日期**：2026-08-02
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（183 个 BI 静态类，30 个源文件）
- **依赖**：无外部依赖

---

> 源：`freemarker-core/src/main/java/freemarker/core/BuiltInsFor*.java`（30 文件，**183 个 BI 静态类**——Java 2.3.34 全集；Rust 已 183/183 全部注册，含最后补齐的 eval_json/is_date_like/next_sibling/previous_sibling/web_safe）
> 架构：`BuiltIn` 基类 + 特化基类 `BuiltInForString/BuiltInForNumber/BuiltInForSequence/BuiltInForHashEx/BuiltInForDate/BuiltInForNode/BuiltInForMarkupOutput/BuiltInForLoopVariable` + 特殊基类 `SpecialBuiltIn/BuiltInWithDirectCallOptimization/BuiltInWithParseTimeParameters/BuiltInBannedWhenAutoEscaping/BuiltInBannedWhenForcedAutoEscaping/BuiltInForLegacyEscaping/BuiltinVariable`
> Rust 实现：每文件一个模块 + 注册表 `builtins/mod.rs`（`&'static str → BuiltInKind`，按名称查找 + 参数解析）

## 1. 完整清单（按源文件分组，183 个——Java 2.3.34 全集）

### 1.1 BuiltInsForStringsBasic（31）—— `builtins/strings.rs`

| BI | 语义要点（对照 Java 逐项复刻） |
|---|---|
| `cap_first` | 首字母大写（Unicode 感知） |
| `capitalize` | 每词首字母大写（按空白分词） |
| `c_lower_case` / `c_upper_case` | 仅 ASCII 的大小写转换（`c_` 前缀 = classic） |
| `lower_case` / `upper_case` | Unicode 大小写 |
| `chop_linebreak` | 去掉尾部单个换行 |
| `contains` / `starts_with` / `ends_with` | 子串判断 |
| `ensure_ends_with` / `ensure_starts_with` | 缺失则补缀 |
| `index_of` / `keep_after` / `keep_after_last` / `keep_before` / `keep_before_last` | 子串定位与截取（`index_of` 支持 fromIndex 参数） |
| `remove_beginning` / `remove_ending` | 精确前缀/后缀移除（`remove_ending` 只在整串以该串结束时生效） |
| `length` | 字符数（UTF-16 code unit vs 码点 —— **注意 Java 用 UTF-16**，Rust `chars().count()` 是码点，**须对齐 Java 语义：按 `char` 计**） |
| `pad` | `pad(length, padString)` 左/右补（长度按 UTF-16 计） |
| `split_` | 分隔符切分（返回序列；支持限制参数） |
| `substring` | `substring(from[, to])`（UTF-16 索引） |
| `trim` | 首尾空白裁剪（Java `String.trim` 语义 = ≤U+0020，**非 Unicode 空白**） |
| `truncate` / `truncate_w` / `truncate_c` / `truncate_m` / `truncate_w_m` / `truncate_c_m` | 截断（长度+省略号+word/char 边界；`TruncateBuiltinAlgorithm` 可插拔，默认 `DefaultTruncateBuiltinAlgorithm`） |
| `uncap_first` | 首字母小写 |
| `word_list` | 按空白分词为序列 |

### 1.2 BuiltInsForStringsEncoding（8）—— `builtins/strings_encoding.rs`

| BI | 语义要点 |
|---|---|
| `html` | HTML 转义（`& < > " '` + 非 ASCII 按需 `&#x...;`，**由 StringUtil.HTMLEnc** 逐字复刻） |
| `xml` | XML 转义（`& < > " '`） |
| `xhtml` | 同 html（占位兼容） |
| `js_string` | JS 字符串转义（含 `\uXXXX` 非 ASCII —— 与 `j_string` 区别：JS 规则） |
| `j_string` | Java 字符串转义（`StringUtil.javaStringEnc`，非 ASCII 转 `\uXXXX`） |
| `json_string` | JSON 转义（`\uXXXX`，控制字符映射） |
| `url` | URL 编码（`URLEscapingCharset` 设置，`+` vs `%20` 规则，`StringUtil.URLEnc` 复刻） |
| `rtf` | RTF 转义（`\`、`{`、`}`、非 ASCII `\uN?`） |

### 1.3 BuiltInsForStringsRegexp（3）—— `builtins/strings_regexp.rs`

| BI | 语义要点 |
|---|---|
| `matches` | 返回匹配信息序列（匹配区间） |
| `groups` | 取正则捕获组（`?groups(1)`；未匹配返回错误 —— `NonNumericalException`/`InvalidReferenceException` 分支） |
| `replace_re` | 正则替换（支持 `${group}` 引用 + lambda 替换函数） |

> **正则差异风险**：Java `java.util.regex` vs Rust `regex` crate —— 字符类、`\w`/`\d` Unicode 属性、反向引用（Rust regex 默认不支持 backreference！`replace_re` 的 `$1` 是替换语法而非匹配语法，但**模式内的 `\1` 反向引用 Java 支持、Rust regex 不支持** → 方案：(a) `fancy-regex` crate（支持反向引用/环视，性能较低）；(b) 记录不支持清单并在错误消息中提示。**推荐 (a)**：语义一致性优先。

### 1.4 BuiltInsForStringsMisc（4）—— `builtins/strings.rs` 或独立 `misc.rs`

| BI | 语义要点 |
|---|---|
| `absolute_template_name` | 相对模板路径 → 绝对（经 TemplateCache 名称规范化） |
| `boolean` | 字符串 → 布尔（`"true"/"false"`，否则 `TemplateModelException`） |
| `eval` | 字符串作为 FTL 表达式解析求值（运行时解析！） |
| `number` | 字符串 → 数字（解析失败抛 `InvalidReferenceException` 风格错误） |

### 1.5 BuiltInsForSequences（16）—— `builtins/sequences.rs`

| BI | 语义要点 |
|---|---|
| `chunk` | 分块（`?chunk(size)` → 子序列序列） |
| `drop_while` / `take_while` | 谓词（lambda）过滤前缀/后缀 |
| `filter` | lambda 谓词过滤 |
| `map` | lambda 映射（结果序列；`?map` 在字符串上也可用 → `BuiltInForSequence` vs 字符串分支） |
| `first` / `last` | 首/尾元素（空序列 → 错误或 null？**对照源码确认**） |
| `join` | 连接（支持分隔符参数；元素转字符串规则） |
| `max` / `min` | 数值或可比较元素极值（比较语义同 `ComparisonExpression`） |
| `reverse` | 反转（惰性或急切？**对照**：急切新序列） |
| `seq_contains` | 元素存在性（数值相等语义） |
| `seq_index_of` | 首次出现索引（支持 fromIndex） |
| `sequence` | 转序列视图 |
| `sort` / `sort_by` | 排序（`sort_by` 按子表达式键排序；数值/字符串/日期比较规则） |

### 1.6 BuiltInsForNumbers（15）—— `builtins/numbers.rs`

| BI | 语义要点 |
|---|---|
| `abs` / `abc` | 绝对值（`abc` 仅整数语义？**对照**：`abs` 通用） |
| `byte` / `short` / `int` / `long` / `float` / `double` / `c` / `cn` | 数值类型转换（`?c` 为 canonical 字符串；`?cn` 为 locale 无关数字——`?c` 家族统一走 CFormat） |
| `ceiling` / `floor` / `round` | 取整（`round` = HALF_UP） |
| `is_infinite` / `is_nan` | 浮点判定 |
| `lower_abc` / `upper_abc` | 数值 → 列号字母（`1→a`、`27→aa`） |
| `number_to_date` | 数值（毫秒或秒？）→ 日期（**对照参数语义**） |

### 1.7 BuiltInsForDates（2）—— `builtins/dates.rs`

| BI | 语义要点 |
|---|---|
| `iso_` | ISO 8601 字符串（`?iso` 可选参数：`"ms"`/`"m"` 截断级别、时区格式） |
| `iso_utc_or_local_` | `?iso_utc` / `?iso_local` 固定时区变体 |

### 1.8 BuiltInsForHashes（2）—— `builtins/hashes.rs`

| BI | 语义要点 |
|---|---|
| `keys` | 键序列（HashEx；`keys()` 方法委托 —— pyo3 侧映射 Python `keys()`） |
| `values` | 值序列（HashEx；Python `values()` 委托） |

### 1.9 BuiltInsForNodes（7）—— `builtins/nodes.rs`

| BI | 语义要点 |
|---|---|
| `ancestors` / `parent` / `root` | 节点祖先/父/根 |
| `children` | 子节点序列 |
| `node_name` / `node_namespace` / `node_type` | 节点元数据 |

### 1.10 BuiltInsForMarkupOutputs（1）—— `builtins/markup.rs`

| BI | 语义要点 |
|---|---|
| `markup_string` | 标记输出 → 其底层字符串 |

### 1.11 BuiltInsForExistenceHandling（7）—— `builtins/existence.rs`

| BI | 语义要点 |
|---|---|
| `default`（`!` 操作符实现） | 缺失/空 → 默认值（**惰性**：默认值表达式在需要时才求值；`(expr)!default` 形式） |
| `exists`（`??` 实现） | 是否存在（非求值内容） |
| `if_exists` | 缺失 → null（链式 `?if_exists`） |
| `has_content` | 非缺失且非空（标量非空串/序列非空/哈希非空） |
| `blank_to_null` / `empty_to_null` / `trim_to_null` | 空串处理 |

### 1.12 BuiltInsForMultipleTypes（23）—— `builtins/multi.rs`

| BI | 语义要点 |
|---|---|
| `string` | 按格式参数转字符串（`?string`/`?string("fmt")`/`?string(pattern)`）；无参时为默认 toString 语义（数字→canonical、日期→设置格式、布尔→true/false） |
| `c` / `cn` | canonical 数字字符串（CFormat 输出，见 specs/2026-08-01-formatting-design.md） |
| `date` / `number`（多类型版） | 类型转换（数字→日期、日期→数字？**对照**：`?number` 在字符串上；`?date` 在数字上为 epoch） |
| `size` | 序列/集合/哈希大小（HashEx） |
| `is_*` 全家族（18 个） | `is_boolean/is_collection/is_collection_ex/is_directive/is_enumerable/is_hash/is_hash_ex/is_indexable/is_macro/is_markup_output/is_method/is_node/is_number/is_sequence/is_string/is_transform` + `is_infinite/is_nan`（数字）—— 角色判定（映射 TModel 槽位检查，见 specs/2026-08-01-architecture-design.md §4.1） |
| `api` / `has_api` | BeanWrapper API 访问（**Rust 受限**，D1：抛 NotSupported 或 serde 受限实现） |
| `namespace` | 命名空间模型访问 |

### 1.13 BuiltInsForCallables（2）—— `builtins/callables.rs`

| BI | 语义要点 |
|---|---|
| `with_args` / `with_args_last` | 柯里化调用（`?with_args(1,2)` 预绑定参数生成部分应用） |

### 1.14 BuiltInsForLoopVariables（10）—— `builtins/loop_vars.rs`

| BI | 语义要点（全部读取循环局部上下文） |
|---|---|
| `counter` | 1 起始计数（`?counter` 取当前值 / `?counter++` 递增返回 —— `PLUS_PLUS` 语法） |
| `index` | 0 起始索引 |
| `has_next` | 是否还有下一项 |
| `is_first` / `is_last` / `is_even_item` / `is_odd_item` | 位置判定 |
| `item_cycle` / `item_parity` / `item_parity_cap` | 循环内轮换（`?item_cycle("odd","even")`） |

### 1.15 BuiltInsForOutputFormatRelated（2）+ BuiltInsWithLazyConditionals（2）—— `builtins/markup.rs` + `builtins/multi.rs`

| BI | 语义要点 |
|---|---|
| `esc` / `no_esc` | 按当前 OutputFormat 转义/不转义（字符串 → 标记输出；`BuiltInForLegacyEscaping` 处理旧版行为） |
| `then_` | `?then(a, b)` 惰性条件选择（`BuiltInsWithLazyConditionals`） |
| `switch_` | `?switch(case1: v1, ...)` 惰性多分支 |

### 1.16 特殊基类实例

- `SpecialBuiltIn`：`?default/exists/if_exists/has_content/keys/values/size/namespace/api/is_*` 等需要特殊求值时机（不先求值左操作数）。
- `BuiltInWithDirectCallOptimization`：`?upper_case` 等在常量折叠时可优化（Rust 可忽略优化，保持语义）。
- `BuiltInWithParseTimeParameters`：`?iso_*`、`?number_to_date`、`?truncate*` 等参数在解析期校验。
- `BuiltInBannedWhenAutoEscaping` / `BuiltInBannedWhenForcedAutoEscaping`：`?html` 等在 autoEscaping 下的禁用与错误消息。

## 2. Rust 实现模板（每个 BI）

```rust
// builtins/strings.rs 示例
pub fn cap_first(ctx: &mut BuiltInCtx) -> Result<TModel, TemplateError> {
    let s = ctx.target.expect_scalar()?;          // 角色检查 → NonStringException 对齐消息
    let mut it = s.chars();                        // 注意：Java 按 UTF-16 语义处
    // ... 实现
}
// 注册表（mod.rs）
pub enum BuiltInKind { Str(fn(...)), Num(..), Seq(..), ..., Lazy(..), Special(..) }
pub fn lookup(name: &str) -> Option<BuiltInKind>;  // 名称 → 实现（一次查表）
```

## 3. 语义风险点（实现时必须逐条对照源码验证）

1. **UTF-16 vs Unicode 码点**：`length/substring/pad/truncate` 系列按 Java `char`（UTF-16）计 —— Rust 需 `encode_utf16().count()` 或等价处理。
2. **正则**：反向引用/环视 → `fancy-regex`；`matches` 返回值结构对齐。
3. **`trim`**：Java `String.trim`（≤U+0020）与 Rust `trim()`（Unicode 空白）**不同**，必须自定义。
4. **`?c` 数字格式**：`?c` 输出与 locale 无关 canonical 形式（整数无小数点、大数无指数？）—— 由 CFormat 统一（specs/2026-08-01-formatting-design.md）。
5. **`?string` 无参语义**：不同版本行为差异（incompatibleImprovements 相关），锁定 2.3.34 行为。
6. **`sort` 比较**：跨类型比较规则（数字 vs 字符串）同 `ComparisonExpression`。
7. **`truncate` 长度单位**：UTF-16；`_w` 按词、`_c` 按字符边界、`_m` 标记感知。
8. **`?pad` 的 padString 循环填充**。
9. **`is_*` 对缺失变量的行为**：`(missing)?is_string` → 报错 vs 返回 false？**对照 `BuiltInsForExistenceHandling`/多类型实现确认**（`??` 与 `is_*` 组合）。
10. **`?join` 元素转换**：数字用 canonical、日期用设置格式、标量原样。

## 4. 验收标准（P3）

1. 183 个 BI 全部注册且可通过名称查表（编译期枚举核对清单，2026-08 阶段 A 达成 183/183）。
2. 黄金套件 `string-builtins1/2`、`string-builtins-regexps`、`encoding-builtins`、`list` 系列、`boolean-formatting`、`type-builtins` 用例逐字节通过。
3. UTF-16 语义用例（含中文/emoji 的 `?length`/`?substring`）与 Java 输出一致。
4. `?c`/`?cn`/`?string(pattern)` 数字格式与 Java CFormat 输出一致。

---

## 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 5）
- `docs/superpowers/plans/2026-08-03-alpha1-governance-hardening.md`（Task B1：最后 5 个 BI 补齐）
- `docs/superpowers/plans/2026-08-04-builtins-coverage-rounds.md`（builtins 对齐 3 批）
