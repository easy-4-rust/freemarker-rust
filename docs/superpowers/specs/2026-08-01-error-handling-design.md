# 错误处理与诊断设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（TemplateException + 30+ 子类）
- **依赖**：无外部依赖

---

> 源：`freemarker-core/src/main/java/freemarker/template/TemplateException.java`、`core/_MiscTemplateException.java`、`core/Non*Exception.java`（30+）、`core/_ErrorDescriptionBuilder.java`、`core/_MessageUtil.java`、`core/ParseException.java`

## 1. 异常层级 → Rust 错误模型

```
TemplateException（根）
 ├── TemplateModelException          → model 层错误（用户模型抛）
 ├── InvalidReferenceException      → 变量缺失（"The following has evaluated to null or missing: ==> x"）
 ├── _MiscTemplateException        → 运行时通用错误（含类型错误）
 │    └── NonBooleanException / NonDateException / NonExtendedHashException /
 │        NonExtendedNodeException / NonHashException / NonListableRightUnboundedRangeModelException /
 │        NonMarkupOutputException / NonMethodException / NonNamespaceException / NonNodeException /
 │        NonNumericalException / NonSequenceException / NonSequenceOrCollectionException /
 │        NonStringException / NonStringOrTemplateOutputException / NonUserDefinedDirectiveLikeException /
 │        UnexpectedTypeException（基类）/ UnknownDateTypeFormattingUnsupportedException /
 │        UnknownDateTypeParsingUnsupportedException / APINotSupportedTemplateException ...
 ├── InvalidFormatParametersException / InvalidFormatStringException / UnformattableValueException /
 │    UnparsableValueException / UndefinedCustomFormatException / UnregisteredOutputFormatException
 ├── ParseException（解析期）
 ├── StopException（stop 指令，非错误但会终止）
 ├── FlowControlException（break/continue 流控信号）
 ├── MalformedTemplateNameException（路径非法）
 ├── TemplateNotFoundException（加载失败）
 └── IoException 包装
```

Rust 对应：

```rust
pub enum TemplateError {
    InvalidReference { name: String, ctx: ErrorCtx },
    TypeMismatch { expected: &'static str, actual: &'static str, ctx: ErrorCtx }, // UnexpectedTypeException 族
    Misc { message: String, ctx: ErrorCtx },            // _MiscTemplateException
    Model(Box<dyn TemplateModelError>),                  // TemplateModelException（用户模型）
    Parse(ParseError),
    Stop { message: Option<String>, ctx: ErrorCtx },
    Flow(FlowKind),                                       // Break/Continue（内部信号）
    NotFound { name: String, ctx: ErrorCtx },
    Io(std::io::Error),
    Python(Box<PyErr>),                                   // pyo3 feature
}
```

## 2. ErrorCtx —— 错误上下文（对应 `_ErrorDescriptionBuilder`）

Java 错误消息结构（`TemplateException.getMessage()`）：

```
Error executing FreeMarker template
FreeMarker template error (DEBUG mode; use RETHROW in production!)
<消息主体>
    at com.example.TemplateRenderer.main(TemplateRenderer.java:9)
    at freemarker.core.Environment.process(Environment.java:xxx)
    at freemarker.template.Template.process(Template.java:xxx)
Caused by: freemarker.core.InvalidReferenceException: The following has evaluated to null or missing:
==> user.name  [in template "test.ftl" at line 3, column 7]
    at freemarker.core.InvalidReferenceException...
```

Rust 对齐结构：

```rust
pub struct ErrorCtx {
    pub template_name: String,
    pub line: u32, pub column: u32,          // beginLine/beginColumn（AST Span）
    pub end_line: u32, pub end_column: u32,
    pub instruction_stack: Vec<StackFrame>,  // 指令栈转储（宏调用链、?string 等）
}
```

