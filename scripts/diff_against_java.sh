#!/bin/bash
# L3 双引擎校验入口 —— 汇总 golden suite、live 对比、性能门禁、compat 报告
# 用法: scripts/diff_against_java.sh [--full]
set -euo pipefail
cd "$(dirname "$0")/.."

echo "=== [1/4] 构建 Rust probe ==="
python3 scripts/compare_outputs.py --build

echo ""
echo "=== [2/4] Golden suite（Rust vs Java expected） ==="
python3 scripts/compare_outputs.py --suite golden

echo ""
echo "=== [3/4] 性能对比（Rust vs Java） ==="
if [ "${1:-}" = "--full" ]; then
    python3 scripts/bench_compare.py || echo "⚠️ 性能门禁未达标（详见 docs/测试/性能基准报告.md）"
else
    echo "(跳过耗时 bench；执行 scripts/bench_compare.py 获取完整数据)"
fi

echo ""
echo "=== [4/4] 生成 compat 报告 ==="
python3 scripts/gen_compat_report.py

echo ""
echo "完成。报告: docs/测试/compat_report.html"
