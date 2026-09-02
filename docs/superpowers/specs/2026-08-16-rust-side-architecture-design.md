# Rust 实现深度架构报告（CodeGraph 实证）

- **日期**：2026-08-16
- **作者**：freemarker-rust 团队
- **状态**：已实施（迁移对照证据）
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97）；本仓库 dev `1b3ed00`（布局迁移 + 生产就绪 Stage 0-5 后）
- **依赖**：CodeGraph 索引（656 文件 / 6,457 节点 / 25,116 边，tool 1.6.0）
- **配对文档**：`2026-08-16-java-upstream-architecture-design.md`（Java 基线侧；本文 §N 与其 §N 主题一一对应）

---

> 证据来源：对 `freemarker-rust` 的 CodeGraph 全量扫描（布局迁移 291→472 文件
> 后首次；调用图 callers/callees/impact + 源码行号级核对）。
> 本文是 Rust 实现侧的**机制级事实清单**，与 Java 上游报告配对构成双侧证据。

## 1. 量化视图（迁移后）

| 维度 | 数值 |
|------|------|
| 索引 | 656 文件 / 6,457 节点（2,103 函数 + 1,187 方法 + 234 struct + 52 enum）/ 25,116 边 |
| 模块分布 | core **295** · template **99** · cache **37** · parser **15**（grammar 拆分家族）· ext **15** · xml **12** · error **4** · builtins **4** · value/span/lib 3 |
| 最大文件 | environment.rs 3,358 · built_ins_for_sequences 1,777 · lexer 1,659 · xml/node 1,290 · exec 1,231 · eval 1,218（grammar.rs 6,837 已按产生式族拆为 13 文件 ≤800） |
| 公开面 | api-baseline **6,054** 项（CI 门禁锁定） |
| 核心枚举 | ExprKind **32** 变体 · ElementKind **44** · TemplateError **9** · Settings **26** 字段 |

**与 Java 的规模关系**：Java 21,724 节点 ↔ Rust 6,457（30%）——聚合枚举
（ExprKind/ElementKind/TemplateError）+ 一文件一对象镜像的双层设计消除类样板，
语义覆盖 412 MAPPED / 0 MISSING（结构对照 spec）。

## 2. 渲染主链（与 Java 链逐点对称）

```
Template::process (template.rs:46) → environment::render (:48) → Environment::process (:694)
  → do_auto_imports_and_includes → run (:781) → run_slice (:788)
run_slice 协作环：push_instruction_frame (:840) → exec::exec_owned (exec.rs:260)
  → consume_outcome (:900) → pop_instruction_frame (:861)
```

- **切片驱动 + mini 栈**：`els` 借引用零克隆执行；`ExecOutcome::Next/Replace` 产物入
  本地 mini 栈（`RunSignal::Completed/Returned/Flow/Stop` 终态信号）；
- **防护**：`MAX_OUTPUT_BYTES = 64MiB`（Java 无此硬限，文档化差异）；
- **影响半径对称性（本轮关键发现）**：
  - Java `Environment.visit` impact = **3 符号**（Java 报告 §2）
  - Rust `exec_owned` impact = **3 符号**（impact 实测：run_slice / run / exec.rs 自身）
  - 两侧各自把分派核心收敛为单调用者骨架，重构安全性同构。

## 3. 内建分派（Java BUILT_INS_BY_NAME 的镜像）

```rust
// built_in.rs:68 —— 生产调用点唯一
if let Some(f) = crate::builtins::lookup(name) { ... }
// builtins/mod.rs:47
pub fn lookup(name: &str) -> Option<BuiltinFn>   // 字符串 match 注册表
```

| Java（上游报告 §3） | Rust |
|---------------------|------|
| `BUILT_INS_BY_NAME` 静态 HashMap | `lookup` 的 match 注册表 |
| `putBI` snake_case + camelCase 双别名 | match 臂别名（web_safe→html 等） |
| `NUMBER_OF_BIS` 编译期 AssertionError | 「183/183 编译期全注册」清单核对惯例 |
| `newBuiltIn(ici, ...)` 工厂 | ICI 版本化在 eval 路径按 `settings.incompatible_improvements` 分流 |

