# -*- coding: utf-8 -*-
"""Python 黄金套件 —— jython25 翻译（docs/10 §8 验收 1）。

逐目录参数化遍历 `freemarker-test/tests/suite/cases/*/`，对每个用例读取
`<base>.ftl` 模板 + `<base>.txt` expected（两侧剥掉 ASF license 注释头），
经 Python API（`fm.FmConfiguration` + `put_template` + `FmTemplate.process`，
空 dict 根数据模型）渲染后与 expected 逐字节比较。

测试台（等价 golden.rs 的 harness，见 freemarker-test/tests/golden.rs 与
tests/common/mod.rs）：
- 公共变量 `message` = "Hello, world!" 经 `set_shared_variable` 提供
  （golden.rs build_data_model 公共变量；须在 get_template 之前设置）。
- 共享模板目录 `tests/suite/templates/` 全部注册进 StringLoader
  （golden.rs load_all_templates，include/import 依赖模板）。
- 比较前归一化换行（\\r\\n|\\r → \\n）并忽略末尾换行差异
  （golden.rs normalize_newlines + FileTestCase.multilineAssertEquals）。

分类（与 golden.rs run_case 的 SKIP 逻辑一一对应）：
- Java 特有设置（object_wrapper 非 SimpleObjectWrapper / ?new 类解析器 /
  ?api 内建）→ SKIP；
- jython25 弃用套件矛盾用例（string-builtins3 / date-type-builtins）→ SKIP；
- 旧 ICI expected（encoding-builtins / string-builtins-ici-2.3.19 /
  type-builtins）→ SKIP；listhashliteral 以 EXPECTED_FILE_OVERRIDES 选
  ici-2.3.21 变体（与引擎 ICI 2.3.34 对齐）→ PASS（golden.rs 亦仅 SKIP
  ici-2.3.20 变体）；
- no_output 用例（无 expected 文件，依赖 assert/assertFails/noOutput
  测试台指令，Python 侧无法注入）→ SKIP；
- 需要 Java Bean/多角色数据模型（bean-maps/varargs/var-layers/...）→ SKIP；
- 需要 Java 配置设置（locale/time_zone/classic_compatible/auto_import/
  output_encoding/strict_syntax，Python API 未暴露）→ SKIP；
- transform 指令需 JythonRuntime（jython25 未迁移）→ SKIP；
- 非 UTF-8 模板字节（charset-in-header）→ SKIP。

分类是显式的（PASS_CASES / SKIP_REASONS 表）：被分类为 pass 的用例必须
逐字节通过；被分类为 skip 的用例若开始通过（引擎能力增长）会 FAIL，提示
重新分类——防止 SKIP 静默漂移。
运行：pytest freemarker-pyo3/tests/test_golden_suite.py -q
"""
from pathlib import Path

import pytest

import freemarker as fm

SUITE_DIR = Path(__file__).resolve().parents[2] / "freemarker-test" / "tests" / "suite"
CASES_DIR = SUITE_DIR / "cases"
SHARED_DIR = SUITE_DIR / "templates"

# golden.rs build_data_model 公共变量（TemplateTestCase.java:184-193 的 message）；
# assert/assertEquals/assertFails/noOutput 指令、testName/iciIntValue/javaObjectInfo
# 无法经 Python API 注入 → 依赖它们的用例一律 SKIP（见 EXPECTED）。
MESSAGE = "Hello, world!"

# ---------------------------------------------------------------------------
# 文件处理（镜像 freemarker-test/tests/common/mod.rs 与 golden.rs）
# ---------------------------------------------------------------------------

def strip_ftl_copyright(src: str) -> str:
    """移除模板开头的 `<#-- ... -->` / `[#-- ... --]` 版权注释块。

    镜像 remove_ftl_copyright_comment_bytes：ASCII 查找 "copyright"，
    取其前最后一个注释起始标记，截到注释结束标记后并吞掉一个换行。
    """
    lower = src.lower()
    idx = lower.find("copyright")
    if idx < 0:
        return src
    before = src[:idx]
    ab = before.rfind("<#--")
    sb = before.rfind("[#--")
    if ab < 0 and sb < 0:
        return src
    if sb > ab:
        start, end_marker = sb, "--]"
    else:
        start, end_marker = ab, "-->"
    after = src[start:]
    end_pos = after.find(end_marker)
    if end_pos < 0:
        return src
    rest = src[start + end_pos + 3:]
    if rest.startswith("\r\n"):
        rest = rest[2:]
    elif rest.startswith("\n"):
        rest = rest[1:]
    return src[:start] + rest


