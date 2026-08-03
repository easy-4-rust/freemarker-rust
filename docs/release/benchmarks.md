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