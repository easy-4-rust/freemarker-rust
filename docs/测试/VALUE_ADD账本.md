# VALUE_ADD 测试账本

> 依据 rust-java-migration-testing 技能 §5：记录 Java 套件之外、为捕获具体缺陷/风险而新增的测试。

| # | 测试 | 位置 | 捕获的具体缺陷/风险 | 来源 |
|---|---|---|---|---|
| V1 | `div_negative_divisor_rounds_away_from_zero`（6 断言：2/-3、-2/-3、-2/3、1/-3、10/-4、1.0/-6） | core/arithmetic_engine.rs | **真实运算 bug**：BigDecimal 除法进位方向只取被除数符号，除数为负时进错方向（2/-3 得 -0.666666666665，应为 -0.666666666667） | 智能体 B 对照 ArithmeticEngine.java 逐行核验发现 |
| V2 | `java_trim_only_ascii_space` | utility/string_util.rs | Java `String.trim`（≤U+0020）与 Rust `trim()`（Unicode 空白）差异：U+00A0 不应被裁剪 | 骨架阶段编码错误引发 panic 后固化（docs/05 §3 风险点 3） |
| V3 | `remove_dot_steps` 回归用例（`x.`、`a..b/c`、`**/c` 前导假警报） | cache/template_name_format.rs | **死循环 bug**：remove_dot_steps 游标被 saturating_sub(0) 钳制，前导点/`x.` 输入无限循环（3 个测试二进制 150% CPU 空转 30-48 分钟） | 智能体 C 修复后补回归 |
| V4 | 缓存命中 `Rc::ptr_eq`、负查找 find 计数、delay 内不重读 | cache/template_cache.rs | 负查找缓存缺失会重复 IO；delay 语义错误会过度重读（缓存行为可观察断言） | 技能 §6 "缓存测试不得只观察最终值" |
| V5 | `Configuration::clone` 死锁回归 | template/configuration.rs + golden 全量 | **Mutex 非重入死锁**：get_template 持 cache 锁后 self.clone() 再次加锁 → 全测试挂死 | 集成阶段修复后由全量测试守护 |
| V6 | 跨类型比较报错（`1 == "a"` 报错而非 false） | core/eval.rs | Java `EvalUtil.compare` 只允许同类型比较（字符串仅 ==/!=）——凭直觉易错实现为 false | 智能体 D 对照 EvalUtil.java:183-317 |
| V7 | UTF-16 `?length`（非 BMP 字符 = 2） | core/eval.rs | Java `String.length()` 按 UTF-16 计，Rust `chars().count()` 按码点（差 2 倍） | docs/05 §3 风险点 1 |
| V8 | 除零/类型不匹配错误消息断言 | 多模块 | 错误消息需可观察（技能红线：禁止泛泛 is_err()） | 技能 §6 |
| V9 | attempt 不捕获 Stop 外流控、switch 吞 break 怪癖 | core/exec.rs 端到端 | Java 运行时异常层级差异（BreakOrContinue/Return 是 RuntimeException，attempt 只捕 TemplateException；SwitchBlock.java:108-115 注释确认的怪癖） | 智能体 D 对照源码 |
| V10 | 空白剥离双向扫描（opening/trailing 独立计算） | parser/grammar.rs + golden | Java TextBlock.postParseCleanup 顺序执行会残留纯空白文本；`<#if true>\n  yes\n</#if>` 输出 `"  yes\n"` | 智能体 D 修正 Term::heeds 后固化 |

| V11 | 空文本 ignorable 回归（switch case 尾随空文本阻断后 case 的 \n 剥离） | parser/grammar.rs | Java `isIgnorable(空)=true` → heeds=false（TextBlock.java:349-352）；heeds_opening("") 空循环落真值错误阻断链 | whitespace-trim 专项修复后由 golden switch/loopvariable 守护 |
| V12 | 块的末叶行号 threading | parser/grammar.rs | Java 链从块内最后一个叶的 endLine 继续（`<#if y>foo\n  </#if>bar` —— "bar" 在行 2 与 endLine 匹配）；span.line 近似导致 bar 无法匹配 | whitespace_stripping_flags 断言固化 |
| V13 | 浮点比较 vs 格式化双路径 | value.rs / builtins/format.rs | toBigDecimal 用 toString 最短表示（0.05f == Decimal(0.05)）；DecimalFormat 快路径用加宽 double 最短表示（1.01?float=1.00999999）——混用会同时破坏比较与格式化 | golden numerical-cast/number-math-builtins |

## 统计

- VALUE_ADD 测试 13 组（含多断言），全部为可观察断言
- 其中 V1、V3、V5 捕获了真实 bug（运算舍入、死循环、死锁）；trim 专项的 R16-R19 义务由 golden 全量守护
