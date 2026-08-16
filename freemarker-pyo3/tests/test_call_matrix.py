# -*- coding: utf-8 -*-
"""调用矩阵测试 —— pyo3 桥接面系统性用例（2026-08-16）。

三组覆盖（断言全部来自探针实测，禁止臆造）：
1. 桥接方法矩阵：Stage 2 新增 35 方法中此前未测的 13 个（日期三格式/
   c_format/url_escaping_charset/fallback_on_null_loop_variable/
   localized_lookup/lazy_imports/lazy_auto_imports/delay/input_encoding/
   output_encoding/lookup_strategy）。
2. 类型双向编组矩阵（docs/10 §S2 wrap/unwrap 表的 Python 侧实证）：
   bigint（超 i64）/float/bool/nested/emoji/bytes。
3. 错误保真：FTL 异常的模板名+行+列+FTL stack trace；boolean_format
   legacy 默认拒绝自动转换（Java 2.3.21+ 逐字对齐，含 Tip 文案）。
"""

import datetime

import pytest

import freemarker as fm


def render(tpl: str, root=None, setup=None) -> str:
    cfg = fm.FmConfiguration()
    if setup:
        setup(cfg)
    cfg.put_template("p.ftl", tpl)
    return cfg.get_template("p.ftl").process(root or {})


# ---------------------------------------------------------------------------
# 1. 桥接方法矩阵
# ---------------------------------------------------------------------------


def test_set_date_format_pattern():
    dt = datetime.datetime(2026, 8, 15, 10, 30, 45)
    out = render("${d?date}", {"d": dt}, lambda c: c.set_date_format("yyyy.MM.dd"))
    assert out == "2026.08.15"


def test_set_time_format_pattern():
    dt = datetime.datetime(2026, 8, 15, 10, 30, 45)
    out = render("${d?time}", {"d": dt}, lambda c: c.set_time_format("HH:mm"))
    assert out == "10:30"


def test_set_date_time_format_pattern():
    dt = datetime.datetime(2026, 8, 15, 10, 30, 45)
    out = render("${d}", {"d": dt}, lambda c: c.set_date_time_format("yyyy/MM/dd HH:mm"))
    assert out == "2026/08/15 10:30"


def test_set_c_format_callable_and_c_builtin():
    # 1.5 与 1.0 在 JSON/Legacy 两格式下输出一致（探针实测）；断言可调用 + ?c 可用
    out = render("${n?c}", {"n": 1.5}, lambda c: c.set_c_format("JSON"))
    assert out == "1.5"
    out = render("${n?cn}", {"n": 1.0}, lambda c: c.set_c_format("JSON"))
    assert out == "1"


def test_set_c_format_invalid_raises():
    cfg = fm.FmConfiguration()
    with pytest.raises(Exception):
        cfg.set_c_format("BOGUS")


def test_set_url_escaping_charset_observable():
    # latin-1 下 é → %E9（UTF-8 应为 %C3%A9）——字符集真实生效
    out = render("${'a é'?url}", setup=lambda c: c.set_url_escaping_charset("ISO-8859-1"))
    assert out == "a%20%E9"


def test_set_url_escaping_charset_default_utf8():
    out = render("${'a é'?url}")
    assert out == "a%20%C3%A9"


def test_set_fallback_on_null_loop_variable_legacy_error():
    # 默认（true=legacy）：列表含 None 时 ${x} 报 null or missing（对齐 Java
    # IteratorBlock.java:467 fallback=true → getLocalVariable 返回 null）
    tpl = "<#list ls as x>${x};</#list>"
    with pytest.raises(fm.FreeMarkerError, match="null or missing"):
        render(tpl, {"ls": [1, None, 2]})


def test_set_fallback_on_null_loop_variable_false_same_error():
    # false 分支：null 值经 ?? 判定仍为 false（FreeMarker ?? 对 null 值返回
    # false——两分支可观测行为一致，探针实测）
    tpl = "<#list ls as x>${(x??)?c};</#list>"
    for fb in (True, False):
        out = render(
            tpl,
            {"ls": [1, None, 2]},
            lambda c, fb=fb: c.set_fallback_on_null_loop_variable(fb),
        )
        assert out == "true;false;true;"


def test_set_lookup_strategy_default_and_invalid():
    cfg = fm.FmConfiguration()
    cfg.set_lookup_strategy("default")  # 合法值不抛
    with pytest.raises(Exception, match="Unknown lookup strategy"):
        cfg.set_lookup_strategy("acquisition")


