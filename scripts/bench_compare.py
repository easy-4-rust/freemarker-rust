#!/usr/bin/env python3
"""Rust vs Java 性能对比 —— L3 性能门禁。

用法:
  python3 scripts/bench_compare.py            # 运行 Rust bench + Java Bench + 对比
  python3 scripts/bench_compare.py --rust-only  # 只跑 Rust bench
  python3 scripts/bench_compare.py --java-only  # 只跑 Java bench
  python3 scripts/bench_compare.py --ratio-only  # 只解析已有结果对比

输出: docs/测试/性能基准报告.md + 门禁结果（默认阈值 ≥ 0.5×）
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
THRESHOLD = 0.5  # Rust ≥ Java 0.5×
JAR = Path.home() / ".m2/repository/org/freemarker/freemarker/2.3.34/freemarker-2.3.34.jar"


def run_rust_bench() -> dict:
    """运行 criterion 并解析 stdout 的 time 行（取中位数）。"""
    out = subprocess.run(
        ["cargo", "bench", "-p", "freemarker", "--bench", "simple_render"],
        cwd=ROOT, capture_output=True, text=True,
    ).stdout
    results = {}
    # 格式: simple_loop_100        time:   [44.217 µs 44.322 µs 44.442 µs]
    for line in out.splitlines():
        m = re.match(r"(\w+)\s+time:\s+\[([\d.]+) (\w+) ([\d.]+) (\w+) ([\d.]+) (\w+)\]", line)
        if m:
            name = m.group(1)
            median = float(m.group(4))  # 中位数是第二个区间值
            unit = m.group(5)
            mult = {"ns": 1.0, "µs": 1e3, "ms": 1e6}[unit]
            results[name] = median * mult
    return results


def run_java_bench() -> dict:
    """运行 Java Bench 并解析 name=ns/op 行。"""
    if not JAR.exists():
        print(f"[ERROR] FreeMarker jar 不存在: {JAR}", file=sys.stderr)
        sys.exit(1)
    classes = ROOT / "scripts/java_probe/classes"
    subprocess.run(
        ["javac", "-cp", str(JAR), "-d", str(classes), str(ROOT / "scripts/java_probe/Bench.java")],
        check=True, capture_output=True,
    )
    out = subprocess.run(
        ["java", "-cp", f"{JAR}:{classes}", "Bench", "20000"],
        capture_output=True, text=True,
    ).stdout
    results = {}
    for line in out.splitlines():
        m = re.match(r"(\w+)=([\d.]+)", line)
        if m:
            results[m.group(1)] = float(m.group(2))
    return results


def write_report(rust: dict, java: dict, ratios: dict) -> Path:
    """生成性能基准报告。"""
    lines = [
        "# 性能基准报告（Rust vs Java FreeMarker 2.3.34）",
        "",
        f"- 生成时间: （见 git 提交）",
        f"- Java: FreeMarker 2.3.34 (jar)，JIT 预热 2000 次，20000 次迭代取均值",
        f"- Rust: criterion（release profile），100 samples 取中位数",
        f"- 门禁: Rust ≥ Java 0.5×（`scripts/bench_compare.py`）",
        "",
        "| 场景 | Java (ns/op) | Rust (ns/op) | Rust/Java | 门禁 0.5× |",
        "|------|-------------|-------------|-----------|----------|",
    ]
    ok_count = 0
    for name in sorted(set(rust) & set(java)):
        r, j = rust[name], java[name]
        ratio = j / r if r else 0
        ok = ratio >= THRESHOLD
        ok_count += ok
        lines.append(f"| {name} | {j:.0f} | {r:.0f} | {ratio:.3f}× | {'✅' if ok else '❌'} |")
    total = len(set(rust) & set(java))
    lines += [
        "",
        f"**结果: {ok_count}/{total} 达标**",
        "",
        "## 未达标分析",
        "",
        "复杂场景（循环/宏/哈希访问）显著落后于 Java，可能瓶颈：",
        "- `TModel` 的 `Rc` 克隆与每帧分配（宏调用/循环变量）",
        "- 指令栈/局部上下文栈每次 push/pop 的分配",
        "- 表达式求值路径中的字符串与索引分配",
        "- 哈希访问未走零分配路径",
        "",
        "优化方向（对应 docs/12 P6）：指令栈零分配、字符串拼接复用、",
        "热点路径（变量查找、数字格式化）减少分配。",
    ]
    report = ROOT / "docs/测试/性能基准报告.md"
    report.parent.mkdir(parents=True, exist_ok=True)
    report.write_text("\n".join(lines) + "\n")
    return report


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rust-only", action="store_true")
    ap.add_argument("--java-only", action="store_true")
    ap.add_argument("--ratio-only", action="store_true")
    args = ap.parse_args()

    rust = java = {}
    if not args.java_only:
        rust = run_rust_bench()
        print("Rust:", json.dumps({k: f"{v:.0f}ns" for k, v in rust.items()}))
    if not args.rust_only:
        java = run_java_bench()
        print("Java:", json.dumps({k: f"{v:.0f}ns" for k, v in java.items()}))

    if not rust or not java:
        print("需要两边的数据才能对比（--rust-only/--java-only 时跳过）")
        return

    ratios = {n: java[n] / rust[n] for n in set(rust) & set(java) if rust[n] > 0}
    report = write_report(rust, java, ratios)
    print(f"\n报告: {report}")
    print("门禁结果:")
    ok = True
    for name, ratio in sorted(ratios.items()):
        passed = ratio >= THRESHOLD
        ok &= passed
        print(f"  {name}: {ratio:.3f}× {'✅' if passed else '❌'}")
    sys.exit(0 if ok else 2)


if __name__ == "__main__":
    main()