## 4. 变量解析链（消费端对照，callers 实测）

| Java 消费点 | Rust 消费点 |
|------------|------------|
| `Identifier._eval`（Identifier.java:36） | `identifier.rs::eval`（:19） |
| `BuiltinVariable.get`（:329，.version/.now/...） | `builtin_variable.rs`（`BuiltinVar` 枚举直配：True/False/Now/Namespace/Main/Globals/Locals/DataModel…） |
| `Environment.__getitem__`（:3394，jython 桥） | `unified_call.rs::exec_call_impl`（:56） |

七级链：`get_variable`（environment.rs:940-979）——局部上下文栈 → 宏帧局部 →
当前命名空间 → 全局命名空间 → 根数据模型 → 共享变量 → 未找到；
strict 抛 `invalid_reference` / classic 返回 nothing 由使用点回退
（逐级注释锚定 Java 行号，与上游报告 §4 的 Java 链逐级对应）。

## 5. 错误装配（两段式镜像）

- **栈快照**：`attach_stack_to_error`（environment.rs:869，`with_stack` 幂等）——
  5 个调用点集中在 run_slice/consume_outcome 错误路径，**弹帧前调用**
  （快照须含当前失败帧）↔ Java `TemplateException` 构造时取
  `getInstructionStackSnapshot()`（上游报告 §8）；
- **精确抛出点**：`TemplateError` 9 变体 + `error/expected_messages/` 70 场景；
  构造经 `error/` 镜像文件（`non_*_exception.rs` 家族）委托——
  对应 Java 每 Non* 异常的单/双点构造（EvalUtil 收敛 + BuiltInForSequence 等）。

## 6. attempt/recover（镜像确认）

```
exec.rs:182-183: ElementKind::Attempt { try_, recover }
  → attempt_block::AttemptBlock::new(try_, recover).exec(env)
  （注释锚 AttemptBlock.java → visitAttemptRecover :3542）
consume_outcome 唯一调用者 = run_slice（callers 实测）
  RunSignal::Flow/Stop → Err 上传 ↔ Java RuntimeException/StopException 穿透
```

## 7. 缓存机制（**修正 Java 报告 §6 的一处表述**）

`template_cache.rs::get_or_load` 实测三机制齐全：

| 机制 | Rust 证据 | Java 锚 |
|------|----------|---------|
| TemplateKey 等价（normalized name + locale/encoding 维度） | `get_or_load` + `refresh_stale` | TemplateKey 五元组 |
| refresh delay | 默认 1s（`Settings.delay` 对齐）；delay 内不验证直接返回（:67-71） | TemplateCache.java:349-365 |
| **负查找缓存** | **已实现**——"负查找条目返回 None"（:52/:68 注释锚 Java:350-365） | storeNegativeLookup（:381/:505） |

> **修正**：Java 上游报告 §6 曾称「负查找缓存是 v1 Rust 未覆盖的行为点」——
> 实际代码已覆盖（本节注释锚在位）。仍为 NA 的仅 CacheStorage 容量/淘汰策略
> （MRU/Soft，结构对照 spec §4）。Java 报告该句以本节为准。

## 8. 格式化与后处理

- **预定义格式名**：`format.rs:600`（`format_number` 设置路径）/`:691`
  （`format_number_with` 显式路径）双接入 `currency`/`percent`；
  `currency_spec`（en_US `$` 前缀 2 位 / de·fr ` €` 后缀 / ja_JP 全角￥ 0 位 /
  zh_CN `¥`）+ `percent_suffix`（de/fr 空格）——5 locale Java 实测基线测试
  `currency_percent_java_baseline`（commit 9a10174）↔ Java
  `getCurrencyInstance/getPercentInstance`（上游报告 §7）；
