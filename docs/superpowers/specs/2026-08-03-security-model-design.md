# 安全模型设计

- **日期**：2026-08-03
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97，2.3 分支线）
- **依赖**：无外部依赖

---

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
| 自动 include/import | `Environment` 三层（Configuration/Environment） | 配置驱动的模板加载（与 Java 同语义；模板文本不可篡改配置） |
| markup 双槽模型 | `TModel.markup_format/markup_plain` | 插值/`?esc` 跨格式重转义（Java DollarVariable/CommonMarkupOutputFormat 同语义） |
| 组合输出格式 | `?c "HTML+XML"` | 逐层转义（纯转义组合，无注入面） |

## 3. 受限子集（决策 1：JVM 反射不实现）

freemarker-rust **不实现** Java `freemarker.ext.beans.BeansWrapper` / `ClassIntrospector` /
`MemberAccessPolicy` / `OverloadedMethods` 体系。
此决策导致以下**不可恢复**的能力缺失（永久 NOT_APPLICABLE，共 15 项，分类确定化：
golden.rs `permanent_na_reason` + docs/superpowers/specs/2026-08-03-acceptance-report-design.md v13）：

### 3.1 POJO 反射缺失（12 项）

Java `BeansWrapper` 通过 `java.beans.Introspector` + `java.lang.reflect` 反射包装任意
Java POJO。Rust 无 JVM 等价物，包装路径仅覆盖 `DynValue` 8 种变体（Null/Str/Int/
Float/Bool/Date/Map/List）。

**影响**：
- 模板中 `user.name` / `user.getFullName()` / `user.children[0].name` 等任意 POJO 访问
  不可用（`beans` 1 项）；
- 重载方法分派（OverloadedMethods）不存在，`TemplateMethodModelEx::exec(args)`
  单一分派（`overloaded-methods-*` 11 项）；
- `?new <FQN>()` 不能反射任意 Java 类（仅引擎内置类查表，见 3.3）。

**用户替代路径**：在 Rust 侧手工包装 POJO 为 `DynValue::Map`（含字段）或 `DynValue::List`，
或实现自定义 `TemplateMethodModelEx` 暴露受控 API。

### 3.2 套件自身问题（3 项）

- `transforms`：Java 特有变换类 `JythonRuntime`（ClassNotFoundException），引擎无 Jython；
- `string-builtins3` / `date-type-builtins`：jython25 弃用套件的过期断言，与真实
  Java 2.3.34 行为矛盾（jar 实测用例本身无法通过）。

### 3.3 `?new` 类解析（已实现，受限）

Java `?new` 通过 `TemplateClassResolver` + `Class.forName` 任意类构造；Rust 已实现
`NewBuiltinClassResolver` 四策略（`freemarker/src/core/template_class_resolver.rs`，
默认 `Unrestricted` 与 Java Configurable.java:477 一致）：
- `unrestricted`：内置类查表（utility 变换类 / ObjectConstructor / 测试夹具），
  未注册类名按 ClassNotFoundException 语义报错；
- `safer` / `allows_nothing`：逐级收紧的默认拒绝；
- `opt-in`：`allowed_classes` 白名单 + `trusted_templates` 路径匹配
  （对应 Java `OptInTemplateClassResolver`）。

### 3.4 `?api`（已实现，无反射攻击面）

`?api`/`?has_api` 已实现（`TemplateApiSupport` trait + `TModel.api` 槽位，
BuiltInsForMultipleTypes.java:226-250 语义）。API 视图由**包装方**（数据模型构造者）
直接提供视图模型，引擎自身无反射能力——模板侧无法通过 `?api` 触达任意对象方法。

### 3.5 XML 节点（已实现）

`freemarker.ext.dom.NodeModel` 语义已由内置 XmlNode 模型 + xpath_subset 子集
（`./` 相对路径、`true()`、`[0]` 索引、visit 前缀宏分派）覆盖（golden XML 系列全部
PASS）；roxmltree 解析不加载外部实体（无 XXE 面）。

## 4. 已实施的控制

