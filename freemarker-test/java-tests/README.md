# Java 测试资产镜像（1:1）

本目录是 Apache FreeMarker（commit 7926e97，2.3.34 线）**两套 Java 测试树的逐字节镜像**，
按 rust-java-migration 技能要求归档，保证 Rust 侧测试与 Java 侧测试资产一一对应：

| 镜像目录 | 源目录（freemarker 仓库） | 内容 |
|---|---|---|
| `jython25/src/test` | `freemarker-jython25/src/test` | 模板套件（TemplateTestSuite/TemplateTestCase + testcases.xml + expected/templates/models 共 241 资源 + 13 Java） |
| `core/src/test` | `freemarker-core/src/test` | 核心单元测试（192 Java + 75 资源：freemarker/core、cache、ext、template、manual 各包） |

## 复制与校验方式

```bash
# 复制（保留原路径结构）
cp -r <freemarker>/freemarker-jython25/src/test freemarker-test/java-tests/jython25/src/test
cp -r <freemarker>/freemarker-core/src/test   freemarker-test/java-tests/core/src/test

# 逐字节校验（diff -r 无输出 = 一致）
diff -r <freemarker>/freemarker-jython25/src/test freemarker-test/java-tests/jython25/src/test
diff -r <freemarker>/freemarker-core/src/test   freemarker-test/java-tests/core/src/test
```

- **测试文件**（模板 `.ftl`、期望输出 `.txt`、`.xml` 等资源）：直接复制，不做任何改写。
- **测试用例源码**（`.java`）：1:1 逐字复制，不做转译/改写。
- 校验日期：2026-08-02（diff -r 逐字节一致，共 521 文件）。

## 与 golden 套件的关系

- `../tests/suite/`（cases/、templates/、manifest.json）是 golden.rs 实际运行的
  模板套件工作副本（从 `jython25/src/test/resources/.../testcases.xml` 提取，
  ftl+expected 逐字节一致）。
- 本目录是完整 Java 测试树档案：任何用例的 Java 测试类与资源的权威对照。
- 更新上游测试资产时：重新执行上述复制命令并保持 diff 清洁。