def strip_expected_license(s: str) -> str:
    """剥掉 expected 文件开头的 `/* ... */` 许可证块（含其后的一个换行）。

    镜像 strip_license_comment（FileTestCase 的 CopyrightCommentRemover）。
    """
    s = s.lstrip()
    if s.startswith("/*"):
        i = s.find("*/")
        if i >= 0:
            out = s[i + 2:]
            if out.startswith("\r\n"):
                return out[2:]
            if out.startswith("\n"):
                return out[1:]
            return out
    return s


def normalize_newlines(s: str) -> str:
    """`\\r\\n` → `\\n`、`\\r` → `\\n`（golden.rs normalize_newlines）。"""
    return s.replace("\r\n", "\n").replace("\r", "\n")


def read_expected(base: str) -> str:
    """读取 expected 并剥 license；UTF-8 失败时按 UTF-16 回退
    （output-encoding2 的 expected 为 UTF-16 BE）。

    多变体 expected 目录（listhashliteral）经 EXPECTED_FILE_OVERRIDES
    指定与引擎 ICI（2.3.34）对齐的变体。"""
    rel = EXPECTED_FILE_OVERRIDES.get(base, f"{base}.txt")
    raw = (CASES_DIR / base / rel).read_bytes()
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError:
        text = raw.decode("utf-16")
    return normalize_newlines(strip_expected_license(text))


# ---------------------------------------------------------------------------
# 测试台（等价 golden.rs 的 base_config + load_all_templates + 公共变量）
# ---------------------------------------------------------------------------

def remove_ftl_copyright_bytes(ftl: bytes) -> bytes:
    """字节版版权剥离（镜像 Rust remove_ftl_copyright_comment_bytes / common/mod.rs:88）：
    ASCII 小写查找 "copyright"，取其前最后一个注释起始标记，截到结束标记后并吞一换行。
    对任意编码安全（标记与版权词均为 ASCII）。"""
    lower = bytes(b | 0x20 if 65 <= b <= 90 else b for b in ftl)
    idx = lower.find(b"copyright")
    if idx < 0:
        return ftl
    before = lower[:idx]
    ab = before.rfind(b"<#--")
    sb = before.rfind(b"[#--")
    if ab < 0 and sb < 0:
        return ftl
    if sb > ab:
        start, end_marker = sb, b"--]"
    else:
        start, end_marker = ab, b"-->"
    end = lower.find(end_marker, idx)
    if end < 0:
        return ftl
    out = ftl[:start] + ftl[end + len(end_marker):]
    # 吞掉紧随的一个换行
    if out[start:start + 1] == b"\r":
        out = out[:start] + out[start + 2:]
    elif out[start:start + 1] == b"\n":
        out = out[:start] + out[start + 1:]
    return out


def load_shared_templates(cfg: fm.FmConfiguration) -> None:
    """注册 tests/suite/templates/ 下全部模板（相对路径，字节版剥版权头）。

    2026-08-16 起全量按原始字节注册（put_template_bytes）——镜像 Rust
    load_all_templates（common/mod.rs:75-86）：非 UTF-8 模板（charset-in-header
    系列）不再跳过，read_encoded 按声明编码解码。
    """
    for p in sorted(SHARED_DIR.rglob("*.ftl")):
        rel = p.relative_to(SHARED_DIR).as_posix()
        cfg.put_template_bytes(rel, remove_ftl_copyright_bytes(p.read_bytes()))


