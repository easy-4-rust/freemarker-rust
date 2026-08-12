# 架构设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97）
- **依赖**：Cargo workspace、PyO3 0.29、roxmltree

---

## 1. Workspace 布局

```
freemarker-rust/
├── Cargo.toml                    # [workspace] members = ["freemarker", "freemarker-pyo3"]
├── .gitignore
├── docs/                         # 本计划文档
├── freemarker/                   # 核心引擎 crate（对应 freemarker-core）
│   ├── Cargo.toml                # 无 pyo3 依赖（feature: "python" 仅用于可选导出）
│   ├── src/
│   │   ├── lib.rs                # 公共 API 面：Configuration/Template/Environment/TemplateModel
│   │   ├── parser/               # 词法 + 递归下降（对应 FTL.jj / FMParser）
│   │   │   ├── mod.rs
│   │   │   ├── lexer.rs          # 多状态词法（5 状态）
│   │   │   ├── grammar.rs        # 产生式函数（Expression/TemplateElement 全量）
│   │   │   └── error.rs          # ParseError（带行/列）
│   │   ├── ast/                  # AST 类型（TemplateElement enum + Expression enum + 辅助）
│   │   │   ├── mod.rs
│   │   │   ├── element.rs        # TemplateElement 家族
│   │   │   ├── expression.rs     # Expression 家族
│   │   │   └── template.rs       # Template（解析产物：root/宏表/行号表）
│   │   ├── env/                  # 渲染引擎
│   │   │   ├── mod.rs
│   │   │   ├── environment.rs    # Environment（指令栈、作用域、命名空间、输出）
│   │   │   ├── configurable.rs   # Configurable 设置链（继承/覆盖/合并）
│   │   │   ├── context.rs        # LocalContext / 循环变量 / 宏参数
│   │   │   └── settings.rs       # 设置键枚举（字符串键 ↔ 类型化访问）
│   │   ├── model/                # 数据模型
│   │   │   ├── mod.rs
│   │   │   ├── traits.rs         # TemplateModel 角色 trait 家族
│   │   │   ├── value.rs          # TModel 角色槽位结构（见 §4）
│   │   │   ├── simple.rs         # SimpleScalar/Number/Boolean/Date/Hash/List/Sequence...
│   │   │   ├── lazy.rs           # LazilyGeneratedCollection 族
│   │   │   └── nothing.rs        # TemplateNullModel / GeneralPurposeNothing
│   │   ├── wrapper/              # 对象包装
│   │   │   ├── mod.rs            # ObjectWrapper trait
│   │   │   ├── simple_wrapper.rs # SimpleObjectWrapper 等价
│   │   │   └── default_wrapper.rs# DefaultObjectWrapper 等价（serde 适配）
│   │   ├── builtins/             # 内建函数（133 个，见 specs/2026-08-02-builtins-design.md）
│   │   │   ├── mod.rs            # 注册表：&'static str → BuiltInKind
│   │   │   ├── strings.rs
│   │   │   ├── strings_encoding.rs
│   │   │   ├── strings_regexp.rs
│   │   │   ├── sequences.rs
│   │   │   ├── numbers.rs
│   │   │   ├── dates.rs
│   │   │   ├── hashes.rs
│   │   │   ├── nodes.rs
│   │   │   ├── markup.rs
│   │   │   ├── existence.rs
│   │   │   ├── multi.rs
│   │   │   └── loop_vars.rs
│   │   ├── directive/            # 内置指令实现
│   │   │   ├── mod.rs
│   │   │   ├── control.rs        # if/list/switch/attempt/break/continue/return/stop...
│   │   │   ├── define.rs         # assign/macro/function/local/global/nested/sep/items
│   │   │   ├── include.rs        # include/import
│   │   │   ├── escape.rs         # escape/noescape/autoesc/noautoesc/outputformat/compress
│   │   │   └── xml.rs            # visit/recurse/fallback/on/body
│   │   ├── fmt/                  # 格式化与输出
│   │   │   ├── mod.rs
│   │   │   ├── output_format.rs  # OutputFormat trait + 各实现
│   │   │   ├── cformat.rs        # CFormat（JSON/Java/JS/XSC...）
│   │   │   ├── date.rs           # ISO/Java 日期格式化与解析（chrono 适配）
│   │   │   ├── number.rs         # Decimal 数字格式化（bigdecimal 适配）
│   │   │   └── markup.rs         # TemplateMarkupOutputModel 族
│   │   ├── cache/                # 加载与缓存
│   │   │   ├── mod.rs
│   │   │   ├── loader.rs         # TemplateLoader trait + File/String/Class/URL/Multi
│   │   │   ├── cache.rs          # TemplateCache（键/负查找/延迟）
│   │   │   ├── storage.rs        # Strong/Mru/Lru 存储
│   │   │   ├── name_format.rs    # TemplateNameFormat（020300/020400 规范化）
│   │   │   └── lookup.rs         # TemplateLookupStrategy（acquisition/本地化回退）
│   │   ├── arithmetic/           # ArithmeticEngine（BigDecimal 默认引擎）
│   │   ├── regexp.rs             # RegexpHelper（regex crate 适配 Java 语义）
│   │   ├── error/                # 错误体系
│   │   │   ├── mod.rs            # TemplateError 枚举
│   │   │   ├── messages.rs       # 逐字消息模板（与 Java 对齐）
│   │   │   └── builder.rs        # _ErrorDescriptionBuilder 等价
│   │   └── util/                 # StringUtil/DateUtil/NumberUtil/OptimizerUtil 等价
│   └── tests/                    # 集成测试（黄金套件，见 specs/2026-08-01-testing-strategy-design.md）
│       ├── suite/                # testcases.xml 翻译数据（.ftl + expected）
│       ├── golden.rs             # 黄金套件 runner
│       └── ...
├── freemarker-pyo3/              # Python 绑定（对应 freemarker-jython25）
│   ├── Cargo.toml                # pyo3 = "0.29"（cdylib + extension-module）
│   ├── pyproject.toml            # maturin 打包
│   ├── src/
│   │   ├── lib.rs                # #[pymodule] freemarker（Configuration/Template 类）
│   │   ├── wrapper.rs            # PyObjectWrapper（JythonWrapper 等价）
│   │   ├── models.rs             # Python 角色适配（dict/list/int/float/str/None/可调用）
│   │   ├── bridge.rs             # TemplateModel ↔ PyObject 双向（unwrap 通用适配器）
│   │   ├── gil.rs                # GIL 获取/释放策略
│   │   └── errors.rs             # PyErr ↔ TemplateError
│   └── tests/                    # Python 侧测试（pytest）
├── benches/                      # 性能基准（criterion）
└── scripts/                      # 套件翻译/对比脚本
```

