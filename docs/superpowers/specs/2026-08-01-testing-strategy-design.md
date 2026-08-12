# 测试与验证策略设计

- **日期**：2026-08-01
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（freemarker-jython25/test/ 521 文件）
- **依赖**：proptest、criterion、cargo-fuzz

---

## 1. 测试金字塔（三层）

```
L1 单元测试（Rust #[test]，每模块内联）
L2 黄金套件（数据驱动集成测试：.ftl + data + expected）
L3 Java 对比测试（同一模板双引擎输出 diff）+ 性能基准
```

## 2. L1 单元测试

- **解析器**：token 级快照（词法状态切换）、产生式边界、错误位置断言。
- **表达式/内建**：每个 BI 至少 3 用例（正常/边界/错误消息）；UTF-16 语义专项（中文/emoji）。
- **算术引擎**：混合类型矩阵（Int×Decimal、Long 溢出、除法 scale 表）。
- **格式化**：CFormat × 数字边界（NaN/Infinity/大整数/指数）、日期模式表、转义字符表。
- **缓存**：过期/负查找/规范化时序测试（`tokio` 不需要，用 `Instant` 注入）。
- 覆盖率目标：核心模块 ≥ 85%（`cargo llvm-cov`）。

## 3. L2 黄金套件（核心资产）

### 3.1 来源与形态