def build_config() -> fm.FmConfiguration:
    """每用例独立配置：message 共享变量 + 共享模板 + 用例模板。

    set_shared_variable 必须在 get_template 之前（get_template 快照配置，
    见 freemarker-pyo3/src/lib.rs FmConfiguration::get_template 注释）。
    """
    cfg = fm.FmConfiguration()
    # 与 Rust golden harness 默认对齐（freemarker-test/tests/common/mod.rs:65
    # strict_syntax = true；引擎 strict 模式下兼容 jython25 旧式标签，探针实测
    # number-literal 仅在 strict=True 下逐字节通过）。
    cfg.set_strict_syntax(True)
    cfg.set_shared_variable("message", MESSAGE)
    load_shared_templates(cfg)
    return cfg


# ---------------------------------------------------------------------------
# 用例级设置（对应 manifest.json 的 settings 字段）
# ---------------------------------------------------------------------------

#: 用例级设置映射（base → settings dict）；缺失的用例走默认配置。
CASE_SETTINGS = {
    "import": {"auto_import": "import_lib.ftl as my"},
    "localization": {"locale": "en_AU"},
    "number-literal": {"locale": "fr_FR"},
    # manifest 用例级 strict_syntax 覆盖 harness 默认（true）
    "non-strict-syntax": {"strict_syntax": "N"},
    "strictinheader": {"strict_syntax": "N"},
    # 编码类（process_bytes / put_template_bytes 翻转，2026-08-16）
    "charset-in-header": {"input_encoding": "ISO-8859-5", "clear_encoding_map": "Y"},
    "output-encoding2": {"output_encoding": "UTF-16"},
    "output-encoding3": {"output_encoding": "ISO-8859-1", "url_escaping_charset": "UTF-16"},
}


def apply_settings(cfg: fm.FmConfiguration, settings: dict) -> None:
    """应用用例级设置（对应 golden.rs apply_settings）。"""
    for k, v in settings.items():
        if k == "locale":
            cfg.set_locale(v)
        elif k == "strict_syntax":
            # golden.rs：v == "Y" || v == "1"（兼容 bool 形态）
            cfg.set_strict_syntax(v in (True, "Y", "1", "true"))
        elif k == "input_encoding":
            cfg.set_input_encoding(v)
        elif k == "output_encoding":
            cfg.set_output_encoding(v)
        elif k == "url_escaping_charset":
            cfg.set_url_escaping_charset(v)
        elif k == "clear_encoding_map":
            pass  # Rust golden 同款 no-op（common/mod.rs:213）
        elif k == "auto_import":
            # Java SettingStringParser.parseAsImportList："path as ns" 逗号分隔
            for item in v.split(","):
                item = item.strip()
                if not item:
                    continue
                if " as " in item:
                    path, ns = item.split(" as ", 1)
                    cfg.set_auto_import(ns.strip(), path.strip())


def localized_template_name(cfg: fm.FmConfiguration, name: str) -> str:
    """局部化模板名回退（对应 golden.rs get_template_localized /
    TemplateCache.lookupWithLocalizedThenAcquisitionStrategy）：
    "foo.ftl" + locale "en_AU" → 依次尝试 foo_en_AU.ftl、foo_en.ftl、foo.ftl，
    首个在 StringLoader 中存在的使用。"""
    locale = cfg.locale
    if not locale or locale == "en_US":
        return name
    last_dot = name.rfind(".")
    if last_dot < 0:
        prefix, suffix = name, ""
    else:
        prefix, suffix = name[:last_dot], name[last_dot:]
    # 逐级缩短 locale：en_AU → en → ""
    loc = f"_{locale}"
    while True:
        candidate = f"{prefix}{loc}{suffix}"
        try:
            cfg.get_template(candidate)
            return candidate
        except fm.FreeMarkerError:
            pass
        # 去掉最后一段
        underscore = loc.rfind("_", 1)
        if underscore < 0:
            break
        loc = loc[:underscore]
    return name


#: 输出为非 UTF-8 编码的用例（process_bytes 字节路径；expected 为转码字节文件）
BYTES_OUTPUT_CASES = {"output-encoding2", "output-encoding3"}


