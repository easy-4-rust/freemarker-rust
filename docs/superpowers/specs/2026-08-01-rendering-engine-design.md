# 渲染引擎与指令迁移设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（Environment.java 3,709 行）
- **依赖**：无外部依赖

---

## 1. Environment 总览（源：`core/Environment.java`，3,709 行）

| Java 成员/方法 | 行号 | Rust 对应 | 说明 |
|---|---|---|---|
| `instructionStack: TemplateElement[]` | :107 | `env.stack: Vec<Element>` | 指令栈（渲染循环核心） |
| `process()` | :315 | `Environment::process(&mut self)` | 渲染入口：`while let Some(el) = stack.pop() { visit(el)? }` |
| `visit(TemplateElement)` | :340 | `fn visit(&mut self, el) -> Result<()>` | `let next = el.accept(self)?; stack.extend(next)` |
| `visit(TemplateElement[])` | :367 | `fn visit_many(&mut self, els)` | 批量入栈（Java 手动内联，Rust 直接循环） |
| `replaceTopElement` | :414 | `fn replace_top(&mut self, el)` | 指令替换（`nested`/`return`/`stop` 依赖） |
| `pushLocalContext / popElement` | :2753/:2919 | `push_local/pop` | 作用域栈（循环变量/宏参数） |
| `handleTemplateException` | :1199 | `fn handle_error(&mut self, e)` | 异常处理（`<#attempt>` 恢复、`RETHROW`、`DEBUG` 模式） |
| `include(name, ctx...)` | :3126 | `fn include(&mut self, name)` | include/import 模板加载与执行 |
| `getTemplateForInclusion` | :3095 | 同上 | 带当前 locale/encoding/customLookupCondition |
| `write(Writer)` | :3666 | `out: &mut dyn Write` | 输出目标（String/文件/缓冲） |
| `Namespace`（内部类） | :3445+ | `Namespace`（HashMap<String, TModel>） | import 的宏命名空间（懒初始化 importLibNamespace :3283） |
| `NestedElementTemplateDirectiveBody` | :3445 | `TemplateDirectiveBody` trait 实现 | 自定义指令 body 回插 |
| `LocalContextStack` | 独立类 | `Vec<LocalContext>` | 循环变量/宏局部作用域链 |

## 2. 渲染循环伪代码（Rust）

```rust
pub struct Environment {
    template: Arc<Template>,
    root_map: TModel,                    // 根数据模型（SimpleHash）
    out: Box<dyn Write>,
    stack: Vec<Element>,                 // 指令栈
    local_context_stack: Vec<LocalContext>,
    namespace_stack: Vec<Rc<Namespace>>, // 当前命名空间链
    main_ns: Rc<Namespace>,
    import_lib_ns: Rc<LazyNamespace>,    // 懒初始化 import 库（Environment.java:3524-3569）
    settings: SettingsSnapshot,          // Configurable 快照（含继承链）
    // 控制流信号（对应 FlowControlException 家族）
    flow: Option<FlowSignal>,            // Break/Continue/Return/Stop
    attempt_depth: usize,
    // 状态
    auto_flushing: bool, cycle: u64, ...
}

pub fn process(&mut self) -> Result<(), TemplateError> {
    self.stack.push(self.template.root());
    while let Some(el) = self.stack.pop() {
        self.visit(&el)?;
        if let Some(sig) = self.flow.take() {
            // 流控信号沿栈向上传播，由目标指令（循环/宏/attempt）消费
            return self.propagate_flow(sig);
        }
    }
    Ok(())
}
```

## 3. 变量解析语义（对应 `_getVariable` / `Environment.getVariable`）

查找顺序（关键语义，必须逐层复刻）：
1. **局部上下文栈**（`local_context_stack`，从栈顶向下）：循环变量（`item`）、宏参数、`<#local>` 变量、`<#nested>` 的调用方局部
2. **当前命名空间**（`namespace_stack` 顶部）：`<#assign>`/`<#global>`/宏定义
3. **全局命名空间**（`main_ns`）：`<#global>` 变量、宏/函数定义
4. **数据模型根**（root map）：外部传入变量
5. 未找到 → `InvalidReferenceException`（消息含建议？`"The following has evaluated to null or missing: ==> name"`，`??`/`!`/`default` 抑制）

特殊：
- `<#import>` 的命名空间引用：`<@ns.macro>`、`ns.var`（`Dot` 解析到 `Namespace` 模型）
- `api` 内建（`?api`）：需 `TemplateModelWithAPISupport`（D1 决策：Rust 侧不实现或受限）
- `now`/`true`/`false`/`float`/`double` 等内置变量（`BuiltinVariable`）

## 4. 指令全清单（源类 → Rust 模块 → 语义要点）

### 4.1 控制流

