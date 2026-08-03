#!/usr/bin/env python3
"""提取 Java 黄金测试套件（testcases.xml + 模板 + 期望输出）到 freemarker-rust 测试目录。

对应 docs/11 §3：testcases.xml 是语义一致性的黄金判据。
用法：
    python3 scripts/extract_suite.py
输出：
    freemarker/tests/suite/cases/<name>/{name}.ftl + {name}.expected.txt
    freemarker/tests/suite/manifest.json   （name → settings/template/expected 映射）

注意：
- 模板文件默认 = name 去掉 "[#endTN]" 后缀 + ".ftl"；expected 同理
- settings 保留原样（runner 按能力执行子集）
- 本脚本仅提取数据，不修改 src/ 下任何代码
"""

import json
import os
import re
import shutil
import xml.etree.ElementTree as ET

# Java 仓库 templatesuite 路径（开发工具：仅提取时使用，CI 不运行；
# 可用 FREEMARKER_JAVA_SUITE 环境变量覆盖）
JAVA_SUITE = os.environ.get(
    "FREEMARKER_JAVA_SUITE",
    "/Users/wandl/workspaces/workspace-github/freemarker/freemarker-jython25/src/test/resources/freemarker/test/templatesuite",
)
RUST_SUITE = os.path.join(os.path.dirname(__file__), "..", "freemarker", "tests", "suite")

END_TN = "[#endTN]"


def norm_name(name: str) -> str:
    """Java: template 默认 = name?keep_before('[#endTN]') + '.ftl'"""
    return name.split(END_TN)[0]


def main():
    tree = ET.parse(os.path.join(JAVA_SUITE, "testcases.xml"))
    root = tree.getroot()
    manifest = {"cases": []}
    copied = 0
    for case in root.findall("testCase"):
        name = case.get("name")
        base = norm_name(name)
        template_attr = case.get("template")
        expected_attr = case.get("expected")
        no_output = (case.get("noOutput") or "false").lower() == "true"

        template_name = template_attr if template_attr else base + ".ftl"
        # Java TemplateTestSuite:210：expected = ATTR_EXPECTED 优先，否则
        # beforeEndTN + afterEndTN + ".txt"（[#endTN] 标记后片段要保留）
        expected_name = expected_attr if expected_attr else name.replace(END_TN, "") + ".txt"

        settings = {}
        for s in case.findall("setting"):
            for k, v in s.attrib.items():
                settings[k] = v

        # 复制模板与期望文件
        tpl_src = os.path.join(JAVA_SUITE, "templates", template_name)
        tpl_dst_dir = os.path.join(RUST_SUITE, "cases", base)
        os.makedirs(tpl_dst_dir, exist_ok=True)
        tpl_dst = os.path.join(tpl_dst_dir, template_name)
        if os.path.exists(tpl_src):
            shutil.copy2(tpl_src, tpl_dst)
            copied += 1
        else:
            tpl_dst = None

        exp_dst = None
        if not no_output:
            exp_src = os.path.join(JAVA_SUITE, "expected", expected_name)
            exp_dst = os.path.join(tpl_dst_dir, expected_name)
            if os.path.exists(exp_src):
                shutil.copy2(exp_src, exp_dst)
                copied += 1
            else:
                exp_dst = None

        manifest["cases"].append({
            "name": name,
            "base": base,
            "template": template_name,
            "template_file": tpl_dst,
            "expected_file": exp_dst,
            "no_output": no_output,
            "settings": settings,
        })

    manifest_path = os.path.join(RUST_SUITE, "manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)

    print(f"提取完成：{len(manifest['cases'])} 个用例，复制 {copied} 个文件")
    print(f"manifest: {manifest_path}")


if __name__ == "__main__":
    main()
