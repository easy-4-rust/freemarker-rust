# RUST_OBLIGATION 测试账本

- **日期**：2026-08-02
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97）
- **依赖**：`2026-08-01-testing-strategy-design.md`

---

# RUST_OBLIGATION 测试账本（初稿）

> 依据 rust-java-migration-testing 技能 §4：记录 Rust 实现引入的、Java 中不存在的正确性义务。
> 状态随实现回填（由各智能体测试与门禁结果更新）。

| # | Rust 机制 | 义务问题 | Rust 测试 | 状态 | 备注 |
|---|---|---|---|---|---|
| R1 | 所有权/`Rc` | TModel 角色槽位生命周期、Arc<Template> 缓存共享无泄漏 | template_cache.rs 的 Arc::ptr_eq 测试 | ✅ 已实现（智能体 C） | 命中返回同一 Arc |
| R2 | 类型化错误 | `TemplateError` 变体区分（InvalidReference/TypeMismatch/Parse/Stop/Flow）与 `to_user_message` 对齐 | error/template_error.rs 消息断言 | ✅ 已实现 | |
| R3 | 数值精度 | BigDecimal 精确运算不降级 f64；HALF_UP 舍入方向（负数除数） | arithmetic_engine.rs 27 测试（含 div_negative_divisor_rounds_away_from_zero） | ✅ 已实现（智能体 B，修复真实 bug） | |
| R4 | UTF-16 语义 | `?length`/`?substring`/`?pad` 按 Java char（UTF-16）计，非码点 | eval.rs UTF-16 ?length 测试（非 BMP=2）+ golden string-builtins 系列 | ✅（V7） | docs/05 §3 风险点 1 |
| R5 | Java trim 差异 | `String.trim` 仅 ≤U+0020，非 Unicode 空白 | utility/string_util.rs java_trim_only_ascii_space | ✅ 已实现 | |
| R6 | 正则差异 | Java 反向引用/环视 vs Rust regex → fancy-regex；`?matches`/`?groups`/`?replace_re` | builtins/strings_regexp.rs + golden string-builtins-regexps（逐字节 PASS） | ✅ | docs/05 §3 风险点 2 |
| R7 | 角色槽位判定 | `?is_*` 与 Java `instanceof` 一致（is_indexable/is_enumerable 差异） | t_model.rs + eval.rs is_* 家族测试 + golden type 相关用例 | ✅ | |
| R8 | 缓存时序 | delay 过期/负查找的确定性（无 GC 语义依赖） | template_cache.rs 6 测试（find 计数、delay 内不重读） | ✅ 已实现（智能体 C） | |
| R9 | 路径安全 | FileLoader `../`/绝对路径逃逸拒绝 | file_template_loader.rs 3 测试 | ✅ 已实现（智能体 C） | |
| R10 | 错误上下文 | 渲染错误拼接模板名+行+列；指令栈转储 | environment.rs attach_location + 错误消息断言测试 | ✅（V8） | docs/09 |
| R11 | 流控信号 | break/continue/return/stop 的栈传播与非法位置报错 | exec.rs 端到端（循环内外 break/continue、stop、宏 return）+ 解析期校验 | ✅ | |
| R12 | 惰性求值 | `x!default` 默认表达式不执行（副作用/错误抑制） | eval.rs Default 惰性测试（用报错表达式验证） | ✅ | |
| R13 | 名称规范化死循环防护 | remove_dot_steps 前导点/`x.` 用例不无限循环 | template_name_format.rs 回归用例 | ✅ 已实现（智能体 C 修复死循环） | 曾 150% CPU 空转 |
| R14 | Send/Sync | Configuration/Template 跨线程共享渲染 | 决策：v1 单线程 Rc 模型（Arc/Rc 死锁修复后统一）；并发升级路径见 docs/07 §5 | ⚠️ 已记录 | v1 约束 |
| R15 | pyo3 GIL | 渲染期 GIL 单次持有/allow_threads 纪律 | freemarker-pyo3 crate 33 测试 + pytest 22 用例 | ✅ 已实现（智能体 H） | docs/10 §4 |
| R16 | 解析期剥离改写文本 | 空白剥离在解析期直接改写 Text（Java `text = substring`），strip_before/strip_after 标记恒 false——链上后文所见即最终内容；`orig_end_line` 保留 token 原始结束行（Java endLine 裁剪时不更新） | grammar.rs whitespace_stripping_flags（解析期最终文本断言，Java jar 实测对照）+ golden nested/whitespace-trim 等 | ✅ 本轮 | TextBlock.java:128/206-208；空文本 ignorable → heeds=false（:349-352） |
| R17 | `<#setting>` 白名单 | 配置级设置（whitespace_stripping/strict_syntax/output_format/auto_escaping）在模板内修改 → 解析错误（Java PropertySetting 白名单） | exec.rs whitespace_stripping_applies（断言 "isn't supported"） | ✅ 本轮 | PropertySetting.java:71-82 |
| R18 | 数字类型转换回绕 | ?int/?long/?byte/?short 用原始类型强转语义（溢出回绕，f64 越界饱和同 JVM d2l），非 Rust `as` 饱和 | eval.rs/numbers.rs + golden numerical-cast（2147483648?int=-2147483648 等） | ✅ 本轮 | Java intValue()/byteValue()；BuiltInsForNumbers.java |
| R19 | Float/Double 格式化快路径 | DecimalFormat 快路径：最短往返表示 + max_frac 舍入（Float 先加宽 Double）；toBigDecimal 比较/算术用 toString 最短表示（两路径不同） | format.rs decimal_format 测试 + golden numerical-cast/number-format | ✅ 本轮 | JDK FastDecimalFormat；ArithmeticEngine.toBigDecimal :608-625 |
| R20 | `.args` 惰性构建 | Java `BuiltinVariable.Args` 仅在模板**访问** `.args` 时构造（Macro.Context.argsSpecialVariableValue）；"位置 catch-all 非空 + .args" 报错只在访问时触发——不访问 `.args` 的宏（`<@m 1 2 3/>`）正常输出 | environment.rs build_args_special（eval.rs 访问时调用，frame 存 def/is_function 快照）；exec.rs macro_catch_all_and_positional + java_ported args_special_variable_test 11 用例 + with_args_built_in_test 18 用例（jar ProbeMacro2-4 实测 2.3.34 三例） | ✅ 本轮 | 修复真实 bug：v1 曾急切构建导致纯位置 catch-all 宏误报 "must only be called with named arguments" |

---

## 对应计划

- `docs/superpowers/plans/2026-08-01-p1-p4-core-implementation.md`
