# freemarker-rust 用户迁移指南

> 面向 Java FreeMarker 用户的 Rust 迁移指南。本文档覆盖快速开始、兼容性总览、
> 差异矩阵、数据模型迁移、配置项对照、错误处理迁移、已知限制与路线。

---

## 1. 快速开始

### Rust 侧

```rust
use std::rc::Rc;
use freemarker::parser::parse;
use freemarker::template::{Configuration, TModel};
use indexmap::IndexMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Rc::new(Configuration::new());
    let tpl = parse(&cfg, "hello", "Hello, ${name}!")?;
    let mut root = IndexMap::new();
    root.insert("name".to_string(), TModel::from_scalar("World".to_string()));
    let mut out: Vec<u8> = Vec::new();
    tpl.process(TModel::from_hash(root), &mut out)?;
    println!("{}", String::from_utf8(out)?);
    Ok(())
}
```

### Python 侧（freemarker-pyo3）

```python
from freemarker import FmConfiguration, FmTemplate

cfg = FmConfiguration()
tpl = FmTemplate("hello", "Hello, ${name}!")
result = tpl.process({"name": "World"})
print(result)  # "Hello, World!"
```

> **安装**：`pip install freemarker-pyo3`（0.1.0b0）

---

## 2. 兼容性总览

行为权威为 Apache FreeMarker `2.3-gae@7926e97`（`incompatible_improvements = 2.3.34`）。

| 维度 | 当前状态 | 说明 |
|------|----------|------|
| golden 套件 | **113/128** 逐字节一致 | 0 FAIL / 0 BLOCKED，15 项有据永久 NA |
| 内建函数 | **183/183**（100%） | Java 2.3.34 全集覆盖 |
| 永久 NA | 15 项 | 反射系 12 项 + JythonRuntime 1 项 + 套件矛盾 2 项 |
| java_ported 测试 | 502/502 PASS | 7 ignored（引擎缺口已记录） |
| 公共 API 基线 | 0 diff | `docs/release/api-baseline.txt` 锁定 |

---

## 3. 差异矩阵

以下 10 条是 Java FreeMarker 与 Rust 版之间的已知差异。每条给出 Java 行为、
Rust 行为和迁移建议。

| # | 差异点 | Java 行为 | Rust 行为 | 迁移建议 |
|---|--------|-----------|-----------|----------|
| 1 | **JVM 反射** | `BeansWrapper` / `DefaultObjectWrapper` 通过反射访问 POJO 属性 | 不可用；数据模型必须显式构造 | 用 `TModel::from_*` 构造器或 `SimpleObjectWrapper` + `DynValue` 显式包装 |
| 2 | **错误模型** | 受检异常（`TemplateException` 层级） | `Result<T, TemplateError>` 无受检异常 | 用 `match` / `?` 运算符处理；错误消息含模板名/行/列/FTL stack trace |
| 3 | **CacheStorage** | 可选 `MruCacheStorage` / 容量 / 过期策略 | 固定 `HashMap`，无 MRU/容量/过期 | 暂无替代；1.0 前评估 LRU 扩展 |
| 4 | **DOCTYPE 降级** | `@document_type$name` 元数据可访问，子节点/属性报 "not currently supported" | 同 Java 行为（逐字对齐） | 无需迁移；行为一致 |
| 5 | **线程中断** | `ThreadInterruption` 后处理检查 `Thread.isInterrupted()` | Rust 无线程中断；后处理为 no-op | 异步场景用 `tokio::CancellationToken` 手动取消 |
| 6 | **正则引擎** | `java.util.regex`（不支持反向引用/环视） | `fancy-regex`（支持反向引用/环视） | 大多数模板无需改动；Java 特有 flag 子集由 `RegexpHelper` 静默忽略 |
| 7 | **日志框架** | SLF4J 路径（`LoggerFactory.getLogger`） | 无日志框架；模板异常静默策略经 `template_exception_handler` 设置 | 生产环境建议接 `tracing` crate；调试设 `template_exception_handler = "debug"` |
| 8 | **ICI 版本** | 可选 `2.3.0` ~ `2.3.34` 任意版本 | 固定 `2.3.34`；不提供旧版本行为开关（`classic_compatible` 有） | 迁移前确保 Java 侧 ICI 已升级到 2.3.34 |
| 9 | **输出格式** | `HTMLOutputFormat` / `XMLOutputFormat` 等类 | 枚举 `OutputFormatKind`（HTML/XML/XHTML/JavaScript/JSON/CSS/RTF/PlainText/Combined） | 用 `settings.output_format` 设置；行为一致 |
| 10 | **模板后处理** | `TemplatePostProcessor` 接口 | `TemplatePostProcessor` trait（AST 变换钩子） | 实现 trait 并通过 `cfg.add_template_post_processor()` 注册 |