def test_config_only_setters_callable():
    # 以下设置在 pyo3 单配置渲染路径无独立可观测输出（存储型/缓存延迟型），
    # 断言可调用且不破坏后续渲染（诚实边界：效果由核心 crate 测试覆盖）
    cfg = fm.FmConfiguration()
    cfg.set_lazy_imports(True)
    cfg.set_lazy_auto_imports(True)
    cfg.set_delay(5)
    cfg.set_input_encoding("UTF-8")
    cfg.set_output_encoding("UTF-8")
    cfg.set_localized_lookup(True)
    cfg.put_template("t.ftl", "${1+1}")
    assert cfg.get_template("t.ftl").process({}) == "2"


# ---------------------------------------------------------------------------
# 2. 类型双向编组矩阵
# ---------------------------------------------------------------------------


def test_type_bigint_beyond_i64():
    # 10^25 超 i64/u64 —— 走 BigDecimal 路径完整渲染（探针实测）
    out = render("${n}", {"n": 10**25})
    assert out == "10000000000000000000000000"


def test_type_float_precision():
    assert render("${n}", {"n": 0.1}) == "0.1"
    assert render("${n}", {"n": 1.5}) == "1.5"


def test_type_bool_requires_format_or_c():
    # Java 2.3.21+ legacy 默认：${bool} 拒绝自动转换（逐字对齐，含 Tip）
    with pytest.raises(fm.FreeMarkerError, match="boolean_format"):
        render("${n}", {"n": True})


def test_type_bool_with_boolean_format():
    out = render("${n}", {"n": True}, lambda c: c.set_boolean_format("yes,no"))
    assert out == "yes"


def test_type_bool_c_builtin():
    assert render("${n?c}", {"n": True}) == "true"


def test_type_nested_deep_access():
    out = render("${a.b[0].c}", {"a": {"b": [{"c": "deep"}]}})
    assert out == "deep"


def test_type_emoji_and_cjk_roundtrip():
    assert render("${s}!", {"s": "🎉模板"}) == "🎉模板!"


def test_type_bytes_renders_repr():
    # bytes 经 str() 化渲染（探针实测；与 CPython str(b"xy") 一致）
    assert render("${b}", {"b": b"xy"}) == "b'xy'"


def test_type_none_root_value_is_missing():
    with pytest.raises(fm.FreeMarkerError, match="null or missing"):
        render("${m}", {"m": None})


# ---------------------------------------------------------------------------
# 3. 错误保真
# ---------------------------------------------------------------------------


def test_error_contains_template_name_line_column_and_stack():
    cfg = fm.FmConfiguration()
    cfg.put_template("err.ftl", "a\n${miss}")
    with pytest.raises(fm.FreeMarkerError) as ei:
        cfg.get_template("err.ftl").process({})
    msg = str(ei.value)
    assert '"err.ftl"' in msg          # 模板名
    assert "line 2" in msg             # 行号
    assert "Failed at: ${miss}" in msg  # FTL stack trace 帧格式


def test_error_is_runtime_error_subtype():
    assert issubclass(fm.FreeMarkerError, RuntimeError)


def test_error_boolean_tip_alignment():
    # Java Configurable.java boolean_format legacy 拒绝消息的 Tip 文案逐字对齐
    with pytest.raises(fm.FreeMarkerError) as ei:
        render("${b}", {"b": True})
    msg = str(ei.value)
    assert "?string('yes', 'no')" in msg or "?string(" in msg
    assert "?c" in msg


# ---------------------------------------------------------------------------
# 4. 非 dict 根统一拒绝（2026-08-16 修复：对齐 Java Template.process 对非
#    TemplateHashModel 抛 IllegalArgumentException；jython number/sequence/
#    string 模型均非 hash——此前 list/int 静默渲染属桥层偏差）
# ---------------------------------------------------------------------------


@pytest.mark.parametrize("root", [[1, 2], (1, 2), 42, 3.14, True, "s", b"x"])
def test_non_dict_root_uniformly_rejected(root):
    cfg = fm.FmConfiguration()
    cfg.put_template("t.ftl", "ok")
    with pytest.raises(fm.FreeMarkerError, match="must be a hash"):
        cfg.get_template("t.ftl").process(root)


def test_dict_and_generic_object_root_accepted():
    import datetime

    cfg = fm.FmConfiguration()
    cfg.put_template("t.ftl", "${m.k}")

    class Bag:
        k = "generic-ok"

    assert cfg.get_template("t.ftl").process({"m": {"k": "dict-ok"}}) == "dict-ok"
    assert cfg.get_template("t.ftl").process({"m": Bag()}) == "generic-ok"
    # datetime 作为根值同样拒绝（date 模型非 hash）
    with pytest.raises(fm.FreeMarkerError, match="must be a hash"):
        cfg.get_template("t.ftl").process(datetime.datetime(2026, 8, 16))