## 2. Crate 与模块依赖规则

```
freemarker (lib)
  ├── parser → ast
  ├── ast → (无依赖，纯数据)
  ├── env → model, error, fmt, cache, arithmetic
  ├── model → error
  ├── wrapper → model
  ├── builtins → ast, env, model, fmt, regexp
  ├── directive → ast, env, model
  ├── fmt → model, error
  ├── cache → ast(parse), model(不直接)
  ├── arithmetic → model, error
  └── util → (无依赖)

freemarker-pyo3 (cdylib)
  └── freemarker (path dep), pyo3 0.29
```

- **禁止反向依赖**：`ast` 不依赖 `env`；`parser` 不依赖 `model`（解析期不做类型检查）。
- **pyo3 隔离**：`freemarker` 的 Cargo.toml 无 pyo3；`freemarker-pyo3` 依赖它。若核心需要持有 `Py<PyAny>` 的通用槽位（D4 选型），通过 `feature = "python"` + 可选的 `pyo3` 依赖注入（`[features] python = ["dep:pyo3"]`），保持默认无 Python。

## 3. 依赖选型

| 领域 | crate | 版本策略 | 说明 |
|---|---|---|---|
| 正则 | `regex` | 1.x | 对齐 Java `java.util.regex` 语义（注意差异：见 specs/2026-08-02-builtins-design.md 正则节） |
| 日期时间 | `chrono` | 0.4 | ISO 8601 格式化/解析；时区用 `chrono-tz` 补充 IANA 库 |
| 高精度数值 | `bigdecimal` | 0.4 | 对齐 `BigDecimalEngine` 语义（加法 max scale、乘法 scale 相加、除法详见 §4） |
| 有序哈希 | `indexmap` | 2.x | 保留插入序的哈希（`?keys`/`?values` 语义） |
| 惰性静态 | `once_cell` 或 std `OnceLock` | — | 内建注册表、设置默认值 |
| 错误 | `thiserror` | 2.x | TemplateError 派生 |
| 缓存 | `lru` 或手写 MRU | — | MRU 语义（强/软双段）用 `Weak` + 手写链表；`lru` 仅当语义可接受 |
| 线程 | std `Arc`/`Mutex`/`RwLock` | — | 不引 tokio；渲染为同步 API |
| JSON | `serde_json` | 1.x | `?json_string`、`JSONCFormat`、`#ftl` JSON 相关 |
| Python | `pyo3` 0.29（仅 freemarker-pyo3） | 锁定 | 见 specs/2026-08-01-pyo3-design.md |

**不做**：Java 风格反射、Jython、`javax` 依赖、GIL 之外的 Python 线程模型。