| 指令 | 源类 | 语义要点 |
|---|---|---|
| `if/elseif/else` | `IfBlock`/`ConditionalBlock` | 条件求值 → boolean 角色；非布尔抛 `NonBooleanException`；无 else 时为假静默 |
| `list` | `IteratorBlock` | 循环变量、`as x`、`sep`/`items`/`else` 子块；`fallbackOnNullLoopVariable` 设置影响 null 变量回退；迭代器（TemplateModelIterator）语义 |
| `items/sep/else` | `Items/Sep/ElseOfList/ListElseContainer` | 与 `list` 搭配的循环细分块（每个块的 `accept` 由 IteratorBlock 驱动） |
| `switch/case/default` | `SwitchBlock/Case` | 多值匹配（表达式结果相等比较）；无匹配且无 default 时静默 |
| `attempt/recover` | `AttemptBlock/RecoveryBlock` | try/catch 语义：recover 块执行后恢复；`attemptExceptionReporter` 回调；错误上下文压栈 |
| `break/continue` | `Break/ContinueInstruction` | 仅循环内合法，否则 `BreakOrContinueException` |
| `return` | `ReturnInstruction` | 宏/函数内返回；非宏内抛异常 |
| `stop` | `StopInstruction` | `StopException` 终止整个渲染（非错误，可被 `?is_*` 外捕获）；`stop` 无参与带参（输出消息） |
| `fallback/on` | `FallbackInstruction/On` | `<#fallback>`（节点模型访问回退） |
| `flush` | `FlushInstruction` | 冲刷输出缓冲（autoFlush 设置） |
| `trim` | `TrimInstruction` | 块内首尾空白裁剪 |

### 4.2 定义与调用

| 指令 | 源类 | 语义要点 |
|---|---|---|
| `assign` | `Assignment`/`AssignmentInstruction` | 支持 `=`、`+=`、`-=`、`*=`、`/=`、`%=`、`++`、`--` 八种形式；`<#assign x>` 捕获块输出 |
| `global` | `BlockAssignment` 系列 | 全局命名空间变量 |
| `local` | `BlockAssignment` 系列 | 宏内局部变量 |
| `macro` | `Macro` | 参数默认值（`param=default`）、可选参数（`param?`）、catch-all（`args...`）、`##` 无换行语法；`<@.ns>` 命名空间限定；`<@macro/>` 自闭合；`nested` 参数（`<#nested x>` 传参） |
| `function` | `Macro`（Function 子类） | 与 macro 同构，`?return` 为返回值；`caller` 内建变量访问调用方局部 |
| `call`（`<@...>`） | `UnifiedCall` | 用户自定义指令分发：内置指令名（`if` 等，语法糖）优先，否则命名空间宏/变量；参数先表达式求值再赋值 |
| `nested` | `BodyInstruction` | 宏体回插（`replaceTopElement` 机制）；可带参数传给调用方 `<#nested x>` |
| `include` | `Include` | 包含模板：相对路径解析、局部/全局变量可见性（默认全部可见；`ignore_missing`/`parse`/`encoding` 参数）；被包含模板的错误上下文 |
| `import` | `LibraryLoad` | 导入宏库命名空间：`<#import "lib.ftl" as ns>`；懒初始化（`LazilyInitializedNamespace`） |
| `interpret` | `Interpret` | 动态解析字符串模板（内置变量 `interpret` 相关） |
| `setting` | `PropertySetting` | 渲染期修改设置（`<#setting locale="...">` 等） |
| `transform` | `TransformBlock` | 旧式 `<@transform>`（TemplateTransformModel） |

### 4.3 转义与输出

| 指令 | 源类 | 语义要点 |
|---|---|---|
| `escape` | `EscapeBlock` | 作用域内插值统一包装表达式（字符串→markup 转换） |
| `noescape` | `NoEscapeBlock` | 取消 escape |
| `autoesc/noautoesc` | `AutoEscBlock/NoAutoEscBlock` | 按 outputFormat 自动转义开关（模板级 `autoEscaping` 设置） |
| `outputformat` | `OutputFormatBlock` | `<#outputformat "HTML">` 切换输出格式（影响 `?esc` 等） |
| `compress` | `CompressedBlock` | 块内空白压缩（`StandardCompress`） |
| `ftl`（头部） | `FtlHeader` | `[#ftl]` 声明编码/属性 |
| `t/lt/gt` | — | 文本转义指令（`<#t>` 行首空白裁剪） |
| `noparse` | — | 块内不解析（词法 NO_PARSE 状态） |
| `comment` | `Comment` | 注释块 |
| `debug` | `DebugBreak` | 调试断点（占位） |

### 4.4 XML 节点

| 指令 | 源类 | 语义要点 |
|---|---|---|
| `visit` | `VisitNode` | 访问节点树（TemplateNodeModel） |
| `recurse` | `RecurseNode` | 递归访问子节点 |
| `body` | `BodyInstruction` | 节点访问中的 body 回插 |

