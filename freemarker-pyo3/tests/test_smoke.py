# -*- coding: utf-8 -*-
"""freemarker-pyo3 Python 侧 smoke 测试（docs/10 §8 验收 1：helloworld/宏/dict/list/
函数/错误捕获；等价 freemarker-jython25 套件子集）。

运行：python3 -m pytest tests/test_smoke.py（或直接 python3 tests/test_smoke.py）
"""
import datetime
import sys

import pytest

import freemarker as fm


def make_cfg():
    cfg = fm.FmConfiguration()
    return cfg


def render(src, root, name="t.ftl"):
    cfg = make_cfg()
    cfg.put_template(name, src)
    return cfg.get_template(name).process(root)


# ---------------------------------------------------------------------------
# helloworld（黄金套件核心用例）
# ---------------------------------------------------------------------------

def test_helloworld():
    out = render("Hello, ${name}!", {"name": "world"})
    assert out == "Hello, world!"


def test_helloworld_unicode():
    out = render("你好，${name}！", {"name": "世界"})
    assert out == "你好，世界！"


# ---------------------------------------------------------------------------
# 宏
# ---------------------------------------------------------------------------

def test_macro():
    out = render(
        "<#macro greet who>Hello, ${who}!</#macro><@greet who='FM'/>", {}
    )
    assert out == "Hello, FM!"


def test_macro_with_nested_body():
    out = render(
        "<#macro wrap><b><#nested></b></#macro><@wrap>bold</@wrap>", {}
    )
    assert out == "<b>bold</b>"


# ---------------------------------------------------------------------------
# dict/list 数据
# ---------------------------------------------------------------------------

def test_list_iteration():
    out = render("<#list items as i>${i};</#list>", {"items": [1, 2, 3]})
    assert out == "1;2;3;"


def test_dict_keys_and_lookup():
    out = render(
        "<#list data?keys as k>${k}=${data[k]};</#list>",
        {"data": {"a": 1, "b": 2}},
    )
    assert out == "a=1;b=2;"


def test_nested_dict_dot_access():
    out = render(
        "${user.name} (${user.age})",
        {"user": {"name": "Ann", "age": 30}},
    )
    assert out == "Ann (30)"


def test_tuple_is_sequence():
    out = render("<#list t as x>${x};</#list>", {"t": (7, 8)})
    assert out == "7;8;"


# ---------------------------------------------------------------------------
# Python 函数作为模板方法
# ---------------------------------------------------------------------------

def test_python_function_as_method():
    out = render(
        "${greet('world')}",
        {"greet": lambda n: "Hello " + n},
    )
    assert out == "Hello world"


def test_python_function_receives_numbers_and_strings():
    out = render(
        "${add(2, 3)}|${upper('ab')}",
        {"add": lambda a, b: a + b, "upper": lambda s: s.upper()},
    )
    assert out == "5|AB"


def test_python_function_receives_engine_dict():
    # 模板内构造的 hash → Python 函数收到 dict（unwrap 方向）
    out = render(
        "<#assign h = {'x': 1}>${pick(h)}",
        {"pick": lambda d: d["x"]},
    )
    assert out == "1"


def test_python_function_receives_engine_list():
    out = render(
        "<#assign l = [1, 2, 3]>${total(l)}",
        {"total": lambda lst: sum(lst)},
    )
    assert out == "6"


def test_python_function_returning_none_is_missing():
    out = render("${f()!'-'}", {"f": lambda: None})
    assert out == "-"


# ---------------------------------------------------------------------------
# 模板错误捕获（docs/10 §5：自定义异常，消息含模板名）
# ---------------------------------------------------------------------------

def test_missing_variable_error_contains_template_name():
    with pytest.raises(fm.FreeMarkerError) as ei:
        render("Before ${missing} after", {}, name="err.ftl")
    msg = str(ei.value)
    assert "missing" in msg
    assert "err.ftl" in msg
    assert "[in template" in msg


def test_python_exception_bridged():
    with pytest.raises(fm.FreeMarkerError) as ei:
        render("${boom()}", {"boom": lambda: 1 / 0}, name="boom.ftl")
    msg = str(ei.value)
    assert "ZeroDivisionError" in msg
    assert "division by zero" in msg
    assert "boom.ftl" in msg


def test_template_not_found():
    cfg = make_cfg()
    with pytest.raises(fm.FreeMarkerError) as ei:
        cfg.get_template("no_such.ftl")
    assert "no_such.ftl" in str(ei.value)


# ---------------------------------------------------------------------------
# PyObjectWrapper（attributes_shadow_items 可构造参数）
# ---------------------------------------------------------------------------

def test_object_wrapper_attributes_shadow_items():
    w = fm.PyObjectWrapper()
    assert w.attributes_shadow_items is True
    w.attributes_shadow_items = False
    assert w.attributes_shadow_items is False
    w2 = fm.PyObjectWrapper(attributes_shadow_items=False, use_cache=True)
    assert w2.attributes_shadow_items is False
    assert w2.use_cache is True


def test_set_object_wrapper():
    class Obj:
        attr_name = "via-attr"
        def __getitem__(self, k):
            return "via-item:" + k
    # 默认 wrapper（attributes_shadow_items=True）：getattr 优先
    cfg = make_cfg()
    cfg.put_template("w.ftl", "${obj.attr_name}")
    out = cfg.get_template("w.ftl").process({"obj": Obj()})
    assert out == "via-attr"
    # attributes_shadow_items=False：getitem 优先
    cfg2 = make_cfg()
    cfg2.set_object_wrapper(fm.PyObjectWrapper(attributes_shadow_items=False))
    cfg2.put_template("w.ftl", "${obj.attr_name}")
    out = cfg2.get_template("w.ftl").process({"obj": Obj()})
    assert out == "via-item:attr_name"


# ---------------------------------------------------------------------------
# set_shared_variable（须在 get_template 之前设置）
# ---------------------------------------------------------------------------

def test_shared_variable():
    cfg = make_cfg()
    cfg.set_shared_variable("brand", "FreeMarker")
    cfg.put_template("s.ftl", "Powered by ${brand}")
    assert cfg.get_template("s.ftl").process({}) == "Powered by FreeMarker"


# ---------------------------------------------------------------------------
# datetime 数据
# ---------------------------------------------------------------------------

def test_datetime_render():
    out = render(
        "${d?datetime}",
        {"d": datetime.datetime(2024, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc)},
    )
    assert out == "Jan 2, 2024 3:04:05 AM"


# ---------------------------------------------------------------------------
# 错误类型（FreeMarkerError 是 RuntimeError 子类）
# ---------------------------------------------------------------------------

def test_freemarket_error_is_runtime_error():
    assert issubclass(fm.FreeMarkerError, RuntimeError)


def test_template_name_getter():
    cfg = make_cfg()
    cfg.put_template("named.ftl", "x")
    assert cfg.get_template("named.ftl").name == "named.ftl"


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))
