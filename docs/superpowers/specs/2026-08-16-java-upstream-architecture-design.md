# Java 上游深度架构报告（CodeGraph 实证）

- **日期**：2026-08-16
- **作者**：freemarker-rust 团队
- **状态**：已实施（迁移对照证据）
- **上游基线**：Apache FreeMarker v2.3.34（commit 7926e9771，2.3 分支线）
- **依赖**：CodeGraph 索引（949 文件 / 21,724 节点 / 60,814 边，tool 1.6.0）

---

> 证据来源：对 `/Users/wandl/workspaces/workspace-github/freemarker` 的两轮
> CodeGraph 深度探索（调用图 callers/callees/impact + 源码行号级核对）。
> 本文是 Java 基线侧的**机制级事实清单**，供 freemarker-rust 各迁移对照
> spec 引用（凡标注「Java 基线见本文 §N」处均可以本文为证据锚）。

## 1. 全仓量化视图

| 维度 | 数值 |
|------|------|
| 索引规模 | 949 文件 / 21,724 节点（9,106 方法 + 1,640 类 + 113 接口）/ 60,814 边 |
| 包分布 | core 282 · template 70 · cache 37 · ext/beans 75 · ext/dom 18 · log 12 · debug 16 · ext/xml 9 · ext/util 5 · ext/rhino 4 · ext/jdom 2 |
| Gradle 模块 | freemarker-core（主体）/ jython20（桥接实现）/ jython25（48 行适配器 + 黄金套件）/ javax-servlet / test-utils / core16 / core9 |
| 最大类 | Configuration 3,877 · Environment 3,709 · Configurable 3,414 · StringUtil 2,263 · BeansWrapper 2,074 · OverloadedNumberUtil 1,289 · BuiltInsForSequences 1,359 |
| 解析器 | FTL.jj 4,845 行（JavaCC 7.0.12）→ FMParser 为构建期生成产物，不入库 |
| 黄金套件 | testcases.xml **129 testCase** / 138 .ftl / 94 expected |
| 索引方法论 | tree-sitter AST 抽取 + 调用边解析；`codegraph query/callers/callees/impact` |

## 2. 渲染主链（第一轮已证，此处收口）

```
FreemarkerServlet.doGet → Template.process(:397) → Environment.process(:322)
  → doAutoImportsAndIncludes → visit
visit 的自证协作环（callees）：
  pushElement(:2902) → TemplateElement.accept(:89) → handleTemplateException(:1199) → popElement(:2919)
visit 的调用者：process（入口）/ invokeNestedContent(:606) /
  invokeMacroOrFunctionCommonPart(:848) / include(:3126) / 各 accept 递归
```

`Environment.visit` 影响半径仅 3 符号（impact 实测：getLocalVariableNames ×2）——
分派经 `accept` 多态完成，`visit` 本身是稳定骨架。

## 3. 内建函数分派机制（BuiltIn.java）

```java
abstract class BuiltIn extends Expression implements Cloneable   // :81
static final HashMap<String, BuiltIn> BUILT_INS_BY_NAME          // :89
putBI(name, bi)；putBI(name, nameSnakeCase, nameCamelCase, bi)   // :331-338 双命名别名
putBI("web_safe", "webSafe", BUILT_INS_BY_NAME.get("html"))      // :312 别名指向同一实例
throw new AssertionError("Update NUMBER_OF_BIS! ...")            // :326 编译期数量自检
static BuiltIn newBuiltIn(int incompatibleImprovements, Expression target, Token keyTk, ...)  // :349
```

三条机制：
1. **静态 HashMap 注册表**，`putBI` 登记snake_case + camelCase 双别名（2.3.23+ 命名约定）；
2. **`NUMBER_OF_BIS` 编译期断言**——注册数与常量不符即 AssertionError（防漏注册的编译期门禁）；
3. **`newBuiltIn` 工厂携带 ICI 参数**——BI 实例化时即绑定 incompatibleImprovements 语义。

**Rust 对照**：`builtins::lookup`（`builtins/mod.rs:47`）字符串 match 即此注册表的等价物；
仓库的「183/183 编译期全注册」惯例即 `NUMBER_OF_BIS` 断言的镜像。

## 4. 表达式求值链与变量解析

```
TemplateElement.accept → Expression.eval（每节点实现）
Identifier._eval (Identifier.java:36) → Environment.getVariable  ← 七级变量链唯一入口
BuiltinVariable.get (:329) → getVariable                          ← .version/.now/.locale 等
Environment.__getitem__ (:3394)                                    ← jython 桥复用变量链
```

变量链七级（Environment.getVariable :2460-2487）：
①局部上下文栈 → ②宏帧局部（getNullableLocalVariable :2426-2442）→ ③当前命名空间 →
④全局命名空间 → ⑤根数据模型 → ⑥共享变量（getDataModelOrSharedVariable :2568-2578）→
⑦未找到返回 null（错误在使用点抛，strict/classic 分流在使用点语义）。

**Rust 对照**：`environment.rs::get_variable` 七级顺序逐级对应（注释锚已核对）。

## 5. 设置解析的内嵌 DSL：`_ObjectBuilderSettingEvaluator`（1,121 行）

- 入口 `eval(String src, Class expectedClass, boolean allowNull, env)`（:103）→
  内部 `new ...eval()`（:106）递归求值；
- `SettingExpression.eval()`（:786 抽象 / :810 实现）——设置值是一套**完整表达式语言**
  （对象构造器语法、属性赋值 `propAssignments.eval()` :156）；
- 其自身复用 `Expression.eval`（callers 证据：_ObjectBuilderSettingEvaluator.java:786）。