## 5. 表达式求值语义（对应各 `eval(Environment)` 方法）

- **结果类型**：全部返回 `TModel`（角色槽位）；`?string` 等内建在求值后应用。
- **短路**：`&&`/`||` 短路（And/OrExpression）；`!`、`??`、`default`、`if_exists` 惰性。
- **类型检查时机**：运算时（`+` 要求数字或字符串角色；`==` 允许跨类型比较语义——数字按数值、字符串按内容、布尔相同、其他 `==` 语义详见 Java `MiscUtil`）。
- **数值运算**：委托 `ArithmeticEngine`（默认 `BigDecimalEngine`）：
  - 加减：`scale = max(s1, s2)`
  - 乘：`scale = s1 + s2`
  - 除：`scale = max(s1, s2)`（？——**必须对照 `ArithmeticEngine.java` 源码逐行确认**，这是语义风险点 #1），`ROUND_HALF_EVEN`
  - 溢出到整数则降级为整数表示（`OptimizerUtil.optimizeNumberRepresentation`）
- **字符串 `+`**：`AddConcatExpression` —— 数字转为字符串用 locale 无关格式化；标量/数字/日期/布尔按各自 `?string` 规则。
- **比较**：`ComparisonExpression` 支持 `> < >= <= == !=`；`lt/gt/lte/gte` 同义；数字/字符串/日期/布尔跨类型时按 Java 规则（字符串 vs 数字 → 尝试解析？**须对照源码确认**）。
- **范围**：`Range`/`RangeModel` —— `1..5` 含端、`1..<5` 排端、`1..*` 无界（惰性 `RightUnboundedRangeModel`）；`BoundedRangeModel` 与 `ListableRightUnboundedRangeModel` 区分大小写语义（`?size` 在有界时可用）。
- **lambda**：`LocalLambdaExpression` —— `x -> expr`；`?map/filter/take_while/drop_while/join` 等接收 lambda；闭包捕获环境（`LocalContext` 快照，Java 用 `LocalContextWithNewLocal`）。
- **方法调用**：`MethodCall` —— `expr(args)`；对 `TemplateMethodModel(Ex)` 分发（Ex 传 TModel 列表，非 Ex 传字符串列表——**JythonWrapper 兼容的关键**）。
- **`new`**：`NewBI` —— 构造对象（Rust 侧受限或不可用，见 D1）。
- **`?api`**：受限（D1）。

## 6. 作用域与命名空间细则

- `Namespace` = `HashMap<String, TModel>` + 懒初始化（import 库仅在首次访问时解析并注册宏）。
- 宏调用：参数 → `LocalContext` 压栈；`<#local>` → 宏专属命名空间层；宏内部 `?is_first`/`item_cycle` 等循环内建读取局部上下文。
- 循环变量作用域：`list` 循环变量在循环体内可见、循环外不可见（`LocalContextStack` 弹出）；`loop` 变量（`?counter`/`?index`/`?has_next` 等）同层管理。
- 命名空间解析：`<@ns.macro/>` 中 `ns` 先查命名空间注册表再查变量。

## 7. 自定义指令与模板方法扩展点（Rust 公共 API）

```rust
pub trait TemplateDirectiveModel {           // 对应 TemplateDirectiveModel.execute
    fn execute(&self, env: &mut Environment, params: &HashMap<String, TModel>,
               loop_vars: &[&mut TModel], body: Option<&dyn TemplateDirectiveBody>)
        -> Result<(), TemplateError>;
}
pub trait TemplateDirectiveBody {            // 对应 TemplateDirectiveBody.render
    fn render(&self, env: &mut Environment, out: &mut dyn Write) -> Result<(), TemplateError>;
}
pub trait TemplateMethodModelEx {            // 对应 TemplateMethodModelEx.exec
    fn exec(&self, args: Vec<TModel>) -> Result<TModel, TemplateError>;
}
pub trait TemplateTransformModel { /* ... */ }
```

用户通过 `Configuration::set_shared_variable(name, model)` / 数据模型注入自定义指令与方法——与 Java 用法对齐。

## 8. 验收标准（P2）

1. 黄金套件控制流类用例（if/list/macros/attempt/switch/break/return）输出逐字节一致。
2. 作用域语义用例（`variables.txt`/`var-layers.txt`/`macro 嵌套`）通过。
3. 流控错误场景（循环外 break、宏外 return）错误消息一致。
4. `nested` 参数传递、`<@macro/>` 自闭合、宏默认参数/可选参数/catch-all 通过。
5. 性能冒烟：10 万次 `${x}` 插值渲染无内存暴涨（预分配输出缓冲）。

---

## 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`（Stage 3-4）
- `docs/superpowers/plans/2026-08-04-builtins-coverage-rounds.md`（指令类拆分 6 批）
