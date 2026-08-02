# Java 测试资源镜像（测试所需文件）

本目录是 Apache FreeMarker（commit 7926e97，2.3.34 线）**两套 Java 测试树的测试所需文件镜像**
（.ftl 模板 / .txt 期望输出 / .xml / .properties），按 rust-java-migration 技能要求归档：

| 镜像目录 | 源目录（freemarker 仓库） | 内容 |
|---|---|---|
| `jython25/src/test/resources` | `freemarker-jython25/src/test/resources` | 模板套件资源：testcases.xml（128 用例清单）、expected/（94 个 .txt）、templates/（138 个 .ftl 含 subdir/subsub）、models/（7 个 XML + BeansTestResources.properties）共 241 文件 |
| `core/src/test/resources` | `freemarker-core/src/test/resources` | 核心测试资源：freemarker/core（ast-/cano-/encodingOverride 快照）、cache、ext/dom、manual（示例模板+期望）、template 共 75 文件 |

## 测试逻辑的 Rust 1:1 实现位置

- **模板套件 128 用例**：`../tests/suite/`（工作副本）+ `../tests/golden.rs`（驱动：settings 应用、数据模型复刻、expected 逐字节比对）
- **core/jython25 单元测试**：`../tests/java_ported/`（每个 Java 测试类对应一个 Rust 测试文件，测试方法 1:1 翻译，错误消息逐字对齐 Java）
- 依赖 JVM 反射/DOM/XPath/jython 的测试（ext/beans、ext/dom 等约 110 个 @Test）登记 NOT_APPLICABLE（见验收报告 §6）

## 复制与校验方式

```bash
cp -r <freemarker>/freemarker-jython25/src/test/resources freemarker-test/java-tests/jython25/src/test/resources
cp -r <freemarker>/freemarker-core/src/test/resources   freemarker-test/java-tests/core/src/test/resources
diff -r <freemarker>/freemarker-jython25/src/test/resources freemarker-test/java-tests/jython25/src/test/resources
diff -r <freemarker>/freemarker-core/src/test/resources   freemarker-test/java-tests/core/src/test/resources
```

- 测试文件直接复制，不做任何改写；校验日期：2026-08-02（diff -r 逐字节一致，共 316 文件）。
- 更新上游测试资产时：重新执行复制命令并保持 diff 清洁。
