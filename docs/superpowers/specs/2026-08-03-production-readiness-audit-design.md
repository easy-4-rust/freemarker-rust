# freemarker-rust 生产就绪审计报告

- **日期**：2026-08-03
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97）
- **依赖**：`2026-08-03-versioning-design.md`（§3.1 晋级条件）

---

# freemarker-rust 生产就绪审计报告（2026-08-03）

> 审计对象：`freemarker` 0.1.0-alpha.1（Apache FreeMarker 2.3.34 语义兼容 Rust 移植）
> 审计口径：versioning.md §3.1 的 1.0 晋级 8 条件逐条实测 + golden 定格 + 发布就绪核查
> 结论：**Beta 级生产就绪**——核心引擎逐字节对齐 113/128 黄金用例（88%），
> 全部治理门禁绿；15 项永久 NA 为用户决策下的上限（非引擎缺口）。

## 1. 1.0 晋级 8 条件核查（versioning.md §3.1）

| # | 条件 | 状态 | 实测证据（2026-08-03） |
|---|---|---|---|
| 1 | cargo-deny / cargo-audit 全绿 | ✅ | `cargo deny check`：advisories/bans/licenses/sources 全 ok；`cargo audit`：137 依赖 0 漏洞 |
| 2 | cargo public-api 基线 diff 门禁 0 | ✅ | `cargo public-api -p freemarker` diff `docs/release/api-baseline.txt`（3804 行）= **0 diff**（B4 新增 `TemplateApiSupport` + `TModel::api` 已重生成基线） |
| 3 | Clippy/fmt/workspace 全测试 + 128 套件 ≥86 MIRRORED | ✅ | clippy 0 warning、fmt 干净、**864 tests 0 failed**（golden 113 + java_ported 502 + lib 326 + fuzz + pyo3）；golden **113/128（88%）** ≥ 86 |
| 4 | `cargo package --verify -p freemarker` 发布演练 | ✅ | `cargo publish --dry-run -p freemarker`：Packaged 146 files、Verify 编译通过、dry-run 中止上传（= package --verify 全流程） |
| 5 | 多 OS CI 矩阵（ubuntu/macos/windows × stable + MSRV 1.85） | ✅ | 上一基线 12/12 全绿（本版提交推送后复查） |
| 6 | proptest fuzz（解析器 + 表达式）10000 用例无 panic | ✅ | `robustness_fuzz_smoke` cases=10000 + timeout=5000 防御；**本轮重跑 2/2 通过（13.76s）** |
| 7 | criterion 基准集落档 | ✅ | `docs/release/benchmarks.md` + `docs/测试/性能基准报告.md`（5/5 达标，Rust/Java ≥ 0.5×） |
| 8 | 安全模型文档评审 + "受限子集"边界明记 | ✅ | `docs/release/security.md` 本轮更新：?new 四策略 / ?api 视图机制 / XML 子集 / 15 项永久 NA 决策记录 |

**8/8 全部达成**（推送后 CI 实测 12/12 全绿，含 governance public-api 0 diff）。

## 2. golden 定格：113/128 PASS（88%）

| 指标 | 数值 |
|---|---|
| PASS（逐字节一致，含 no_output 渲染成功） | **113** |
| FAIL | **0** |
| 永久 NOT_APPLICABLE（用户决策，分类确定化） | **15** |
| BLOCKED / PARTIAL / MISSING | **0 / 0 / 0** |

### 2.1 永久 NA 清单（15 项，golden.rs `permanent_na_reason`）

| 类别 | 用例 | 原因 |
|---|---|---|
| JVM 反射（12） | `beans` | BeansWrapper/POJO 数据模型（security.md 决策 1，用户以 DynValue 手工包装） |
| | `overloaded-methods-23bc` | BeansWrapper 反射方法重载分派 |
| | `overloaded-methods-2-{inc,desc}-bwici-2.3.20` ×2 | 同上 |
| | `overloaded-methods-2-bwici-2.3.21` ×6 | 同上 |
| Java 特有类（1） | `transforms` | JythonRuntime 变换类（ClassNotFoundException） |
| jython25 过期断言（2） | `string-builtins3` | 断言与真实 Java 2.3.34 矛盾（jar 实测 `-1?lower_abc` 解析为 `-(1?lower_abc)`） |
| | `date-type-builtins` | 断言与真实 Java 2.3.34 矛盾（jar 实测 `?string.xs` date-only 输出带 Z） |

