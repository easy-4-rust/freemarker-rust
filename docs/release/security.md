# 安全模型

> 本文件定义 freemarker-rust 的安全威胁模型、暴露面与限制（**受限子集**）。
> 与 thymeleaf-rust 同口径，但 freemarker-rust 明确**不实现** JVM 反射能力。

## 1. 核心原则

freemarker-rust 是一款**模板渲染引擎**，处理**用户提供的模板文本**并对**用户提供的数据模型**求值。
两个面都来自不可信源（web 端上传的模板、HTTP 请求数据），因此威胁模型围绕"恶意输入触发非预期行为"展开。

## 2. 暴露面

| 面 | 入口 | 风险面 |
|---|---|---|
| 模板解析 | `parser::parse(cfg, name, text)` | 输入长度 + 嵌套深度 → 拒绝服务（CPU/栈） |
| 表达式求值 | `core::eval::eval(env, expr)` | 无限循环（Java `?number`/`?date` 内部 while） |
| 模板渲染 | `Template::process(root, out)` | 输出体积 → 内存（无显式限制） |
| 模板数据模型 | `ObjectWrapper::wrap/unwrap` | 类型强制 / 反射泄漏 |
| 内建 `?new` | `utility_transforms::new_utility_class` | 任意类实例化（已硬编码 6 类白名单） |
| 模板加载 | `cache::TemplateCache` | 路径遍历（由 `TemplateLoader` 链负责） |
| 编码 | `transcode_output` | 编码错误处理（`?` 替换） |

## 3. 受限子集（决策 1：JVM 反射不实现）

freemarker-rust **不实现** Java `freemarker.ext.beans.BeansWrapper` / `ClassIntrospector` /
`MemberAccessPolicy` / `OverloadedMethods` / `MemberAccessPolicy` 体系。
此决策导致以下**不可恢复**的能力缺失（永久 NOT_APPLICABLE，共 33 项）：

### 3.1 POJO 反射缺失（25 项）

Java `BeansWrapper` 通过 `java.beans.Introspector` + `java.lang.reflect` 反射包装任意
Java POJO。Rust 无 JVM 等价物，包装路径仅覆盖 `DynValue` 8 种变体（Null/Str/Int/
Float/Bool/Date/Map/List）。

**影响**：
- 模板中 `user.name` / `user.getFullName()` / `user.children[0].name` 等任意 POJO 访问
  不可用；
- 模板中 `?api` 内建恒错误（`freemarker-rust/freemarker/src/builtins/multi.rs:303`）；
- `?new <FQN>()` 仅支持 6 个硬编码类（`StandardCompress`/`NormalizeNewlines`/
  `HtmlEscape`/`ObjectConstructor`/`NewTestModel`/`SimpleTestMethod`），其余返回
  `ClassNotFoundException`；
- 重载方法分派（OverloadedMethods）不存在，`TemplateMethodModelEx::exec(args)`
  单一分派。

**用户替代路径**：在 Rust 侧手工包装 POJO 为 `DynValue::Map`（含字段）或 `DynValue::List`，
或实现自定义 `TemplateMethodModelEx` 暴露受控 API。

### 3.2 `?api` 内建（2 项）

`?api` 依赖 `TemplateModelWithAPISupport` + `BeanModel.getAPI()`（Java 反射元数据），
Rust 无该能力。

### 3.3 `?new` 类解析器（4 项）

Java `?new` 通过 `TemplateClassResolver` + `Class.forName` + `SecurityManager` 任意类
构造；Rust 仅支持 6 个硬编码类。

### 3.4 XML 节点（2 项）

`xml-fragment` / `xmlns2` / `xml-ns_prefix-scope` 等需 `freemarker.ext.dom.NodeModel` +
`org.w3c.dom` + `Jaxen XPath`；Rust 有 `roxmltree` 替代但未实现 NodeModel 适配。

## 4. 已实施的控制

| 控制 | 状态 | 引用 |
|---|---|---|
| 表达式 `?new` 白名单（6 类） | ✅ | `freemarker/src/template/utility_transforms.rs:38-77` |
| `ObjectWrapper.unwrap` 对 Method/Directive/TransformModel 拒绝 | ✅ | `freemarker/src/template/simple_object_wrapper.rs:67-124` |
| 0 `unsafe` 块 | ✅ | `grep -rn 'unsafe {' freemarker/src/` 0 命中 |
| `unsafe_code = "forbid"` 全 crate | ✅ | `Cargo.toml` `workspace.lints`（待统一） |
| `cargo deny` 许可/来源/重复门禁 | ✅ | `deny.toml` + CI |
| `cargo audit` 漏洞扫描 | ✅ | `.cargo/audit.toml` + CI |
| proptest 解析器/渲染 fuzz | ✅ | `freemarker-test/tests/robustness_fuzz_smoke.rs` |
| `cargo public-api` 基线 diff | ✅ | `docs/release/api-baseline.txt` + CI |
| 多 OS 矩阵（ubuntu/macos/windows + MSRV 1.85） | ✅ | `.github/workflows/ci.yml` |

## 5. 剩余风险与未实现

| 风险 | 状态 | 后续 |
|---|---|---|
| 模板无限循环 / 递归 | 未实现层数/指令深度限制 | 阶段 3 续作：可加 `max_template_depth` 设置 |
| 模板体积 | 未限制输出字节上限 | 阶段 3 续作：可加 `max_output_bytes` |
| 编码错误边界 | UTF-16/ISO-8859-1 输出未映射码点替换为 `?`（与 Java `OutputStreamWriter` 一致） | 已对齐，无新风险 |
| Servlet/JSP 集成 | 不在 v1 范围 | 后续阶段（用户提供 HTTP 框架适配） |
| Jython 集成 | v1 通过 `freemarker-pyo3` 替代，PyO3 0.29 | 持续维护 |

## 6. 与 thymeleaf-rust 安全模型的差异

| 维度 | thymeleaf-rust | freemarker-rust |
|---|---|---|
| JVM 反射 | 不需要（Thymeleaf 表达式走 OGNL 子集，由白名单 + 类型 ACL 控制） | **缺失**（Freemarker `BeansWrapper` 不可 1:1 实现） |
| 表达式安全 | `restrict_external_access` + `forbid_unsafe_expression_results` + 类 ACL | `?new` 白名单（6 类） + `ObjectWrapper.unwrap` 类型拒绝 |
| `unsafe` | 全 forbid，0 块 | 全 forbid，0 块 |
| Fuzz | proptest 解析器 + 表达式 + 渲染 smoke | proptest 解析器 + 渲染 smoke（无表达式分派） |
| 漏洞治理 | cargo-deny + cargo-audit + 10 项 RUSTSEC 豁免 | cargo-deny + cargo-audit（**当前 0 项豁免**，因 audit 0 error） |

## 7. 决策记录

- 2026-08-03：决策 1（接受限制，文档明记）—— freemarker-rust = 受限子集，POJO 反射不实现；
  33 项 NOT_APPLICABLE 永久保留。
- 2026-08-03：决策 2（pyo3 不发布）—— freemarker-pyo3 `publish=true` 但本轮不演练 PyPI/maturin 发布。
- 2026-08-03：决策 3（全修 BLOCKED）—— 阶段 B 修复 4 项工程量缺口（output-encoding2/3、
  number-literal、bean-maps）+ 1 项文档同步（identifier-escaping）+ 1 项 NOT_APPLICABLE
  登记（transforms）。