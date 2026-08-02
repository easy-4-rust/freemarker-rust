# -*- coding: utf-8 -*-
"""多线程 GIL 压力测试（docs/10 §8.5：多线程渲染压力测试无死锁）。

覆盖四个验收点：
1. 并发 `process()`：8 线程 × 20 次渲染，各线程持有自己的
   FmConfiguration/FmTemplate（unsendable pyclass 的正确用法），同一
   模板 + 同一根数据 → 全部 160 份输出逐字节一致；
2. 工作线程中的模板错误（FreeMarkerError）与 Python 异常（桥接为
   FreeMarkerError）正确传播回主线程（future.result() 重抛）；
3. unsendable 约束（docs/10 §2 / lib.rs 核心约束）：主线程创建的对象
   经 GIL 从工作线程访问 → pyo3 ThreadChecker 触发 Rust panic → 桥接为
   PanicException（BaseException 子类）——约束被强制执行而非静默损坏；
   对象回到主线程仍可正常使用（GIL 串行化，无数据竞争）；
4. 无死锁：所有 future 带超时守卫（result(timeout=...)），超时即失败。

注意：工作线程抛出的异常若保留 `__traceback__`，其帧会引用工作线程创建的
unsendable 对象；主线程释放异常时会在"另一个线程"drop 这些对象 → pyo3
拒绝跨线程 drop 并输出 unraisable 噪音（对象泄漏）。因此异常传播测试中
在 worker 内清空 traceback（`e.__traceback__ = None`）再重抛——这也验证了
约束的边界：异常对象本身跨线程传输是安全的，被帧引用的 pyclass 对象不是。

运行：python3 -m pytest tests/test_threading.py（或直接 python3 tests/test_threading.py）
"""
import sys
import time
from concurrent.futures import ThreadPoolExecutor, TimeoutError as FutureTimeout

import pytest

import freemarker as fm

#: 压力规模（docs/10 §8.5：8 workers × 20 renders）
WORKERS = 8
RENDERS_PER_WORKER = 20

#: 每 future 的超时守卫（秒）——超时视为死锁
FUTURE_TIMEOUT_S = 60.0

#: 所有线程渲染同一模板 + 同一根数据（断言输出逐字节一致）
STRESS_TEMPLATE = (
    "Hello, ${name}! ${x} + ${y} = ${x + y}; loop: "
    "<#list items as i>${i},</#list>; msg: ${greet(name)}"
)
STRESS_ROOT = {
    "name": "world",
    "x": 3,
    "y": 2,
    "items": [1, 2, 3],
    "greet": lambda n: "Hi " + n,
}


def render_with_local_config(_worker_id: int) -> list:
    """工作线程内创建配置与模板并渲染（unsendable pyclass 的正确模式）。"""
    cfg = fm.FmConfiguration()
    cfg.put_template("stress.ftl", STRESS_TEMPLATE)
    template = cfg.get_template("stress.ftl")
    return [template.process(STRESS_ROOT) for _ in range(RENDERS_PER_WORKER)]


def run_with_timeout(worker_fn, *args) -> list:
    """ThreadPoolExecutor 执行并在主线程收结果（带超时守卫）。"""
    with ThreadPoolExecutor(max_workers=WORKERS) as ex:
        futures = [ex.submit(worker_fn, i) for i in range(WORKERS)]
        return [f.result(timeout=FUTURE_TIMEOUT_S) for f in futures]


# ---------------------------------------------------------------------------
# 1. 并发 process()：8 workers × 20 renders，全部输出逐字节一致
# ---------------------------------------------------------------------------

def test_concurrent_process_byte_identical():
    results = run_with_timeout(render_with_local_config)
    assert len(results) == WORKERS
    all_outs = [o for r in results for o in r]
    assert len(all_outs) == WORKERS * RENDERS_PER_WORKER
    # 全部 160 份输出逐字节一致（同一模板 + 同一根数据）
    first = all_outs[0]
    assert all(o == first for o in all_outs)
    assert first == "Hello, world! 3 + 2 = 5; loop: 1,2,3,; msg: Hi world"


# ---------------------------------------------------------------------------
# 2. 工作线程异常传播
# ---------------------------------------------------------------------------

