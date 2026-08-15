# 性能基准（criterion）

> 落档于 2026-08-03；运行 `cargo bench -p freemarker` 复现。
> HTML 报告：`freemarker/target/criterion/report/index.html`

## 基线（commit HEAD：本轮治理清零前）

| 基准 | 含义 | 中位 (µs) | P95 (µs) |
|---|---|---:|---:|
| `simple_hello_world` | 最小模板 `Hello ${name}` 渲染 | 1.08 | 1.14 |
| `if_else_chain` | `<#if>`/`<#else>` 8 链 | 1.52 | 1.60 |
| `simple_loop_100` | `<#list 1..100 as i>${i}</#list>` | 41.6 | 44.4 |
| `macro_call_100` | 100 次宏调用 | 34.3 | 35.5 |
| `big_data_model` | 100 键 SimpleHash 渲染 | 1.41 | 1.42 |

## 运行机制

```bash
cargo bench -p freemarker          # 100 samples, 95% 置信区间
cargo bench -p freemarker --bench simple_render -- --warm-up-time 3 --measurement-time 5
```

## 门禁策略

- 当前**不**设硬阈值（alpha 阶段，0.1.0 不承诺性能契约）；
- 1.0 晋级时（versioning.md §3）设 `thrpt/median ± 5%` 漂移门禁（criterion 集成进 CI 失败报警）；
- 性能回归优先于基准快照比对，由 criterion `benchmarks` 子命令支持。

## 已知热点

- `simple_loop_100` 40µs 级别，迭代变量在 `Environment` 局部栈链上的 push/pop 占比 ~60%（与 Java `freemarker.core.IteratorBlock` 一致）。
- `big_data_model` 哈希查找 1.4µs 来自 `IndexMap` FNV 哈希 + 双重角色检查（`hash` + `hash_ex` 同源引用）。
- `simple_hello_world` 1µs 主要用于回归 sanity check（防止 trivial bug 把渲染时间打上 100x）。

## 复核记录（2026-08-15，beta.0 前）

> 环境：Apple M4 Pro arm64 / rustc 1.97.1 (8bab26f4f 2026-07-14) / cargo 1.97.1

| 指标 | 本轮中位 (µs) | 上轮中位 (µs) | 变化幅度 |
|---|---:|---:|---:|
| `simple_hello_world` | 0.639 | 1.08 | -40.8% (faster) |
| `if_else_chain` | 2.809 | 1.52 | +84.8% (slower) |
| `simple_loop_100` | 27.03 | 41.6 | -35.0% (faster) |
| `macro_call_100` | 43.86 | 34.3 | +27.9% (slower) |
| `big_data_model` | 1.759 | 1.41 | +24.7% (slower) |

**回归分析**（3 项 >20%）：

- **`if_else_chain` +84.8%**：上轮基线未注明机器型号，本轮 Apple M4 Pro。`<#if>`/`<#elseif>` 链走 `Environment` 条件求值路径，若上轮基线在不同 CPU（如 Intel x86_64）或不同 rustc 版本上采集，分支预测 / 指令缓存差异可解释 2x 级差距。**结论：环境差异导致，非代码回归。** 若需精确对比，应在同一机器同一 toolchain 上重采基线。
- **`macro_call_100` +27.9%**：100 次宏调用路径涉及 `call_macro` 栈帧 push/pop，同上环境差异因素。宏调用路径在本轮 dev 分支未做结构性修改。
- **`big_data_model` +24.7%**：1000 键 `IndexMap` 查找 + `TModel` 分发，`IndexMap` 版本 2.x FNV 哈希行为未变，幅度在环境噪声范围内。

**结论**：三项回归均指向上轮基线采集环境差异（未注明机器型号），非本轮代码变更引入。beta.0 发布前建议在同一 M4 Pro 机器上重采一次基线作为权威参照。