- **格式聚合**：`iso_date_format.rs`（1,122）+ `java_date_format.rs`（969）承载
  Java 三级工厂树（ISO/Java/Alias）全部角色；`@name` 自定义格式报错路径已实现，
  注册表属 P3 缺口；
- **TemplatePostProcessor**：`Configuration.post_processors: RefCell<Registry>`
  （configuration.rs:43/:196 `add/remove_template_post_processor`）+
  get_template 后 `apply_all` ↔ Java `addTemplatePostProcessor` + cache 集成；
  ThreadInterruption 实现为 no-op + tokio CancellationToken 建议（差异文档化）。

## 9. pyo3 桥

- `FmConfiguration` **34-35 方法**（35 个 set_*/get_* + process；原 7 方法，
  Stage 2 补齐 commit 33f057e）；
- `FmTemplate::process`（lib.rs:383）含**根类型守卫**：标量/序列/bytes 根统一报
  "The data model must be a hash"（commit 086cca5，对齐 Java 对非
  TemplateHashModel 抛 IllegalArgumentException；PyPI 0.1.0b0 上发现的边缘行为）；
- wrap/unwrap 双向 + `FreeMarkerError`（⊂ RuntimeError）异常桥 +
  `unsendable` 线程约束（pyo3 运行时校验）。

## 10. Java ↔ Rust 对称性总表（双侧报告合并结论）

| 机制 | Java（上游报告 §N） | Rust（本报告 §N） | 对称性 |
|------|--------------------|--------------------|--------|
| 分派核心影响半径 | visit = 3（§2） | exec_owned = 3（§2） | ✅ 同构 |
| 内建注册 | HashMap + 双别名 + 断言（§3） | match 注册表 + 清单核对（§3） | ✅ |
| 变量链 | getVariable 七级 + 3 消费点（§4） | get_variable 七级 + 3 消费点（§4） | ✅ |
| 设置 DSL | _ObjectBuilderSettingEvaluator 1,121 行（§5） | NA-DESIGN（类型化 setter 替代） | ✅ 裁定 |
| 缓存 | TemplateKey + 负查找 + delay（§6） | 同三机制（§7，本轮修正） | ✅ |
| 格式化工厂 | 三级继承 + Alias + 回退（§7） | 双聚合文件 + currency/percent（§8） | ✅ |
| 错误装配 | 两段式 + 精确抛出点（§8） | 两段式 + 镜像文件委托（§5） | ✅ |
| attempt/recover | AttemptBlock + handleTemplateException（§9） | AttemptBlock + consume_outcome（§6） | ✅ |
| 节点规模 | 21,724（§1） | 6,457 = 30%（§1） | 聚合枚举收益 |

## 11. 索引维护说明

- 本仓库与 Java 仓库的 `.codegraph/` 索引及 CLI 二进制（~/.codegraph/versions/）
  曾于 2026-08-16 被外部清理，本轮重装 tool 1.6.0 并重建（freemarker-rust 656 文件）；
- 后续查询前应先 `codegraph status` 检查存在性，丢失时 `codegraph init -i`
  重建（~1s，非阻塞步骤）。

## 12. 引用本文的对照锚

引用格式：`Rust 侧证据见本 spec §N`。配对引用示例：
- Java 机制 ↔ Rust 实现的逐点对照 → 本文 §10 总表
- 渲染循环/错误装配/变量链的行号级锚 → 本文 §2/§4/§5
- 与 `2026-08-16-java-upstream-architecture-design.md` 联合阅读构成双侧证据链。

---

## 对应计划

- `docs/superpowers/plans/2026-08-15-production-readiness.md`（Stage 2 pyo3 桥接 /
  Stage 3 grammar 拆分——本文 §1/§9 的规模与行为证据）
- `docs/superpowers/plans/2026-08-14-layout-parity-migration.md`（布局对齐轮——
  本文 §1 模块分布的成因）