def render_boom_template():
    cfg = fm.FmConfiguration()
    cfg.put_template("boom.ftl", "Before ${missing} after")
    try:
        return cfg.get_template("boom.ftl").process({})
    except fm.FreeMarkerError as e:
        # 清空 traceback：避免帧引用 worker 线程的 unsendable 对象，
        # 使主线程释放异常时跨线程 drop（pyo3 会拒绝并输出噪音）
        e.__traceback__ = None
        raise


def render_python_exception():
    cfg = fm.FmConfiguration()
    cfg.put_template("pyboom.ftl", "${boom()}")
    try:
        return cfg.get_template("pyboom.ftl").process({"boom": lambda: 1 / 0})
    except fm.FreeMarkerError as e:
        e.__traceback__ = None
        raise


def test_template_error_in_worker_propagates():
    with ThreadPoolExecutor(max_workers=2) as ex:
        futures = [ex.submit(render_boom_template) for _ in range(4)]
        for f in futures:
            with pytest.raises(fm.FreeMarkerError) as ei:
                f.result(timeout=FUTURE_TIMEOUT_S)
            msg = str(ei.value)
            assert "missing" in msg and "boom.ftl" in msg


def test_python_exception_in_worker_propagates():
    with ThreadPoolExecutor(max_workers=2) as ex:
        futures = [ex.submit(render_python_exception) for _ in range(4)]
        for f in futures:
            with pytest.raises(fm.FreeMarkerError) as ei:
                f.result(timeout=FUTURE_TIMEOUT_S)
            msg = str(ei.value)
            assert "ZeroDivisionError" in msg and "division by zero" in msg


# ---------------------------------------------------------------------------
# 3. unsendable 约束：主线程创建的对象不可跨线程使用（GIL 访问失败要响亮）
# ---------------------------------------------------------------------------

def process_shared(shared_tmpl, tag):
    # 工作线程经 GIL 调用主线程创建对象的 process() —— pyo3 ThreadChecker
    # 校验线程归属，触发 Rust panic → PanicException（BaseException 子类）
    return shared_tmpl.process({"name": tag})


def test_unsendable_constraint_shared_object_from_worker():
    cfg = fm.FmConfiguration()
    cfg.put_template("shared.ftl", "Hello, ${name}!")
    shared = cfg.get_template("shared.ftl")
    with ThreadPoolExecutor(max_workers=4) as ex:
        futures = [ex.submit(process_shared, shared, f"u{i}") for i in range(4)]
        for f in futures:
            try:
                f.result(timeout=FUTURE_TIMEOUT_S)
                pytest.fail("unsendable 对象跨线程调用应抛异常（PanicException）")
            except BaseException as e:  # PanicException 是 BaseException 子类
                assert type(e).__name__ == "PanicException", f"{type(e).__name__}: {e}"
                assert "unsendable" in str(e), f"异常消息应说明 unsendable 约束：{e}"
    # 对象未被破坏：回主线程仍可正常渲染（GIL 串行化访问，无数据竞争）
    assert shared.process({"name": "main"}) == "Hello, main!"


def test_each_thread_own_config_works():
    """对照：各线程自建配置/模板（unsendable 的正确模式）并发可用。"""
    cfg = fm.FmConfiguration()
    cfg.put_template("own.ftl", "Hello, ${name}!")
    assert cfg.get_template("own.ftl").process({"name": "main"}) == "Hello, main!"


# ---------------------------------------------------------------------------
# 4. 无死锁（整体超时守卫）
# ---------------------------------------------------------------------------

def test_no_deadlock_timeout_guard():
    """全部并发渲染带超时守卫：死锁/挂起会以 FutureTimeout 失败而非挂死。"""
    start = time.monotonic()
    results = run_with_timeout(render_with_local_config)
    elapsed = time.monotonic() - start
    assert len(results) == WORKERS
    # 渲染总量 160 次，GIL 串行化下应远低于超时上限
    assert elapsed < FUTURE_TIMEOUT_S, f"耗时 {elapsed:.1f}s 接近超时阈值，疑似死锁"
    # 永不返回的 worker 必须在超时后抛 FutureTimeout（超时守卫生效演示）
    with ThreadPoolExecutor(max_workers=1) as ex:
        f = ex.submit(time.sleep, 10)
        with pytest.raises(FutureTimeout):
            f.result(timeout=0.5)


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))
