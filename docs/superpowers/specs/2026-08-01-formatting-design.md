# 格式化与自动转义设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（OutputFormat/CFormat/TemplateDateFormat/TemplateNumberFormat 等）
- **依赖**：无外部依赖

---

> 源：`freemarker-core/src/main/java/freemarker/core/` 下 `*OutputFormat*.java`、`*CFormat*.java`、`*TemplateDateFormat*.java`、`*TemplateNumberFormat*.java`、`JSONParser.java`、`ExtendedDecimalFormatParser.java`

## 1. OutputFormat 体系（`fmt/output_format.rs`）

```rust
pub trait OutputFormat {
    fn name(&self) -> &str;                                   // "HTML" / "plainText" / ...
    fn is_markup(&self) -> bool;                              // 输出是否标记类型
    fn escape(&self, s: &str) -> String;                      // escape 内建（?esc）转义
    fn output_model_class(&self) -> MarkupKind;               // 标记输出模型类型（HTML/XML/...）
    fn from_markup_output(&self, m: &TModel) -> Result<String, TemplateError>; // 标记输出 → 字符串
}
```

| Java 类 | Rust | 转义规则要点（逐字对照 `StringUtil`） |
|---|---|---|
| `PlainTextOutputFormat` | `PlainText` | 无转义；`is_markup = false` |
| `HTMLOutputFormat` | `Html` | `& < > " '`；非 ASCII 按 `HTMLEnc`（`&#xHHHH;` 规则） |
| `XMLOutputFormat` | `Xml` | `& < > " '` |
| `XHTMLOutputFormat` | `XHtml` | 同 HTML |
| `JavaScriptOutputFormat` | `JavaScript` | `js_string` 转义规则（`\` `'` `"` 换行等 + 非 ASCII `\uXXXX`） |
| `JSONOutputFormat` | `Json` | `json_string` 规则（`\uXXXX`、控制字符） |
| `CSSOutputFormat` | `Css` | CSS 转义 |
| `RTFOutputFormat` | `Rtf` | RTF 转义 |
| `CommonMarkupOutputFormat` | `CommonMarkup` | 泛型标记（用户扩展） |
| `CombinedMarkupOutputFormat` | `Combined` | 多格式组合（`?esc` 按内容选择） |
| `TemplateMarkupOutputModel` 族 | `MarkupModel { kind, content }` | HTML/XML/RTF/XHTML 变体 + `?markup_string` |
| `UndefinedOutputFormat` | 错误哨兵 | 未注册格式报错 |

- **注册表**：`Configuration.STANDARD_OUTPUT_FORMATS` → Rust 静态表（`"HTML"→Html` 等）；`setOutputFormat`/`setOutputEncoding` 联动（输出编码影响 `?html` 是否转非 ASCII）。
- **`?esc`/`?no_esc`**：字符串 → 当前 OutputFormat 的标记模型；`?no_esc` 用于已转义内容。

## 2. CFormat 体系（`fmt/cformat.rs`，`?c` 家族）

```rust
pub trait CFormat {
    fn name(&self) -> &str;   // "JSON" / "Java" / "JavaScript" / "JavaScriptOrJSON" / "XSC" / "Legacy"
    fn format_number(&self, n: &TNumber) -> Result<String, TemplateError>;
    fn format_string(&self, s: &str) -> Result<String, TemplateError>;
    fn format_boolean(&self, b: bool) -> Result<String, TemplateError>;
    fn format_date(&self, d: &DateValue) -> Result<String, TemplateError>;
    fn format_value(&self, v: &TModel) -> Result<String, TemplateError>;  // 递归（哈希/序列 → JSON 对象/数组）
}
```

| Java | Rust | 语义要点 |
|---|---|---|
| `LegacyCFormat` | `Legacy` | 默认：`?c` 老行为（数字 canonical、字符串按 ?string 无引号等） |
| `JSONCFormat` | `JsonC` | 数字 `?c` 输出 JSON 数字（整数无 `.0`、非有限值 → 错误）；字符串带引号转义 |
| `JavaCFormat` / `JavaScriptCFormat` | `JavaC`/`JsC` | Java/JS 字面量（布尔 true/false、null、`'`/`"` 规则差异） |
| `JavaScriptOrJSONCFormat` | `JsOrJsonC` | 按 `?c` 与 `?cn` 上下文选择（**`?cn` = locale 无关 canonical 数字**） |
| `XSCFormat` | `XscC` | XML Schema canonical（`1.0E3` 指数形式等） |
| `StandardCFormats` | 静态注册表 | 按名称查找（`"JSON"`/`"Java"`/`"XSC"`...） |

**`?c` 数字输出细则**（对照 `CTemplateNumberFormat`/各 CFormat 实现）：
- 整数：`123`（无小数点）；Long 溢出 → 原样；浮点：`1.5`、`1.0E10`（指数阈值）；Decimal：去尾零。
- **`?cn`（数字，locale 无关）** vs **`?c`（随 CFormat）**：`${1.5?c}` 在 JSON 下 `1.5`，在 Java 下 `1.5`；`${"a"?c}` JSON 下 `"a"`、Legacy 下 `a`。

