# freemarker-pyo3

`freemarker` 模板引擎的 Python 绑定（pyo3）—— Java `freemarker-jython25`
（`freemarker.ext.jython`）的 Rust 迁移。设计见 [`docs/10-pyo3集成设计.md`](../docs/10-pyo3集成设计.md)。

- 模块名：`freemarker`（对应 Java `import freemarker.template.Configuration`）
- 构建：maturin（cdylib + pyo3 0.29）
- Python 版本：>= 3.9（CPython）

## 安装

```bash
# 方式一：从源码构建并安装 wheel
cd freemarker-pyo3
maturin build --release
pip install target/wheels/freemarker_pyo3-*.whl

# 方式二：开发模式（当前环境可直接 import，改动即时生效）
cd freemarker-pyo3
maturin develop --release

# 方式三：发布到 PyPI 后
pip install freemarker-pyo3
```

## 快速开始

```python
import datetime

import freemarker as fm

# 1. 配置（对应 Java new Configuration(VERSION_2_3_34))
cfg = fm.FmConfiguration()

# 2. 注册模板（对应 StringTemplateLoader.put）
cfg.put_template("hello.ftl", "Hello, ${name}! 今天是 ${today?date}")

# 3. 获取模板并渲染（process 返回 str）
template = cfg.get_template("hello.ftl")
out = template.process({"name": "世界", "today": datetime.date(2024, 1, 2)})
print(out)   # Hello, 世界! 今天是 Jan 2, 2024
```

数据模型支持 Python 的 `dict` / `list` / `tuple` / `int` / `float` / `str` /
`bool` / `None` / `datetime` / 可调用对象（函数成为模板方法）：

```python
cfg.put_template("t.ftl", "<#list items as i>${i};</#list> ${greet('FM')}")
cfg.get_template("t.ftl").process(
    {"items": [1, 2, 3], "greet": lambda n: "Hello " + n}
)   # -> "1;2;3; Hello FM"
```

模板内构造的数据传给 Python 函数时会自动还原为 `dict` / `list` 等
（unwrap 方向，见下表）。

## API 参考

### `FmConfiguration`

| 方法 | 对应 Java | 说明 |
|---|---|---|
| `FmConfiguration()` | `new Configuration(VERSION_2_3_34)` | 默认 incompatibleImprovements=2.3.34 |
| `put_template(name, source)` | `StringTemplateLoader.put` | 注册字符串模板 |
| `get_template(name) -> FmTemplate` | `Configuration.getTemplate` | 解析并缓存；不存在抛 `FreeMarkerError`。注意：解析时配置被快照，`set_shared_variable` 必须在 `get_template` 之前 |
| `set_shared_variable(name, obj)` | `Configuration.setSharedVariable` | 注册共享变量（任意 Python 对象，经 wrapper 包装） |
| `set_object_wrapper(wrapper)` | `Configuration.setObjectWrapper` | 应用 `PyObjectWrapper` 配置 |

### `FmTemplate`

| 成员 | 说明 |
|---|---|
| `process(root) -> str` | 渲染：root 为数据模型（dict 等）；错误抛 `FreeMarkerError`（消息含模板名与 `[in template ...]` 定位）。渲染入口持有单次 GIL（docs/10 §4） |
| `name`（属性） | 模板名（对应 `Template.getName()`） |

### `PyObjectWrapper`

对应 Java `JythonWrapper`（docs/10 §1）。

| 成员 | 默认 | 说明 |
|---|---|---|
| `attributes_shadow_items`（属性） | `True` | 通用对象取属性优先（getattr），否则下标优先（getitem）；对应 `setAttributesShadowItems` |
| `use_cache`（属性） | `False` | 包装模型缓存开关；对应 `ModelCache.setUseCache` |

### `TemplateModelAdapter`

unwrap 方向（TModel → Python）的通用适配器（docs/10 §3，对应 Java
`TemplateModelToJythonAdapter`）：模板内构造的 hash/sequence/method 模型
传入 Python 函数时若无法还原为原生类型，则包成该对象，支持
`__getitem__` / `__call__` / `__len__` / `__bool__`；`template_model`
属性返回模型类型描述（调试辅助）。通常不需要直接使用。

### `FreeMarkerError`

模板错误统一桥接为 `freemarker.FreeMarkerError`（`RuntimeError` 子类）：

- 模板语法/求值错误：消息为 Java 风格错误文本，含模板名与行列定位
  （如 `[in template "t.ftl" at line 3, column 5]`）；
- Python 异常（模板调用函数抛出）：作为 cause 嵌入消息
  （如 `ZeroDivisionError: division by zero`）。

## 线程模型（unsendable 约束）

`FmConfiguration` / `FmTemplate` / `PyObjectWrapper` 内部含 `Rc`（非
`Send`），pyclass 均标记 `unsendable`（docs/10 §2）：**对象只能在创建它的
线程中使用**，跨线程访问会被 pyo3 运行时校验拦截并抛 `PanicException`
（`BaseException` 子类，消息含 "unsendable"）——约束失败是响亮的，不会
静默损坏数据。

```python
import threading
from concurrent.futures import ThreadPoolExecutor

cfg = fm.FmConfiguration()
cfg.put_template("t.ftl", "Hello, ${name}!")
tmpl = cfg.get_template("t.ftl")

def worker(i):
    return tmpl.process({"name": i})   # 错误：主线程创建的对象跨线程使用

with ThreadPoolExecutor(max_workers=2) as ex:
    f = ex.submit(worker, 1)
    f.result()  # -> PanicException（unsendable 约束）

# 正确模式：每个线程创建自己的配置/模板（GIL 串行化渲染，无死锁）
def good_worker(i):
    c = fm.FmConfiguration()
    c.put_template("t.ftl", "Hello, ${name}!")
    return c.get_template("t.ftl").process({"name": i})

with ThreadPoolExecutor(max_workers=8) as ex:
    outs = [f.result() for f in (ex.submit(good_worker, i) for i in range(8))]
```

另注意：工作线程抛出的 `FreeMarkerError` 若保留 `__traceback__`，其帧会
引用工作线程创建的 unsendable 对象，主线程释放异常时会触发跨线程 drop
（pyo3 拒绝并输出 unraisable 噪音）——跨线程传异常前请先清空 traceback
（见 `tests/test_threading.py` 的说明）。

## 测试

```bash
cargo test -p freemarker-pyo3        # Rust 单测（33 个；rlib 构建链接 libpython）
python -m pytest tests/              # Python 侧：smoke + 多线程 + 黄金套件
```

- `tests/test_smoke.py` —— API 冒烟测试（docs/10 §8 验收 1：helloworld/
  宏/dict/list/函数/错误捕获）
- `tests/test_threading.py` —— 多线程 GIL 压力测试（docs/10 §8.5：
  8 线程 × 20 渲染逐字节一致、异常传播、unsendable 约束、超时防死锁）
- `tests/test_golden_suite.py` —— Python 黄金套件（jython25 翻译，逐目录
  参数化 `freemarker-test/tests/suite/cases/*/`；34 个用例逐字节 PASS，
  其余按 golden.rs 同款分类 SKIP 并记录原因）

## 与 Java 版的差异（摘要）

- py2 遗留（`PyInstance` / `__tojava__`）与 `JythonRuntime` /
  `ext.ant` 不迁移（docs/10 §6）；
- 版本适配器反射机制删除（pyo3 编译期绑定 CPython ABI）；
- `Decimal`：整数 → `int`，非整数 → `float`；
- GIL 串行化替代 Jython 单线程语义，行为等价（见上文线程模型）。

## License

Apache-2.0（与 freemarker 核心一致）。