---

## 4. 数据模型迁移

### Java 侧（BeansWrapper 自动包装）

```java
// Java: BeansWrapper 自动将 POJO 包装为 TemplateHashModel
Map<String, Object> root = new HashMap<>();
root.put("user", userObject);  // POJO 属性自动暴露
root.put("items", list);
Template tpl = cfg.getTemplate("page.ftl");
tpl.process(root, writer);
```

### Rust 侧（显式构造 TModel）

```rust
use indexmap::IndexMap;
use freemarker::template::TModel;
use freemarker::value::TNumber;

// Rust: 每个值必须显式构造为 TModel
let mut user = IndexMap::new();
user.insert("name".to_string(), TModel::from_scalar("Alice".to_string()));
user.insert("age".to_string(), TModel::from_number(TNumber::Int(30)));

let mut root = IndexMap::new();
root.insert("user".to_string(), TModel::from_hash(user));
root.insert("items".to_string(), TModel::from_sequence(vec![
    TModel::from_scalar("Rust".to_string()),
    TModel::from_scalar("FreeMarker".to_string()),
]));

let mut out = Vec::new();
tpl.process(TModel::from_hash(root), &mut out)?;
```

### 通过 DynValue 简化动态数据

对于来自 JSON / 请求体的动态数据，可用 `DynValue` + `SimpleObjectWrapper`：

```rust
use freemarker::template::{DynValue, ObjectWrapper, SimpleObjectWrapper};

let payload = DynValue::Map(vec![
    ("name".to_string(), DynValue::Str("Alice".to_string())),
    ("age".to_string(), DynValue::Num(30.0)),
]);
let root = SimpleObjectWrapper.wrap(&payload)?.unwrap_or_else(TModel::nothing);
```

### TModel 构造器速查

| 构造器 | Java 等价 | 用途 |
|--------|-----------|------|
| `TModel::from_scalar(s)` | `SimpleScalar` | 字符串 |
| `TModel::from_number(TNumber::Int(i))` | `SimpleNumber(int)` | 整数 |
| `TModel::from_number(TNumber::Decimal(d))` | `SimpleNumber(BigDecimal)` | 高精度小数 |
| `TModel::from_boolean(b)` | `SimpleBoolean` | 布尔 |
| `TModel::from_date(DateValue::new(...))` | `SimpleDate` | 日期/时间 |
| `TModel::from_sequence(vec![...])` | `SimpleSequence` | 有序列表（可重复访问） |
| `TModel::from_collection(vec![...])` | `SimpleCollection` | 一次性集合 |
| `TModel::from_hash(indexmap)` | `SimpleHash` | 键值对（保留插入序） |
| `TModel::from_method(m)` | `TemplateMethodModelEx` | 宿主函数 |
| `TModel::from_directive(d)` | `TemplateDirectiveModel` | 自定义指令 |
| `TModel::from_xml_str(xml)` | `NodeModel.parse(source)` | XML 文档节点 |

---

## 5. 配置项对照

### Settings 字段（26 项）