| 控制 | 状态 | 引用 |
|---|---|---|
| `?new` 四策略解析器（unrestricted/safer/allows_nothing/opt-in + trusted_templates；默认 Unrestricted 与 Java 一致） | ✅ | `freemarker/src/core/template_class_resolver.rs` |
| `?api` API 视图由包装方提供（引擎无反射，无反射攻击面） | ✅ | `TModel.api` 槽位 + `TemplateApiSupport`（`freemarker/src/template/`） |
| XML 解析（roxmltree，不加载外部实体） | ✅ | `freemarker/src/xml/mod.rs` |
| `ObjectWrapper.unwrap` 对 Method/Directive/TransformModel 拒绝 | ✅ | `freemarker/src/template/simple_object_wrapper.rs:67-124` |
| 0 `unsafe` 块 | ✅ | `grep -rn 'unsafe {' freemarker/src/` 0 命中 |
| `unsafe_code = "forbid"` 全 crate | ✅ | `Cargo.toml` `workspace.lints`（待统一） |
| `cargo deny` 许可/来源/重复门禁 | ✅ | `deny.toml` + CI |
| `cargo audit` 漏洞扫描 | ✅ | `.cargo/audit.toml` + CI |
| proptest 解析器/渲染 fuzz | ✅ | `freemarker-test/tests/robustness_fuzz_smoke.rs` |
| `cargo public-api` 基线 diff | ✅ | `docs/release/api-baseline.txt` + CI |
| 多 OS 矩阵（ubuntu/macos/windows + MSRV 1.85） | ✅ | `.github/workflows/ci.yml` |
| auto include/import 仅从配置读取（模板文本不可控制加载列表；与 Java 一致） | ✅ | `configurable.rs` + `environment.rs` |
| markup 捕获/跨格式转义与 Java 逐字节对齐（972 测试锁定） | ✅ | `block_assignment.rs`/`apply_escape`/`markup_outputs.rs` |
| `?new` 设置解析剥引号（SettingStringParser 语义；白名单不变） | ✅ | `template_class_resolver.rs` |

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
| 表达式安全 | `restrict_external_access` + `forbid_unsafe_expression_results` + 类 ACL | `?new` 四策略解析器 + `?api` 视图由包装方提供 + `ObjectWrapper.unwrap` 类型拒绝 |
| `unsafe` | 全 forbid，0 块 | 全 forbid，0 块 |
| Fuzz | proptest 解析器 + 表达式 + 渲染 smoke | proptest 解析器 + 表达式 10000 用例 + 渲染 smoke |
| 漏洞治理 | cargo-deny + cargo-audit + 10 项 RUSTSEC 豁免 | cargo-deny + cargo-audit（**当前 0 项豁免**，因 audit 0 error） |

## 7. 决策记录

- 2026-08-03：决策 1（接受限制，文档明记）—— freemarker-rust = 受限子集，POJO 反射
  不实现；**15 项** NOT_APPLICABLE 永久保留（12 JVM 反射 + 1 transforms + 2 jython25
  过期断言），分类确定化（golden.rs `permanent_na_reason`）。
- 2026-08-03：决策 2（用户确认）—— BeansWrapper 方法重载 11 项标记**永久 NA**，
  不实现反射方法分派（`TemplateMethodModelEx` 单一分派为边界）。
- 2026-08-03：决策 3（用户确认）—— `?api` 以**包装方提供视图**方式实现（非反射）；
  `?new` 以四策略解析器实现（默认与 Java 一致 Unrestricted）。
- 2026-08-03：决策 2（pyo3 不发布）—— freemarker-pyo3 `publish=true` 但本轮不演练 PyPI/maturin 发布。
- 2026-08-03：决策 3（全修 BLOCKED）—— 阶段 B 修复 4 项工程量缺口（output-encoding2/3、
  number-literal、bean-maps）+ 1 项文档同步（identifier-escaping）+ 1 项 NOT_APPLICABLE
  登记（transforms）。
- 2026-08-04：复查（1.0 晋级条件 8）—— 新增语义安全评审通过：auto include/import
  分层仅配置驱动、markup 双槽与组合格式为纯转义路径、`?new` 剥引号不改白名单；
  无新注入面，0 unsafe 保持。覆盖率审计见 docs/superpowers/specs/2026-08-04-coverage-audit-design.md。

---

## 对应计划

- `docs/superpowers/plans/2026-08-03-alpha0-production-readiness.md`（生产就绪）