## 3. 数字格式化（`fmt/number.rs`）

- 默认 `numberFormat` = `"number"` 内部名 → `CTemplateNumberFormat`（canonical，locale 无关）——**2.3.34 默认即 `?c` 语义**（`BackwardCompatibleTemplateNumberFormat` 处理旧版差异）。
- 模式字符串：`"0.##"`/`"#,##0.00"` 等 → **DecimalFormat 语义子集**（`ExtendedDecimalFormatParser`，含引号字面量、分号正负模式、`%`/`‰`、指数、分组）。Rust 用 `bigdecimal` + 自实现模式解析（或引 `num-format` crate 后按需裁剪）。
- `?string("0.##")` 显式模式 → 每调用解析并缓存（`TemplateNumberFormatFactory` 缓存）。

## 4. 日期格式化（`fmt/date.rs`）

- `ISOLikeTemplateDateFormat`（约 1,300 行源）：`"yyyy-MM-dd"`、`"HH:mm:ss"`、`"yyyy-MM-dd'T'HH:mm:ssXXX"`、`"short"/"medium"/"long"/"full"` 命名模式 → 预定义 ISO 模式表；支持 `'T'` 引号、`XXX`/`XX`/`X` 时区、`B` 等。**逐项对照实现**。
- `ISOTemplateDateFormat`/`XSTemplateDateFormat`/`JavaTemplateDateFormat`（委托 `DateUtil` 的 Java `SimpleDateFormat` 模式——Rust 仅支持 ISO 模式 + 常用子集；`JavaTemplateDateFormat` 对不支持模式抛 `InvalidFormatStringException`？**实现时对照**）。
- 工厂：`AliasTemplateDateFormatFactory`（`iso`/`iso_utc`/`iso_local` 别名）、`ISOLikeTemplateDateFormatFactory`。
- 时区语义：`Environment.time_zone`、`sql_date_and_time_time_zone`、`?iso_utc_or_local` 的 UTC/本地选择；`DateUtil` 的 `TIME_ZONE_*` 常量。

## 5. 自动转义与空白处理（`fmt/` + 解析器标记）

### 5.1 autoEscaping（设置 + `<#autoesc>`）

- 模板级设置 `autoEscaping`：`on/off/default`（`"default"` = 随 outputFormat 的 `isOutputFormatMixingAllowed` + incompatibleImprovements：2.3.24+ 默认 HTML 模板自动转义为 true 语义？**对照源码确认默认矩阵**）。
- `<#autoesc>`/`<#noautoesc>` 块级覆盖；`?esc`/`?no_esc` 表达式级。
- 转义时机：**插值输出时**（`DollarVariable`/`Interpolation` 的 accept），非转义存储。
- `BuiltInBannedWhenAutoEscaping`：`?html` 等在自动转义开启时抛错（消息含建议）。

### 5.2 whitespaceStripping（`template/Configuration.java` + 解析期标记）

- 规则（`WhitespaceStripping` 等价逻辑，Java 在 `FMParser`/`Template` 侧标记）：行首空白后跟 FTL 指令行 → 剥离该行空白与换行；`<#t>`（行首裁剪）、`<#nt>`（不裁剪）显式控制；`[#ftl]` 可关闭。
- **Rust 实现位置**：解析期在 `TextBlock` 上打标记（`strip_before/strip_after`），渲染期执行——与 Java 一致（解析期标记、渲染期裁剪，保证 `?interpret` 动态模板行为一致）。

### 5.3 `#compress`（`CompressedBlock` → `StandardCompress`）

- 行间空白压缩：每行首尾空白 + 连续空白行合并为单行（保留行内空白）。

## 6. JSON 相关（`JSONParser.java` → Rust）

- `JSONParser`：模板表达式中的 JSON 字面量解析（`{"a": 1}` 作为哈希字面量）—— 2.3.x 的 `json` 序列字面量？**确认**：`JSONParser` 用于 `?json_string` 校验与 `#json` 字面量（若存在）。Rust 用 `serde_json` 解析 + 错误消息对齐。

## 7. 验收标准（P4 附属）

1. `boolean-formatting.txt`、`dateformat-iso-like.txt`、`dateformat-iso-bi*.txt`、`dateparsing.txt`、`encoding-builtins*.txt`、`whitespace-trim.txt`、`wstripping.txt` 黄金用例逐字节通过。
2. `?c`/`?cn` 在 JSON/Java/JS/XSC/Legacy 五种 CFormat 下输出与 Java 一致（含边界：NaN/Infinity、大整数、指数）。
3. 自动转义组合矩阵（autoesc 开/关 × ?esc/?no_esc × 块级覆盖）输出一致。
4. 空白剥离边界（指令行首/行尾、`<#t>`、`[#ftl] whitespace_stripping=false`）通过。

---

## 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 6）
- `docs/superpowers/plans/2026-08-04-refactor-2c-3a-3b-batches.md`（Task 3.1：c_format 变体 + 自动转义禁令）
- `docs/superpowers/plans/2026-08-04-builtins-coverage-rounds.md`（Task 3.2：格式/输出模型拆分）