| Settings 字段 | Java 对应 | 默认值 | 说明 |
|---------------|-----------|--------|------|
| `locale` | `Configuration.setLocale` | `"en_US"` | 日期/数字格式化 locale |
| `time_zone` | `Configuration.setTimeZone` | GMT | 接受 IANA 名或 `GMT±HH` |
| `time_zone_id` | `TimeZone.getID()` | `"GMT+00:00"` | `.time_zone` 内置变量读数 |
| `number_format` | `Configuration.setNumberFormat` | `"number"` | 数字格式模式 |
| `boolean_format` | `Configuration.setBooleanFormat` | `"true,false"` | 布尔格式（逗号分隔） |
| `date_format` | `Configuration.setDateFormat` | — | 日期格式模式 |
| `time_format` | `Configuration.setTimeFormat` | — | 时间格式模式 |
| `date_time_format` | `Configuration.setDateTimeFormat` | — | 日期时间格式模式 |
| `output_format` | `Configuration.setOutputFormat` | `PlainText` | 输出格式枚举 |
| `auto_escaping` | `Configuration.setAutoEscaping` | `Default` | 自动转义模式 |
| `c_format` | `Configuration.setCFormat` | `JavaScript or JSON` | C 格式变体 |
| `whitespace_stripping` | `Configuration.setWhitespaceStripping` | `true` | 空白剥离 |
| `strict_syntax` | `Configuration.setStrictSyntax` | `false` | 严格语法模式 |
| `classic_compatible` | `Configuration.setClassicCompatible` | `false` | 经典兼容模式 |
| `incompatible_improvements` | `Configuration(Version)` | `2.3.34` | ICI 版本 |
| `output_encoding` | `Configuration.setOutputEncoding` | `"UTF-8"` | 输出编码 |
| `url_escaping_charset` | `Configuration.setURLEscapingCharset` | `"UTF-8"` | URL 转义字符集 |
| `fallback_on_null_loop_variable` | `Configurable.setFallbackOnNullLoopVariable` | `false` | 循环变量 null 回退 |
| `delay` | `Configuration.setTemplateUpdateDelay` | `1` | 模板更新延迟（秒） |
| `localized_lookup` | `Configuration.setLocalizedLookup` | `true` | 局部化模板查找 |
| `lookup_strategy` | `Configuration.setTemplateLookupStrategy` | `Default020300` | 查找策略 |
| `input_encoding` | `Configuration.setDefaultEncoding` | `None`（UTF-8） | 输入编码 |
| `template_exception_handler` | `Configuration.setTemplateExceptionHandler` | `"rethrow"` | 异常处理器 |
| `new_builtin_class_resolver` | `Configuration.setNewBuiltinClassResolver` | `Unrestricted` | `?new` 类解析器 |
| `lazy_imports` | `Configuration.setLazyImports` | `false` | 延迟导入 |
| `lazy_auto_imports` | `Configuration.setLazyAutoImports` | `None` | 延迟自动导入 |

### Configuration 其他方法

| 方法 | Java 对应 |
|------|-----------|
| `cfg.set_shared_variable(name, model)` | `Configuration.setSharedVariable` |
| `cfg.add_auto_import(ns, path)` | `Configuration.addAutoImport` |
| `cfg.add_auto_include(path)` | `Configuration.addAutoInclude` |
| `cfg.clear_template_cache()` | `Configuration.clearTemplateCache` |
| `cfg.get_template(name)` | `Configuration.getTemplate` |
| `cfg.get_template_localized(name, locale)` | `Configuration.getTemplate(name, locale, ...)` |
| `cfg.get_template_encoded(name, encoding)` | `Configuration.getTemplate(name, null, null, encoding, ...)` |

### Python 侧（FmConfiguration 35 方法）

Python 绑定 `FmConfiguration` 暴露 35 个方法，覆盖上述所有常用设置。
详见 `freemarker-pyo3/src/lib.rs` 或 `pip install freemarker-pyo3` 后 `help(FmConfiguration)`。

---

## 6. 错误处理迁移

### Java 侧

```java
try {
    template.process(dataModel, writer);
} catch (TemplateException e) {
    System.err.println("Template error: " + e.getMessage());
    System.err.println("  in " + e.getTemplateName()
        + " at line " + e.getLineNumber());
}
```

### Rust 侧