- 源：`freemarker-jython25/src/test/resources/freemarker/test/templatesuite/testcases.xml`（**100+ 用例**）+ `templates/*.ftl` + `expected/*.txt`。
- Rust 形态：独立 **`freemarker-test` 模块**（对应 Java 侧 freemarker-test-utils + templatesuite 的角色；
  整体功能测试收敛于此，`freemarker` / `freemarker-pyo3` 内仅保留局部单元测试）：
  ```
  freemarker-test/
  ├── Cargo.toml        # dev 专用 crate（publish = false），依赖公开 API 驱动引擎
  ├── src/lib.rs        # 模块说明（允许 dead_code）
  └── tests/
      ├── golden.rs     # 黄金套件 runner：manifest.json（128 用例）→ 渲染 → 逐字节 diff
      ├── common/       # assert/assertEquals/assertFails/noOutput 断言指令 + 设置应用
      └── suite/        # 从 Java templatesuite 复制的用例（模板与 expected 逐字节一致）
  ```
  ```
- 翻译脚本：`scripts/extract_suite.py` —— 从 Java 仓库提取 testcases.xml + 模板 + 期望输出，生成 Rust 测试数据（一次性 + 可重复运行，供上游同步）。

### 3.2 数据模型注入

- Java 侧用例数据是 Java 对象/beans；Rust 侧统一用 **JSON 数据文件**（`.data.json`）经 `serde_json → TModel`（wrapper `SimpleObjectWrapper`+serde 适配）注入：
  - `Map` → SimpleHash；`Array` → Sequence；`String/Number/Bool/null` → 标量对应
  - **数据转换规则记录在 `tests/suite/README.md`**，与 Java 用例的语义等价性人工评审（对象字段名 → JSON 键）

### 3.3 执行与断言

- 默认配置 `Configuration(VERSION_2_3_34)` + `StringLoader` 注册全部模板。
- 输出 diff：**逐字节**（`expected.txt` 含精确换行）；差异报告输出统一 diff 视图（`similar` crate 或自实现）。
- 错误用例（`errorExpected` 属性）：断言 `TemplateError` 消息与 `expected` 一致（specs/2026-08-01-error-handling-design.md §4 容忍清单内允许差异）。
- 运行方式：`cargo test --test golden`；CI 必跑。

### 3.4 套件覆盖矩阵（按类别拆分 runner 便于定位）

| 类别 | 代表用例（testcases.xml） | 对应 P 阶段 |
|---|---|---|
| 基础输出 | helloworld、comment、variables、non-strict-syntax | P2 |
| 控制流 | if、list、list2、list3、list-bis、switch、attempt、compress、whitespace-trim、wstripping | P2 |
| 宏与调用 | macros、assignments、include、include2、import、interpret、multimodels | P2 |
| 字符串内建 | string-builtins1、string-builtins2、string-builtins-regexps、lastcharacter | P3 |
| 数值与类型 | arithmetic、comparisons、boolean、boolean-formatting、type-builtins* | P3 |
| 集合 | hashliteral、listliteral、iterators、exthash、bean-maps | P3 |
| 编码/转义 | encoding-builtins*、escapes、xml、url | P3/P4 |
| 日期 | dateformat-iso-like、dateformat-iso-bi*、dateformat-java、dateparsing | P4 |
| 本地化 | 多 locale 用例（`localized` 属性） | P4 |
| 错误 | 各 `errorExpected` 用例 | P2/P6 |
| Python | helloworld、macros、listhash（jython 数据） | P5 |

## 4. L3 Java 对比测试

- **自动化对比工具**：`scripts/diff_against_java.sh` ——
  1. `gradle :freemarker-jython25:test`（Java 侧跑同一套件）→ 输出基线
  2. `cargo test --test golden` 输出 Rust 结果
  3. `scripts/compare_outputs.py` 逐用例 diff，生成 `compat_report.html`（通过/差异/缺失三栏）
- **随机属性测试**：`proptest` 生成模板片段（表达式/内建组合）→ 双引擎对比（需 Java 侧 harness `scripts/java_probe/`，单类 main 方法渲染任意模板，回报 JSON 结果）——P6 打磨期启用。
- **错误消息对比**：`scripts/compare_errors.py` 对照 specs/2026-08-01-error-handling-design.md §4 基线。

## 5. 性能基准（`benches/`，criterion）

| 基准 | 场景 | 目标 |
|---|---|---|
| `render_simple` | `${x}` × 1000 插值 | ≥ Java 0.5× |
| `render_loop` | `list` 10k 项 | ≥ Java 0.5× |
| `render_macro` | 宏调用 × 1000 | ≥ Java 0.5× |
| `parse_large` | 50KB 模板解析 | 对比基线 |
| `render_py_bridge` | Python dict 数据渲染（pyo3） | ≥ Java Jython 版 1×（**Jython 慢，Rust 应显著快**） |
| `cache_hit` | 缓存命中路径（零分配检查 `dhat`） | 与 Java 同量级 |

- 基线方式：Java 侧用 JMH 或简单 `System.nanoTime` harness（`scripts/java_probe/Bench.java`），同一模板/数据。
- 内存：`dhat` 堆分析（P6）。

## 6. 覆盖率与质量门禁（CI）

```
cargo test          # L1 + L2 全量
cargo clippy -D warnings
cargo fmt --check
cargo llvm-cov      # 核心模块 ≥ 85%
pytest tests/       # freemarker-pyo3 Python 侧
maturin build + smoke test
compat_report.html  # L3 差异 ≤ 容忍清单
```

## 7. 验收判据汇总（对应 specs/2026-08-01-project-overview-design.md §4）

1. L2 黄金套件通过率 100%（逐字节）。
2. L3 对比：无容忍清单外差异。
3. L1 覆盖率达标；错误消息基线测试通过。
4. 基准达到性能目标；`dhat` 报告无异常分配热点。
5. Python 套件（P5）全绿 + 多线程冒烟。

---

## 对应计划

- `docs/superpowers/plans/2026-08-03-alpha0-production-readiness.md`（鲁棒性/安全）
- `docs/superpowers/plans/2026-08-03-alpha1-governance-hardening.md`（golden harness 收口）
- `docs/superpowers/plans/2026-08-04-builtins-coverage-rounds.md`（java_ported 测试新增）
- `docs/superpowers/plans/2026-08-04-coverage-test-completion.md`（覆盖率补齐）