def decode_output_bytes(data: bytes, encoding: str) -> str:
    """按 output_encoding 解码转码字节为 str（镜像 Rust common/mod.rs decode_bytes）。"""
    enc = (encoding or "UTF-8").upper()
    if enc == "ISO-8859-1":
        return data.decode("latin-1")
    if "UTF-16" in enc:
        bom_be = data[:2] == b"\xfe\xff"
        bom_le = data[:2] == b"\xff\xfe"
        if bom_be or bom_le:
            data = data[2:]
            return data.decode("utf-16-be" if bom_be else "utf-16-le")
        return data.decode("utf-16-le" if "LE" in enc else "utf-16-be")
    return data.decode("utf-8", errors="replace")


def render_case(base: str):
    """渲染用例模板。成功返回 (out, None)；失败返回 (None, 错误消息)。"""
    ftl = CASES_DIR / base / f"{base}.ftl"
    if not ftl.exists():
        return None, "目录内无 <base>.ftl（manifest 变体模板名）"
    raw = ftl.read_bytes()
    cfg = build_config()
    # 应用用例级设置（须在 put_template / get_template 之前）
    if base in CASE_SETTINGS:
        apply_settings(cfg, CASE_SETTINGS[base])
    # 非 UTF-8 模板（charset-in-header：ISO-8859-5 字节）按原始字节注册
    try:
        src = raw.decode("utf-8")
        cfg.put_template(f"{base}.ftl", strip_ftl_copyright(src))
    except UnicodeDecodeError:
        cfg.put_template_bytes(f"{base}.ftl", remove_ftl_copyright_bytes(raw))
    # 局部化模板查找（locale 非 en_US 时尝试变体名）
    tpl_name = localized_template_name(cfg, f"{base}.ftl")
    try:
        template = cfg.get_template(tpl_name)
    except fm.FreeMarkerError as e:
        return None, f"解析失败：{e}"
    try:
        if base in BYTES_OUTPUT_CASES:
            # output_encoding 非 UTF-8：经 process_bytes 取转码字节，再按该编码
            # 解码回 str 比较（镜像 Rust render_case 的 decode 回路，common/mod.rs:256-261）
            out_bytes = template.process_bytes({})
            return decode_output_bytes(out_bytes, cfg.output_encoding), None
        return template.process({}), None
    except fm.FreeMarkerError as e:
        return None, f"渲染失败：{e}"


def matches_expected(base: str, out: str) -> bool:
    """golden.rs 的逐字节比较：归一化 + 忽略末尾换行差异。"""
    exp = read_expected(base)
    out = normalize_newlines(out)
    if out.endswith("\n") and not exp.endswith("\n"):
        exp += "\n"
    elif not out.endswith("\n") and exp.endswith("\n"):
        exp = exp.rstrip("\n")
    return out == exp


def diff_preview(base: str, out: str) -> str:
    """输出差异预览（golden.rs diff_preview：首处不同 ±30 字节）。"""
    exp = read_expected(base)
    out = normalize_newlines(out)
    common = 0
    for a, b in zip(out, exp):
        if a != b:
            break
        common += 1
    a = out[common - 30:common + 40]
    b = exp[common - 30:common + 40]
    return (
        f"字节 {common} 起不同：\n"
        f"  actual:   {a!r}\n"
        f"  expected: {b!r}\n"
        f"  [actual len={len(out)}, expected len={len(exp)}]"
    )


# ---------------------------------------------------------------------------
# 显式分类表（golden.rs run_case SKIP 逻辑的 Python 翻译）
# ---------------------------------------------------------------------------

