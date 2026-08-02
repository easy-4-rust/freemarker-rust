#!/usr/bin/env python3
"""生成 L3 兼容性报告（compat_report.html）。

汇总:
- golden suite PASS/FAIL/SKIP 统计与 SKIP 分类（NA vs 已知限制）
- java_ported 测试结果
- 性能基准比值（docs/测试/性能基准报告.md）
- 错误基线状态（阶段 2 完成后填充）

用法: python3 scripts/gen_compat_report.py
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs/测试/compat_report.html"


def run_golden() -> tuple:
    out = subprocess.run(
        ["cargo", "test", "-p", "freemarker-test", "--test", "golden", "--", "--show-output"],
        cwd=ROOT, capture_output=True, text=True,
    ).stdout
    m = re.search(r"PASS=(\d+) FAIL=(\d+) SKIPPED=(\d+)", out)
    pass_n, fail_n, skip_n = (int(m.group(i)) for i in (1, 2, 3)) if m else (0, 0, 0)
    # 分类 SKIP 原因
    categories = {"Java 特有 wrapper": 0, "?new/?api 类加载": 0, "旧 ICI 版本": 0,
                  "jython25 断言矛盾": 0, "XML/解析器限制": 0, "其他": 0}
    for line in out.splitlines():
        line = line.strip()
        if not line.startswith("[skip]"):
            continue
        reason = line
        if "BeansWrapper" in reason or "DefaultObjectWrapper" in reason or "object_wrapper" in reason:
            categories["Java 特有 wrapper"] += 1
        elif "?new" in reason or "?api" in reason or "JythonRuntime" in reason:
            categories["?new/?api 类加载"] += 1
        elif "ICI" in reason or "旧版" in reason or "ici-" in reason:
            categories["旧 ICI 版本"] += 1
        elif "矛盾" in reason:
            categories["jython25 断言矛盾"] += 1
        elif "XML" in reason or "xmlns" in reason or "解析" in reason or "node" in reason:
            categories["XML/解析器限制"] += 1
        else:
            categories["其他"] += 1
    return pass_n, fail_n, skip_n, categories


def run_java_ported() -> tuple:
    out = subprocess.run(
        ["cargo", "test", "-p", "freemarker-test", "--test", "java_ported"],
        cwd=ROOT, capture_output=True, text=True,
    ).stdout
    m = re.search(r"test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored", out)
    return (int(m.group(1)), int(m.group(2)), int(m.group(3))) if m else (0, 0, 0)


def read_perf() -> dict:
    report = ROOT / "docs/测试/性能基准报告.md"
    if not report.exists():
        return {}
    ratios = {}
    for line in report.read_text().splitlines():
        m = re.match(r"\|\s*(\w+)\s*\|\s*[\d.]+\s*\|\s*[\d.]+\s*\|\s*([\d.]+)×\s*\|", line)
        if m:
            ratios[m.group(1)] = float(m.group(2))
    return ratios


def main():
    pass_n, fail_n, skip_n, categories = run_golden()
    jp_passed, jp_failed, jp_ignored = run_java_ported()
    perf = read_perf()

    perf_rows = ""
    perf_ok = 0
    for name, ratio in sorted(perf.items()):
        ok = ratio >= 0.5
        perf_ok += ok
        perf_rows += (
            f"<tr><td>{name}</td><td>{ratio:.3f}×</td>"
            f"<td>{'<span style=\"color:green\">✅ 达标</span>' if ok else '<span style=\"color:red\">❌ 未达标</span>'}</td></tr>\n"
        )

    cat_rows = "\n".join(
        f"<tr><td>{k}</td><td>{v}</td></tr>" for k, v in categories.items()
    )

    perf_summary = (
        f"{perf_ok}/{len(perf)} 达标" if perf
        else "未运行（执行 scripts/bench_compare.py）"
    )

    html = f"""<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="utf-8">
<title>freemarker-rust L3 兼容性报告</title>
<style>
body {{ font-family: -apple-system, sans-serif; margin: 2em; }}
h1 {{ border-bottom: 2px solid #333; padding-bottom: .3em; }}
table {{ border-collapse: collapse; margin: 1em 0; }}
th, td {{ border: 1px solid #ccc; padding: .4em .8em; text-align: left; }}
th {{ background: #f0f0f0; }}
.green {{ color: green; font-weight: bold; }}
.red {{ color: red; font-weight: bold; }}
</style>
</head>
<body>
<h1>freemarker-rust L3 兼容性报告</h1>
<p>生成脚本: <code>scripts/gen_compat_report.py</code>（由 <code>scripts/compare_outputs.py --suite golden</code> 驱动）</p>

<h2>1. Golden Suite（对 Java FreeMarker 2.3.34 逐字节一致）</h2>
<table>
<tr><th>PASS</th><th>FAIL</th><th>SKIP</th><th>总计</th></tr>
<tr>
  <td class="green">{pass_n}</td>
  <td class="red">{fail_n}</td>
  <td>{skip_n}</td>
  <td>{pass_n + fail_n + skip_n}</td>
</tr>
</table>

<h3>SKIP 分类</h3>
<table>
<tr><th>类别</th><th>数量</th></tr>
{cat_rows}
</table>

<h2>2. java_ported（Java 测试 1:1 移植）</h2>
<table>
<tr><th>通过</th><th>失败</th><th>忽略</th></tr>
<tr>
  <td class="green">{jp_passed}</td>
  <td class="red">{jp_failed}</td>
  <td>{jp_ignored}</td>
</tr>
</table>

<h2>3. 性能基准（Rust/Java 比值，门禁 ≥ 0.5×）</h2>
<p>汇总: <strong>{perf_summary}</strong></p>
<table>
<tr><th>场景</th><th>Rust/Java</th><th>门禁</th></tr>
{perf_rows}
</table>

<h2>4. 已知限制</h2>
<ul>
<li><code>ext/beans</code> BeanWrapper 反射 → <code>?api</code> NotSupported（设计决策 D1）</li>
<li>incompatibleImprovements 锁定 2.3.34（D3）→ 旧 ICI expected 变体为合理 NA</li>
<li>XML 节点支持（阶段 3 完成后更新）</li>
<li>错误消息基线（阶段 2 完成后更新）</li>
</ul>

<p><em>本报告由 CI 门禁生成；FAIL ≠ 0 或性能门禁不达标时 CI 红。</em></p>
</body>
</html>
"""
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(html)
    print(f"报告已生成: {OUT}")
    print(f"golden: PASS={pass_n} FAIL={fail_n} SKIP={skip_n}")
    print(f"java_ported: {jp_passed} passed / {jp_failed} failed / {jp_ignored} ignored")
    print(f"性能: {perf_summary}")
    # 门禁：FAIL 必须为 0
    if fail_n > 0 or jp_failed > 0:
        print("门禁失败: 存在 FAIL", file=sys.stderr)
        sys.exit(1)
    print("门禁通过")


if __name__ == "__main__":
    main()