## 4. 核心设计模式（本项目的三个关键决策）

### 4.1 角色槽位模型（TModel）—— 对应 Java 多接口实现

Java 的 `TemplateModel` 是根接口，具体对象可实现多个子接口（如 `JythonModel implements TemplateBooleanModel, TemplateScalarModel, TemplateHashModel, TemplateMethodModelEx, ...`）。Rust 无多态多接口，采用**角色槽位结构**：

```rust
/// 每个模型同时携带其实现的所有角色（Option 槽位）
pub struct TModel {
    pub scalar:   Option<Box<dyn TemplateScalarModel>>,
    pub number:   Option<Box<dyn TemplateNumberModel>>,
    pub boolean:  Option<Box<dyn TemplateBooleanModel>>,
    pub date:     Option<Box<dyn TemplateDateModel>>,
    pub sequence: Option<Box<dyn TemplateSequenceModel>>,
    pub hash:     Option<Box<dyn TemplateHashModel>>,     // 含 Ex/Ex2 标志
    pub method:   Option<Box<dyn TemplateMethodModel>>,
    pub directive:Option<Box<dyn TemplateDirectiveModel>>,
    pub node:     Option<Box<dyn TemplateNodeModel>>,
    pub collection: Option<Box<dyn TemplateCollectionModel>>,
    pub transform:Option<Box<dyn TemplateTransformModel>>,
    pub markup:   Option<Box<dyn TemplateMarkupOutputModel>>,
    pub py:       Option<Py<PyAny>>,      // 仅 feature="python" 启用
    // 元数据
    pub kind: ModelKind,                    // 快速分类（用于 ?is_* 内建与错误消息）
    pub class_name: &'static str,           // 错误消息中的类型名（对齐 getClass().getName()）
}
```

- **构造**：每个实现提供 `impl From<X> for TModel` 填充对应槽位。
- **判型**：`is_scalar()` 等辅助方法检查槽位（等价 `instanceof`）。
- **扩展**：用户自定义指令/方法模型只需实现对应 trait。
- **权衡**：每模型一次堆分配 + 每角色一次装箱；缓存后摊销为零。渲染热路径用 `kind` 枚举快速分派，避免 Option 链。

### 4.2 指令栈渲染模式（保留 Java 语义）

```rust
// Java: element.accept(env) -> TemplateElement[]（下一个要执行的指令）
pub trait TemplateElement {
    fn accept(&self, env: &mut Environment) -> Result<Vec<Element>, TemplateError>;
    // 错误时提供行号/列号/描述（对应 getBeginLine()/getEndColumn()/getDescription()）
}
```

- `Environment` 持有 `instruction_stack: Vec<Element>`（对应 `Environment.java:107`），`process()` 循环 pop 执行。
- 支持 `replace_top`（对应 `replaceTopElement` :414，`nested`/`return`/`stop` 依赖）。
- `visit(TemplateElement[])` 批量入栈 —— 内联优化版（`:367`）在 Rust 中用迭代器 + 直接循环实现，不做手动内联（编译器自动）。

### 4.3 错误模型

```rust
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    // 对应 freemarker.template.TemplateException 层级
    InvalidReference { name: String, ctx: ErrorContext },   // 未定义变量（含 ??/! 语义）
    TypeMismatch { expected: &'static str, actual: &'static str, ctx: ErrorContext }, // NonXxxException 族
    Parse(ParseError),
    Stop,                                      // stop 指令
    BreakOrContinue(FlowKind),                 // 流控违规
    Io(std::io::Error),
    Python(Box<PyErr>),                        // 仅 pyo3 feature
    Other { message: String, ctx: ErrorContext },
}
pub struct ErrorContext {                      // 对应 _ErrorDescriptionBuilder 输出
    pub template_name: String, pub line: u32, pub column: u32,
    pub instruction_stack: Vec<String>, pub message: String,
}
```

消息文本由 `error/messages.rs` 统一生成，逐字对齐 Java（见 specs/2026-08-01-error-handling-design.md）。

## 5. 与参考项目的一致性

- workspace 布局、`scripts/` 工具、文档风格对齐 `thymeleaf-rust`。
- 黄金测试 runner 设计对齐 `thymeleaf-rust/tests/`（数据驱动：`(name, ftl, data, expected)` 三元组）。
- 依赖基线检查复用 `scripts/check-dependency-baseline.py` 模式（根工作区 `dependency-baseline.toml`）。

---

## 对应计划

- `docs/superpowers/plans/2026-08-01-p0-skeleton-baseline.md`（Stage 1-3）
- `docs/superpowers/plans/2026-08-04-p6-polish-alignment.md`（文件级拆分）