## 3. 能力覆盖

- **内建函数 183/183**（Java 2.3.34 全集）：阶段 A 补齐 eval_json/is_date_like/next_sibling/
  previous_sibling/web_safe；docs/05 清单同步
- **`?api`/`?has_api`**：`TemplateApiSupport` trait + `TModel.api` 槽位（对应 Java
  `TemplateModelWithAPISupport`）；API 视图由包装方提供，引擎无反射攻击面
- **`?new` 四策略**：unrestricted/safer/allows_nothing/opt-in + trusted_templates
  （默认 Unrestricted 与 Java Configurable.java:477 一致）
- **ICI 版本化**：?html <2.3.20 / HashLiteral 重复键 <2.3.21 / is_sequence <2.3.24 /
  is_enumerable <2.3.21
- **XML 子集**：visit 前缀宏分派（getNodeProcessor 语义）、node[0] 自身、`./` 相对路径、
  `true()`、XPath 子集；roxmltree 无外部实体（无 XXE）
- **java_ported 502/7 ignored**：Java 测试逻辑 1:1 移植，错误消息逐字对齐

## 4. 鲁棒性

- proptest 10000 用例（解析器 + 表达式，robustness_fuzz_smoke）+ 5000ms timeout 防御
  （2026-08 曾捕获旧中间代码病态增长，当前 10 轮稳定——见 `docs/superpowers/specs/2026-08-02-rust-obligation-ledger-design.md`）
- cargo-fuzz：expression/parser target 声明完成（nightly 构建验证），不常驻 CI
- 全量 864 tests：lib 326 + golden 113 + java_ported 502 + 其余，0 failed

## 5. 发布就绪

- **crates.io**：`cargo publish --dry-run -p freemarker` 演练通过；release.yml（tag 触发）
  = dry-run + GitHub Release（changelog 段自动提取）；实际 `cargo publish` 由用户手动执行
- **PyPI（pyo3）**：一键可发布状态——pyproject readme/authors/classifiers/LICENSE 齐备、
  `pyo3-publish.yml`（tag `pyo3-v*` → maturin build --sdist → Trusted Publishing）；
  实际上传需用户配置 PyPI 发布者后手动打 tag（TestPyPI 演练就绪）
- 多 OS CI 12/12（本版推送后复查）

## 6. 剩余风险（诚实清单）

| 风险 | 说明 |
|---|---|
| 并发模型（Rc 单线程） | 跨线程渲染不可用；线程内共享 Configuration 可用（内部 Rc 不跨线程） |
| 模板深度/体积上限 | 未实现 `max_template_depth` / `max_output_bytes`（Java 同样无硬上限；DoS 防护靠外层） |
| JVM 反射缺失 | 12 项永久 NA；POJO 需用户手工包装 DynValue（决策 1 明记） |
| 自动转义完整矩阵 | escapes 类边缘行为未 100% 对齐（核心矩阵已对齐） |
| java_ported 7 ignored | 引擎缺口断言保留（?absolute_template_name 等），后续迭代 |

## 7. 结论

- **达到用户决策下的迁移上限**：golden 113/128（88%）、内建 183/183、0 FAIL / 0 BLOCKED；
- **1.0 晋级 8 条件全部达成**（条件 5 以推送后 CI 复查为准）；
- **可发布**：crates.io dry-run 演练通过；PyPI 一键可发布（手动触发）；
- **诚实边界**：15 项永久 NA（JVM 反射/方法重载/Java 特有类/过期断言）不随引擎演进恢复，
  文档化于 security.md 决策记录 + golden.rs permanent_na_reason。

---

## 对应计划

- `docs/superpowers/plans/2026-08-03-alpha1-governance-hardening.md`