#: 逐字节 PASS 的用例（34 个；空根 + message 共享变量 + 共享模板）
#: - listhashliteral：expected 用 ICI >=2.3.21 变体（引擎固定 ICI 2.3.34 的重复键
#:   覆盖语义；golden.rs 仅 SKIP ici-2.3.20 变体，ici-2.3.21 变体 PASS）。
#: - new-unrestricted：manifest 含 new_builtin_class_resolver 设置（golden.rs 设置
#:   分类为 SKIP），但模板仅用引擎内置 ?new 测试夹具类（NewTestModel/
#:   ObjectConstructor，见 freemarker/src/template/utility_transforms.rs），
#:   空根即可渲染且逐字节匹配 → 实测 PASS，归类为 pass。
PASS_CASES = {
    "arithmetic", "comment", "compress", "default", "escapes",
    "hashconcat", "identifier-escaping", "identifier-non-ascii", "import",
    "include", "interpret", "iterators", "lastcharacter", "listhashliteral",
    "listliteral", "localization", "loopvariable", "macros-return", "macros2",
    "nested", "new-defaultresolver", "new-unrestricted", "newlines1", "newlines2",
    "non-strict-syntax", "noparse", "number-literal", "numerical-cast",
    "charset-in-header", "output-encoding1", "output-encoding2", "output-encoding3",
    "precedence", "root", "strictinheader",
    "string-builtins2", "stringliteral", "variables", "whitespace-trim",
    "wstrip-in-header",
}

#: expected 文件覆盖（多变体目录：选与引擎 ICI 2.3.34 对齐的变体）
EXPECTED_FILE_OVERRIDES = {
    "listhashliteral": "listhashliteral-ici-2.3.21.txt",
}