**Rust 对照**：NA-DESIGN 的正确性证据——Rust 用户直接调用类型化 setter，无需复刻该 DSL；
`setSetting(String,String)` 通用入口因此一并 NA。

## 6. TemplateCache 内部（1,010 行）

```
getTemplate → getTemplateInternal(:323)
├─ TemplateKey：name/locale/customLookupCondition/encoding/parseAsFTL 五元组
├─ refresh delay 内直接返回缓存副本（:349）
├─ storeCached（:395/:446）
└─ storeNegativeLookup（:381/:505）  ← 负查找同样缓存（防反复 IO）
```

**Rust 对照**：三机制（TemplateKey/负查找/delay）均已实现——负查找条目返回
None、delay 内不验证（template_cache.rs:52/:67-71，注释锚 Java:350-365），
详见 `2026-08-16-rust-side-architecture-design.md` §7（2026-08-16 修正：原表述
「负查找未覆盖」有误，以 Rust 侧报告为准）；仍为 NA 的仅 CacheStorage
容量/淘汰策略（MRU/Soft，结构对照 spec §4）。

## 7. 格式化工厂继承树

```
TemplateDateFormatFactory（抽象）
├── ISOLikeTemplateDateFormatFactory（抽象）→ ISO/XS 系列
├── JavaTemplateDateFormatFactory           → Java 模式语法（yyyy-MM-dd）
└── AliasTemplateDateFormatFactory（final） → 别名工厂（custom_formats 语法糖）
Number 侧完全对称（JavaTemplateNumberFormatFactory / Alias / ...）
```

- Configurable 持有 `customDateFormats`/`customNumberFormats`（:406-407）；
- 带 `WithoutFallback` 双访问器（:1308/:1318），且 Javadoc 三处警告
  「get 返回值不反映 Map 键粒度回退」（:934/:1295/:2023）。

**Rust 对照**：`builtins/iso_date_format.rs` + `java_date_format.rs` 聚合承载全部角色；
Alias 工厂语义 ↔ `@name` 自定义格式（报错路径已实现——结构对照 spec §3 #4；
注册表属 P3 缺口）。预定义格式名 `currency`/`percent` 已按 Java 实测基线实现
（commit 9a10174，测试 `currency_percent_java_baseline`，5 locale）。

## 8. 错误装配链（两段式，异常精确抛出点）

- **类型强制侧**：`EvalUtil.coerceModelToStringOrMarkup(:388) → coerceModelToTextualCommon`
  ——插值输出前的最后类型收敛点，类型不匹配的 `Non*Exception` 由此抛出；
- **消费侧精确点**（callers 实证）：
  - `NonSequenceException` 仅两处构造——`BuiltInForSequence._eval(:28)`、
    `RecurseNode.accept(:46)`;
  - 即「序列内建吃到非序列」「recurse 吃到非节点」两个确切语义点。

**结论**：每个 Non* 异常都有单点/双点精确抛出位置——这是 Rust 侧 70 场景
错误消息逐字对齐能在 460 调用点零歧义落位的结构性原因（error/expected_messages/）。

## 9. attempt/recover 链

`AttemptBlock extends TemplateElement`（final，AttemptBlock.java:29）
配合 `Environment.handleTemplateException(:1199)`：

```
accept 内抛 → handleTemplateException → 最近 AttemptBlock → recover 子树执行
```

**Rust 对照**：`attempt_block.rs` + `consume_outcome` 的 `RunSignal` 分流（environment.rs:900-918）。

## 10. 其他机制级事实（速查）

| 机制 | 证据 | Rust 对照 |
|------|------|----------|
| 特殊变量 `.version`/`.now` | BuiltinVariable.java:66/:78 常量表 | BuiltinVar 枚举变体 |
| Java string.trim 语义（≤U+0020） | StringUtil.java（utility/string_util.rs java_trim 注释锚） | `java_trim` |
| jython25 本体 | `_Jython25VersionAdapter.java` 全模块仅 48 行 | pyo3 完整桥接在 jython20 的 ext/jython 13 文件语义 |
| 组合格式 | CombinedMarkupOutputFormat.java:63-65/:78-80 | combined_markup_output_format.rs |
| StatefulTemplateLoader | Java instanceof 可选接口 | `TemplateLoader::reset_state` 默认钩子（结构对照 spec §3 #8） |
| 线程中断 | ThreadInterruptionSupportTemplatePostProcessor 遍历 AST 注入检查元素 | no-op + tokio CancellationToken 建议（差异已文档化） |
| DOCTYPE | DocumentTypeModel.getNodeName = "@document_type$"+name；children/get 抛 "not currently supported" | 自扫声明降级实现（e57dcfd，7 测试） |

## 11. 引用本文的对照锚

凡迁移对照需要 Java 基线证据处，引用格式：`Java 基线见本 spec §N`。
已有对照落点：
- `specs/2026-08-04-java-rust-structure-mapping-design.md`（412 MAPPED 清单）
- `specs/2026-08-01-rendering-engine-design.md`（渲染循环伪代码的 Java 行号锚）
- `specs/2026-08-02-builtins-design.md`（183 BI 清单与特化基类）
- `specs/2026-08-01-error-handling-design.md`（70 场景 parity）
- `docs/user-guide.md` §差异矩阵（面向用户的 10 条边界）

---

## 对应计划

- `docs/superpowers/plans/2026-08-15-production-readiness.md`（生产就绪 Stage 0-5，
  其中 Stage 1 golden NA 复核引用本文 §2-9 的机制级证据）
- `docs/superpowers/plans/2026-08-14-layout-parity-migration.md`（布局对齐轮）