```rust
use freemarker::error::TemplateError;

match tpl.process(root, &mut out) {
    Ok(()) => { /* 成功 */ }
    Err(e) => {
        // TemplateError 的 Display 输出包含模板名/行/列/FTL stack trace
        eprintln!("Template error: {e}");

        // 按变体匹配提取结构化信息
        match &e {
            TemplateError::Parse { template, line, col, message } => {
                eprintln!("  解析错误: 模板 {:?} 第 {} 行第 {} 列", template, line, col);
            }
            TemplateError::InvalidReference { name, .. } => {
                eprintln!("  变量缺失: {}", name);
            }
            TemplateError::TypeMismatch { expected, actual, .. } => {
                eprintln!("  类型不匹配: 期望 {}，实际 {}", expected, actual);
            }
            TemplateError::NotFound { name } => {
                eprintln!("  模板未找到: {}", name);
            }
            _ => eprintln!("  其他错误: {e}"),
        }
    }
}
```

### 错误变体速查

| 变体 | Java 等价 | 触发场景 |
|------|-----------|----------|
| `Parse` | `ParseException` | FTL 语法错误 |
| `InvalidReference` | `InvalidReferenceException` | 变量缺失 |
| `TypeMismatch` | `UnexpectedTypeException` | 类型不匹配 |
| `Misc` | `_MiscTemplateException` | 通用运行时错误 |
| `NotFound` | `TemplateNotFoundException` | 模板加载失败 |
| `Stop` | `StopException` | `<#stop>` 指令 |
| `Model` | `TemplateModelException` | 数据模型层错误 |
| `Io` | `IOException` | I/O 写入失败 |

---

## 7. 已知限制与路线

### 当前限制（0.1.0）

| 限制 | 影响 | 路线 |
|------|------|------|
| `Configuration` 基于 `Rc`，非 `Send`/`Sync` | 不能跨线程共享同一实例 | 每个 worker 线程 clone 一份；1.0 前评估 `Arc` 方案 |
| 无 MRU/容量/过期缓存策略 | 长期运行可能内存增长 | 1.0 前评估 LRU 扩展 |
| 无 SLF4J 日志集成 | 调试信息通过 `template_exception_handler` 控制 | 建议应用层接 `tracing` crate |
| `?api` 视图需手动构造 | Java BeansWrapper 自动暴露方法 | 已实现 `TemplateApiSupport` trait，包装方提供视图 |
| WASM target 尚未声明 | 不能在浏览器/边缘环境运行 | 解析器对 `no_std` 友好，待配置 |

### 版本路线

| 版本 | 预计时间 | 关键里程碑 |
|------|----------|-----------|
| `0.1.0-beta.0` | 2026-08-15（已 tag） | 功能冻结、稳定性验证、文档收口 |
| `0.1.0` | 2026-08 下旬 ~ 2026-09 | 首个功能完整版；crates.io + PyPI 发布 |
| `1.0.0` | 2026-09 ~ 2026-10 | SemVer 承诺生效；公共 API 稳定 |

详细路线图见 [`superpowers/VERSION-PLAN.md`](superpowers/VERSION-PLAN.md)。

---

## 附录：深入学习入口

| 文档 | 内容 |
|------|------|
| [`specs/2026-08-01-architecture-design.md`](superpowers/specs/2026-08-01-architecture-design.md) | 整体架构与模块边界 |
| [`specs/2026-08-02-builtins-design.md`](superpowers/specs/2026-08-02-builtins-design.md) | 183 内建函数兼容矩阵 |
| [`specs/2026-08-01-data-model-design.md`](superpowers/specs/2026-08-01-data-model-design.md) | TModel 数据模型设计 |
| [`specs/2026-08-03-security-model-design.md`](superpowers/specs/2026-08-03-security-model-design.md) | 安全模型与受限子集 |
| [`specs/2026-08-03-migration-parity-ledger-design.md`](superpowers/specs/2026-08-03-migration-parity-ledger-design.md) | 128 fixture 逐项 disposition |
| [`specs/2026-08-04-java-rust-structure-mapping-design.md`](superpowers/specs/2026-08-04-java-rust-structure-mapping-design.md) | Java→Rust 结构对照（412 MAPPED） |
| [`api-stability.md`](api-stability.md) | API 稳定性承诺 |
| [examples/](../freemarker/examples/) | 7 个可运行示例（克隆仓库后 `cargo run --example <name>`） |