#: SKIP 原因（golden.rs 同款分类；键 = base 目录名）
SKIP_REASONS = {
    # --- Java 特有设置（golden.rs run_case：object_wrapper / ?new / ?api）---
    "api-builtins": "?api 内建 + object_wrapper=DefaultObjectWrapper（Java BeanWrapper 特有）",
    "beans": "object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper）",
    "default-xmlns": "object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper）",
    "list": "object_wrapper=DefaultObjectWrapper(2.3.22,...)（Java 特有 wrapper）",
    "list-bis": "object_wrapper=DefaultObjectWrapper(2.3.22,...)（Java 特有 wrapper）",
    "list2": "object_wrapper=DefaultObjectWrapper(2.3.22,...)（Java 特有 wrapper）",
    "list3": "object_wrapper=DefaultObjectWrapper(2.3.22,...)（Java 特有 wrapper）",
    "new-allowsnothing": "?new 类解析器（new_builtin_class_resolver，Java 特有）",
    "new-optin": "?new 类解析器（new_builtin_class_resolver，Java 特有）",
    "new-safer": "?new 类解析器（new_builtin_class_resolver，Java 特有）",
    "overloaded-methods-2-bwici-2.3.21": "object_wrapper=BeansWrapper 变体（Java 特有 wrapper）",
    "overloaded-methods-2-desc-bwici-2.3.20": "object_wrapper=BeansWrapper 变体（Java 特有 wrapper）",
    "overloaded-methods-2-inc-bwici-2.3.20": "object_wrapper=BeansWrapper 变体（Java 特有 wrapper）",
    "overloaded-methods-23bc": "object_wrapper=BeansWrapper 变体（Java 特有 wrapper）",
    "sequence-builtins": "object_wrapper=BeansWrapper/DefaultObjectWrapper（Java 特有 wrapper）",
    "xml-fragment": "object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper）",
    "xmlns1": "object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper）",
    "xmlns3": "object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper）",
    "xmlns4": "object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper）",
    "xmlns5": "object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper）",

    # --- jython25 弃用套件矛盾（golden.rs：用例断言与真实 Java 引擎矛盾）---
    "string-builtins3": "用例断言与真实 Java 引擎矛盾（-1?lower_abc 解析为 -(1?lower_abc)；jython25 弃用模块过期断言）",
    "date-type-builtins": "用例断言与真实 Java 引擎矛盾（?string.xs 对 date-only 输出带 Z；jython25 弃用模块过期断言）",

    # --- 旧 ICI expected（本引擎固定 ICI 2.3.34，无法对齐）---
    # 注：listhashliteral 的 ici-2.3.21 变体与引擎对齐 → PASS（见 PASS_CASES）；
    # 仅 ici-2.3.20 变体（保留重复键）无法对齐。目录级参数化以
    # EXPECTED_FILE_OVERRIDES 选 2.3.21 变体。
    "encoding-builtins": "expected 由 ICI <2.3.20 的旧版 ?html 行为生成（不转义 '），本引擎固定 ICI 2.3.34",
    "string-builtins-ici-2.3.19": "expected 由 ICI 2.3.19 的旧版 ?html 行为生成（不转义 '），本引擎固定 ICI 2.3.34（转义）",
    "type-builtins": "expected 由 ICI <2.3.24 行为生成（方法模型 ?is_sequence/?is_enumerable 不排除），本引擎固定 ICI 2.3.34（排除）",

    # --- no_output 用例（无 expected 文件；依赖 Java 测试台指令）---
    "assignments": "no_output 用例，依赖 Java 测试台指令 assert（Python 侧无法注入）",
    "classic-compatible-mode2": "no_output 用例 + classic_compatible=2 设置（Java 特有）",
    "dateformat-iso-bi": "no_output 用例，依赖 Java 测试台指令 assert",
    "dateformat-iso-bi-ici-2.3.21": "no_output 用例，依赖 Java 测试台指令 assert",
    "dateformat-iso-like": "no_output 用例，依赖 Java 测试台指令 assert",
    "dateparsing": "no_output 用例，依赖 Java 测试台指令 assert",
    "existence-operators": "no_output 用例，依赖 Java 测试台指令 assert",
    "number-math-builtins": "no_output 用例，依赖 Java 测试台数据模型（fNan/dPinf/...）",
    "range-ici-2.3.20": "no_output 用例，依赖 Java 测试台指令 assert",
    "range-ici-2.3.21": "no_output 用例，依赖 Java 测试台指令 assert",
    "range-lazy": "no_output 用例，依赖 Java 测试台指令 assert",
    "setting": "no_output 用例，依赖 Java 测试台指令 assert",
    "simplehash-char-key": "no_output 用例，依赖 Java 测试台数据模型（mStringC/mCharC/...）",
    "string-builtin-coercion": "no_output 用例，依赖 Java 测试台指令 assert",
    "string-builtins-ici-2.3.20": "no_output 用例，依赖 Java 测试台指令 assert",
    "switch-builtin": "no_output 用例，依赖 Java 测试台指令 assert",
    "then-builtin": "no_output 用例，依赖 Java 测试台指令 assert",
    "url": "no_output 用例，依赖 Java 测试台指令 assert",

    # --- 模板字节/文件名（非引擎能力）---
    "xml-ns_prefix-scope": "目录内无 <base>.ftl（manifest 变体模板名 xml-ns_prefix-scope-main.ftl）",
    "xmlns2": "目录内无 <base>.ftl（manifest 变体：与 xmlns1 同模板）",

    # --- 依赖 Java 测试台指令（assert/assertEquals/assertFails/noOutput）---
    "boolean": "依赖 Java 测试台指令 assert + 数据模型（boolean1/hash1/...）",
    "boolean-formatting": "依赖 Java 测试台指令 noOutput + beans 数据模型（beansBoolean/...）",
    "comparisons": "依赖 Java 测试台指令 assert/assertFails/noOutput + 数据模型",
    "hashliteral": "依赖 Java 测试台指令 assertEquals/noOutput + iciIntValue",
    "if": "依赖 Java 测试台指令 assertFails",
    "include2": "依赖 Java 测试台指令 assertFails",
    "macros": "依赖 Java 测试台指令 assertFails",
    "recover": "依赖 Java 测试台指令 assert/assertFails",
    "string-builtins-regexps": "依赖 Java 测试台指令 assert",
    "string-builtins-regexps-matches": "依赖 Java 测试台指令 assertEquals",
    "string-builtins1": "依赖 Java 测试台指令 assertEquals",
    "switch": "依赖 Java 测试台指令 assertFails",

    # --- 依赖 Java Bean/多角色数据模型（build_data_model 专用模型）---
    "bean-maps": "需要 Java Bean 数据模型（TestMapBean/TestBean 属性）",
    "dateformat-java": "需要 Java 数据模型（date/sqlDate/sqlTime/...）",
    "listhash": "需要 Java 数据模型（listables 序列家族）",
    "multimodels": "需要 Java 多角色数据模型（MultiModel1-5）",
    "number-format": "需要 Java 数据模型（int/double/bigDecimal/...）",
    "number-to-date": "需要 Java 数据模型（bigInteger/bigDecimal 时间戳）",
    "specialvars": "需要 Java 测试台数据模型（.vars 探测）",
    "stringbimethods": "需要 Java 双角色数据模型（multi = TestBoolean）",
    "var-layers": "需要 Java 数据模型（x/z/y 变量分层）",
    "varargs": "需要 Java Bean 方法模型（VarArgTestModel 签名调度）",

    # --- 依赖 Java 配置设置（部分已翻转，剩余保持 SKIP）---
    "classic-compatible": "classic_compatible=Y 设置 + Java 数据模型（beansArray/...）",

    # --- jython25 未迁移的 Java 运行时 ---
    "transforms": "transform 指令需要 JythonRuntime（java.lang.ClassNotFoundException；jython25 未迁移）",
}