**消息模板清单**（`error/messages.rs`，逐字复刻，标注源文件与行）：
- `The following has evaluated to null or missing: ==> {expr} [in template "{name}" at line {l}, column {c}]`
- `For "{target}" something that is a {expected} is required, but this has evaluated to a {actual} (wrapper: {wrapper}):` —— `_UnexpectedTypeErrorExplainerTemplateModel` 参与描述
- `Failed to get the value of {expr}`
- `Method call failed on object with class {cls}: {cause}`（`MethodCall` 包装）
- `Error executing FreeMarker template` + `FreeMarker template error (DEBUG mode...)` 前缀
- `Parsing error in template "{name}" at line {l}, column {c}. {details}` + `Error tokenizing`/`Error parsing` 变体
- `Template not found for name "{name}".` + `The problematic instruction was: ...`（指令栈） + `Java stack trace` 段
- 类型名：`actual` 用 `type_name()`（TModel 槽位元数据），如 `simple hash`、`sequence`、`string`、`number`、`boolean`、`date`、`method`、`directive`、`node`、`nothing` —— **与 Java 的 `toFTLTypeName` 一致**

## 3. 错误处理流程（对应 `Environment.handleTemplateException :1199-1235`）

1. 渲染中任意 `TemplateError`（非 Flow/Stop）进入处理：
   - 若在 `<#attempt>` 深度内 → 记录并交给 `RecoveryBlock` 恢复（`attemptExceptionReporter.report` 回调，默认 `LoggingAttemptExceptionReporter`）
   - 否则 → 交给 `templateExceptionHandler`：
     - `RETHROW_HANDLER`：原样抛出（生产默认）
     - `DEBUG_HANDLER`：打印调试消息到 stderr 后抛出
     - `HTML_DEBUG_HANDLER`：HTML 转义调试消息
     - `IGNORE_HANDLER`：静默继续（输出部分截断）
2. `wrapUncheckedExceptions`：Rust 无 checked/unchecked 之分 → 统一走 `TemplateError`（用户模型异常原样透传）。
3. `logTemplateExceptions`：错误经 `freemarker.log.Logger` 记录（Rust `log` crate 适配）。

## 4. 消息逐字对齐策略

- **首选**：直接运行 Java 版（gradle 测试）产生错误消息样例，建立 `error/expected_messages/*.txt` 基线，Rust 单测断言相等。测试模板：
  - 未定义变量、`??`/`!`/`default` 抑制后仍错
  - 类型不匹配（`${1 + "a"}`、`<#if 1>`、`${x[0]}` 非序列）
  - 解析失败（未闭合标签、非法 token）
  - `stop`/`break`/`return` 违规
  - include 缺失、路径非法
- **容忍差异清单**（记录在案，允许偏差）：
  - Java 堆栈帧（`at freemarker.core...`）→ Rust 用指令栈摘要替代（P6 对齐格式）
  - `wrapper: ...` 后缀（包装器类名）→ Rust 用 wrapper 名称占位
  - 版本号/时间戳类信息
- **不可容忍**：模板名、行号、列号、指令名、变量名、消息主体文本。

## 5. 调试与诊断辅助

- `?dump`/`#debug`：`FreeMarkerTree`（AST dump）—— P6 提供 `Template::dump()`（Debug 格式对齐）。
- `TemplateProcessingTracer`：渲染轨迹（Rust `tracing` 可选 feature，默认关闭）。
- `Environment.isInAttemptBlock`（:599）、`attempt_depth` 计数用于错误上下文。

## 6. 验收标准（P6 专项）

1. 错误消息基线测试 100% 通过（§4 清单）。
2. `<#attempt><#recover>` 组合（含嵌套 attempt、recover 内再错）与 Java 行为一致。
3. `templateExceptionHandler` 四模式行为一致（DEBUG 输出格式对齐）。
4. 指令栈转储：宏调用链/`?string` 嵌套/循环嵌套的错误定位与 Java 一致。

---

## 对应计划

- `docs/superpowers/plans/2026-08-01-p0-skeleton-baseline.md`（Stage 2：错误体系骨架）
- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 7：错误消息对齐）
- `docs/superpowers/plans/2026-08-04-m5-error-alignment.md`（M5 错误对齐收尾）
- `docs/superpowers/plans/2026-08-04-builtins-coverage-rounds.md`（Task 3.1：异常类拆分）