ALL_CASES = sorted(PASS_CASES | set(SKIP_REASONS))


# ---------------------------------------------------------------------------
# 参数化测试
# ---------------------------------------------------------------------------

def _assert_equal_expected(base: str, out: str) -> None:
    if not matches_expected(base, out):
        pytest.fail(f"用例 {base} 输出与 expected 不一致：\n{diff_preview(base, out)}")


@pytest.mark.parametrize("base", ALL_CASES)
def test_case(base: str) -> None:
    """每用例独立验证：pass 分类必须逐字节通过；skip 分类必须仍不可通过。"""
    out, err = render_case(base)

    if base in PASS_CASES:
        # 被分类为 pass 的用例：渲染失败即测试失败（不允许静默漂移）
        if err is not None:
            pytest.fail(f"用例 {base}（分类 pass）渲染失败：{err}")
        _assert_equal_expected(base, out)
        return

    reason = SKIP_REASONS[base]
    if err is not None:
        # 如预期失败 → skip（错误消息仅作诊断信息）
        pytest.skip(f"{reason}（{err.splitlines()[0]}）")
    # 渲染成功但被分类为 skip：校验确实"不可通过"
    has_expected = (CASES_DIR / base / f"{base}.txt").exists()
    if has_expected:
        if matches_expected(base, out):
            pytest.fail(
                f"用例 {base} 已可逐字节通过（当前分类为 skip：{reason}），应重新分类为 pass"
            )
    else:
        # no_output 语义：渲染成功无报错即视为"可通过"
        pytest.fail(
            f"用例 {base} 已可渲染成功（no_output 语义；当前分类为 skip：{reason}），应重新分类为 pass"
        )
    pytest.skip(reason)


def on_disk_cases():
    """套件目录下全部子目录（109 个用例目录）。"""
    return [p.name for p in CASES_DIR.iterdir() if p.is_dir()]


def test_golden_suite_coverage() -> None:
    """套件目录与分类表一一对应（新增用例目录必须显式分类，防漏网）。"""
    on_disk = set(on_disk_cases())
    missing = sorted(on_disk - set(ALL_CASES))
    extra = sorted(set(ALL_CASES) - on_disk)
    assert not missing, f"未分类的用例目录：{missing}"
    assert not extra, f"分类表中不存在的目录：{extra}"
    assert len(ALL_CASES) == len(PASS_CASES) + len(SKIP_REASONS)


def test_all_selected_cases_pass_or_classified() -> None:
    """整体断言：分类完整，且 PASS 数满足阈值（docs/10 §8 验收 1）。"""
    assert len(PASS_CASES) >= 20, (
        f"Python 黄金套件要求 >= 20 个逐字节 PASS，当前 {len(PASS_CASES)}"
    )
    assert len(PASS_CASES) + len(SKIP_REASONS) == len(on_disk_cases()), (
        f"分类不完整：pass={len(PASS_CASES)} skip={len(SKIP_REASONS)} "
        f"目录={len(on_disk_cases())}"
    )


if __name__ == "__main__":
    import sys
    sys.exit(pytest.main([__file__, "-v"]))
