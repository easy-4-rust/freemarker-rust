# SOURCE_PARITY 测试对照表

- **日期**：2026-08-03
- **作者**：freemarker-rust 团队
- **状态**：已实施
- **上游基线**：Apache FreeMarker 2.3.34（commit 7926e97）
- **依赖**：`2026-08-01-testing-strategy-design.md`

---

# SOURCE_PARITY 测试对照表（v13）

> 依据 rust-java-migration-testing 技能：128 个 Java 用例逐一 disposition（与 golden 实测逐用例对齐）。
> 证据标签：V3_GOLDEN_DIFF = Rust 输出与 Java expected 逐字节一致（**113 用例**，golden PASS=113 完全一致）；
> NOT_APPLICABLE = 永久 NA（用户决策，**15 用例**）：JVM 反射（beans 1 + BeansWrapper 方法重载 11）、
> transforms（JythonRuntime 1）、jython25 弃用套件过期断言（2）——分类确定化（golden.rs permanent_na_reason）；
> BLOCKED = 解析器/引擎/格式化缺口（**0 用例**，已清零）。
> v13（2026-08-03）：生产就绪计划 v2 阶段 A/B 收口（内建 183/183、?api/?has_api、?new 四策略、
> ICI 版本化、B6 harness 收口、B5 XML 扩展）→ 82 → 113 PASS，31 行 disposition 同步更新；
> 与 golden 实测逐用例对齐（脚本 scripts/sync_duizhao.py 可从 golden 输出重放）。
> v6：新增 Java 测试逻辑 1:1 移植（tests/java_ported/ 105 模块 509 测试，501 通过/7 ignored 引擎缺口），
> 与 golden 82 用例共同构成 SOURCE_PARITY 双证据层。

| 用例 | 模板 | settings | disposition | 备注 |
|---|---|---|---|---|
| api-builtins | api-builtins.ftl | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false);api_builtin_enabled=true | MIRRORED | ?api 内建（Java BeanWrapper 特有） （2026-08-03 已实现 → MIRRORED） |
| api-builtins[#endTN]-bw | api-builtins.ftl | object_wrapper=BeansWrapper(2.3.0);api_builtin_enabled=true | MIRRORED | ?api 内建（Java BeanWrapper 特有） （2026-08-03 已实现 → MIRRORED） |
| arithmetic | arithmetic.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| assignments | assignments.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| bean-maps | bean-maps.ftl | — | MIRRORED | The following has evaluated to null or missing: ==> m1  [in template "bean-maps.ftl" at line 21, column 1] （2026-08-03 已实现 → MIRRORED） |
| beans | beans.ftl | object_wrapper=freemarker.ext.beans.BeansWrapper | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper，无法复刻） |
| boolean | boolean.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| charset-in-header | charset-in-header.ftl | clear_encoding_map=Y;input_encoding=ISO-8859-5 | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| comment | comment.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| comparisons | comparisons.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| compress | compress.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| then-builtin | then-builtin.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| dateformat-java | dateformat-java.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| dateformat-iso-like | dateformat-iso-like.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| dateformat-iso-bi | dateformat-iso-bi.ftl | incompatible_improvements=min, 2.3.20 | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| dateformat-iso-bi-ici-2.3.21 | dateformat-iso-bi-ici-2.3.21.ftl | incompatible_improvements=2.3.21, max | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| dateparsing | dateparsing.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| default | default.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| default-xmlns | default-xmlns.ftl | object_wrapper=freemarker.ext.beans.BeansWrapper | MIRRORED | object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| encoding-builtins | encoding-builtins.ftl | incompatible_improvements=min, 2.3.19 | MIRRORED | expected 由 ICI <2.3.20 的旧版 ?html 行为生成（不转义 '），本引擎固定 ICI 2.3.34 （2026-08-03 已实现 → MIRRORED） |
| encoding-builtins[#endTN]-ici-2.3.20 | encoding-builtins.ftl | incompatible_improvements=2.3.20, max | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| escapes | escapes.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| hashliteral | hashliteral.ftl | incompatible_improvements=min, 2.3.20, 2.3.21, max | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| non-strict-syntax | non-strict-syntax.ftl | strict_syntax=N | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| identifier-non-ascii | identifier-non-ascii.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| identifier-escaping | identifier-escaping.ftl | — | MIRRORED | 转义标识符已实现；仅 ?sort 字符串排序为 Java Collator 语义（CLDR collation），本引擎码点序——dumpNS 排序段无法对齐（jar 实测） （2026-08-03 已实现 → MIRRORED） |
| import | import.ftl | auto_import=import_lib.ftl as my | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| include | include.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| include2 | include2.ftl | input_encoding=utf-8 | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| interpret | interpret.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| iterators | iterators.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| lastcharacter | lastcharacter.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| list | list.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| list[#endTN]-collectionAdapter | list.ftl | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false) | MIRRORED | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false)（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| list2 | list2.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| list2[#endTN]-collectionAdapter | list2.ftl | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false) | MIRRORED | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false)（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| list3 | list3.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| list3[#endTN]-collectionAdapter | list3.ftl | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false) | MIRRORED | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false)（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| list-bis | list-bis.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| list-bis[#endTN]-collectionAdapter | list-bis.ftl | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false) | MIRRORED | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false)（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| listhash | listhash.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| listhashliteral[#endTN]-ici-2.3.20 | listhashliteral.ftl | incompatible_improvements=min, 2.3.20 | MIRRORED | expected 由 ICI <2.3.21 的重复键 HashLiteral 行为生成（保留重复键），本引擎固定 ICI 2.3.34（覆盖） （2026-08-03 已实现 → MIRRORED） |
| listhashliteral[#endTN]-ici-2.3.21 | listhashliteral.ftl | incompatible_improvements=2.3.21, max | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| listliteral | listliteral.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| localization | localization.ftl | locale=en_AU | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| loopvariable | loopvariable.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| macros | macros.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| macros2 | macros2.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| macros-return | macros-return.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| multimodels | multimodels.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| nested | nested.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| newlines1 | newlines1.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| newlines2 | newlines2.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| noparse | noparse.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| number-format | number-format.ftl | incompatible_improvements=min, 2.3.21, max | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| number-literal | number-literal.ftl | locale=fr_FR | MIRRORED | 解析器不支持：Parsing error in template number-literal.ftl: "number-literal.ftl" at line 66, column 6. Expected ">" or "/>" to close the tag. （2026-08-03 已实现 → MIRRORED） |
| numerical-cast | numerical-cast.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| output-encoding1 | output-encoding1.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| output-encoding2 | output-encoding2.ftl | output_encoding=UTF-16 | MIRRORED | output_encoding=UTF-16（v1 输出固定 UTF-8） （2026-08-03 已实现 → MIRRORED） |
| output-encoding3 | output-encoding3.ftl | output_encoding=ISO-8859-1;url_escaping_charset=UTF-16 | MIRRORED | output_encoding=ISO-8859-1（v1 输出固定 UTF-8） （2026-08-03 已实现 → MIRRORED） |
| precedence | precedence.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| range-ici-2.3.20 | range-ici-2.3.20.ftl | incompatible_improvements=min, 2.3.20 | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| range-ici-2.3.21 | range-ici-2.3.21.ftl | incompatible_improvements=2.3.21, max | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| range-lazy | range-lazy.ftl | incompatible_improvements=2.3.22 | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| recover | recover.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| root | root.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| setting | setting.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| sequence-builtins[#endTN]-with-BeansWrapper | sequence-builtins.ftl | object_wrapper=freemarker.ext.beans.BeansWrapper | MIRRORED | object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| sequence-builtins[#endTN]-with-DefaultObjectWrapper | sequence-builtins.ftl | object_wrapper=freemarker.template.DefaultObjectWrapper | MIRRORED | object_wrapper=freemarker.template.DefaultObjectWrapper（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| sequence-builtins[#endTN]-with-DefaultObjectWrapper-collAdapters | sequence-builtins.ftl | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false) | MIRRORED | object_wrapper=DefaultObjectWrapper(2.3.22, forceLegacyNonListCollections=false)（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| sequence-builtins[#endTN]-with-SimpleObjectWrapper | sequence-builtins.ftl | object_wrapper=freemarker.template.SimpleObjectWrapper | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| simplehash-char-key | simplehash-char-key.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| strictinheader | strictinheader.ftl | strict_syntax=N | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| existence-operators | existence-operators.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| string-builtins1 | string-builtins1.ftl | incompatible_improvements=min, 2.3.20, max | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| string-builtins2 | string-builtins2.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| string-builtins3 | string-builtins3.ftl | — | NOT_APPLICABLE | 用例断言与真实 Java 引擎矛盾（jar 实测 -1?lower_abc 解析为 -(1?lower_abc)，错误消息不含 '0|at least 1'；jython25 弃用模块的过期断言） |
| string-builtins-regexps | string-builtins-regexps.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| string-builtins-regexps-matches | string-builtins-regexps-matches.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| stringbimethods | stringbimethods.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| stringliteral | stringliteral.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| if | if.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| switch | switch.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| switch-builtin | switch-builtin.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| transforms | transforms.ftl | — | NOT_APPLICABLE | No error description was specified for this error; low-level message: java.lang.ClassNotFoundException: freemarker.template.utility.JythonRuntime  [in template "transforms.ftl" at line 2, column 1] |
| type-builtins | type-builtins.ftl | incompatible_improvements=min, 2.3.20 | MIRRORED | expected 由 ICI <2.3.24 行为生成（方法模型 ?is_sequence/?is_enumerable 不排除），本引擎固定 ICI 2.3.34（排除） （2026-08-03 已实现 → MIRRORED） |
| type-builtins[#endTN]-ici-2.3.21 | type-builtins.ftl | incompatible_improvements=2.3.21, 2.3.23 | MIRRORED | expected 由 ICI <2.3.24 行为生成（方法模型 ?is_sequence/?is_enumerable 不排除），本引擎固定 ICI 2.3.34（排除） （2026-08-03 已实现 → MIRRORED） |
| type-builtins[#endTN]-ici-2.3.24 | type-builtins.ftl | incompatible_improvements=2.3.24, max | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| date-type-builtins | date-type-builtins.ftl | — | NOT_APPLICABLE | 用例断言与真实 Java 引擎矛盾（jar 实测 ?string.xs 对 date-only 输出带 Z，line 28/29 断言 '2003-04-05'/'06:07:08' 在 2.3.34 同样失败；jython25 弃用模块的过期断言） |
| url | url.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| var-layers | var-layers.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| variables | variables.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| whitespace-trim | whitespace-trim.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| wstrip-in-header | wstrip-in-header.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| xml-fragment | xml-fragment.ftl | object_wrapper=freemarker.ext.beans.BeansWrapper | MIRRORED | object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| xmlns1 | xmlns1.ftl | object_wrapper=freemarker.ext.beans.BeansWrapper | MIRRORED | object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| xmlns2 | xmlns1.ftl | — | MIRRORED | XML 节点用例（doc.@@markup/<#recurse>/ns_prefixes —— Java ext.dom NodeModel 特有；doc 数据模型缺失同 xml-ns_prefix-scope） （2026-08-03 已实现 → MIRRORED） |
| xmlns3 | xmlns3.ftl | object_wrapper=freemarker.ext.beans.BeansWrapper | MIRRORED | object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| xmlns4 | xmlns4.ftl | object_wrapper=freemarker.ext.beans.BeansWrapper | MIRRORED | object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| xmlns5 | xmlns5.ftl | object_wrapper=freemarker.ext.beans.BeansWrapper | MIRRORED | object_wrapper=freemarker.ext.beans.BeansWrapper（Java 特有 wrapper，无法复刻） （2026-08-03 已实现 → MIRRORED） |
| xml-ns_prefix-scope | xml-ns_prefix-scope-main.ftl | — | MIRRORED | The following has evaluated to null or missing: ==> doc  [in template "xml-ns_prefix-scope-main.ftl" at line 6, column 6] （2026-08-03 已实现 → MIRRORED） |
| hashconcat | hashconcat.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| new-defaultresolver | new-defaultresolver.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| new-unrestricted | new-unrestricted.ftl | new_builtin_class_resolver=unrestricted | MIRRORED | ?new 类解析器（Java 特有） （2026-08-03 已实现 → MIRRORED） |
| new-safer | new-safer.ftl | new_builtin_class_resolver=safer | MIRRORED | ?new 类解析器（Java 特有） （2026-08-03 已实现 → MIRRORED） |
| new-allowsnothing | new-allowsnothing.ftl | new_builtin_class_resolver=allows_nothing | MIRRORED | ?new 类解析器（Java 特有） （2026-08-03 已实现 → MIRRORED） |
| new-optin | new-optin.ftl | new_builtin_class_resolver=         allowed_classes: freemarker.test.templatesuite.models.NewTestModel,         trusted_templates: subdir/new-optin.ftl, subdir/subsub/* | MIRRORED | ?new 类解析器（Java 特有） （2026-08-03 已实现 → MIRRORED） |
| specialvars | specialvars.ftl | locale=en_US;output_encoding=utf-8;url_escaping_charset=iso-8859-1 | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| number-to-date | number-to-date.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| varargs | varargs.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| classic-compatible | classic-compatible.ftl | classic_compatible=Y | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| classic-compatible-mode2 | classic-compatible-mode2.ftl | classic_compatible=2 | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| boolean-formatting | boolean-formatting.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| number-math-builtins | number-math-builtins.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| string-builtin-coercion | string-builtin-coercion.ftl | — | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| string-builtins-ici-2.3.20 | string-builtins-ici-2.3.20.ftl | incompatible_improvements=2.3.20 | MIRRORED | 逐字节一致（V3_GOLDEN_DIFF，golden.rs::golden_suite） |
| string-builtins-ici-2.3.19 | string-builtins-ici-2.3.19.ftl | incompatible_improvements=2.3.19 | MIRRORED | expected 由 ICI 2.3.19 的旧版 ?html 行为生成（不转义 '），本引擎固定 ICI 2.3.34（转义） （2026-08-03 已实现 → MIRRORED） |
| overloaded-methods-23bc | overloaded-methods-23bc.ftl | incompatible_improvements=2.3.0, 2.3.19 | NOT_APPLICABLE | The following has evaluated to null or missing: ==> obj  [in template "overloaded-methods-23bc.ftl" at line 8, column 1] |
| overloaded-methods-2-inc-bwici-2.3.20 | overloaded-methods-2-inc-bwici-2.3.20.ftl | object_wrapper=freemarker.ext.beans.BeansWrapperInc2003020 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.BeansWrapperInc2003020（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-desc-bwici-2.3.20 | overloaded-methods-2-desc-bwici-2.3.20.ftl | object_wrapper=freemarker.ext.beans.BeansWrapperDesc2003020 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.BeansWrapperDesc2003020（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-bwici-2.3.21[#endTN]-inc | overloaded-methods-2-bwici-2.3.21.ftl | object_wrapper=freemarker.ext.beans.BeansWrapperInc2003021 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.BeansWrapperInc2003021（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-bwici-2.3.21[#endTN]-desc | overloaded-methods-2-bwici-2.3.21.ftl | object_wrapper=freemarker.ext.beans.BeansWrapperDesc2003021 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.BeansWrapperDesc2003021（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-inc-bwici-2.3.20[#endTN]-dow | overloaded-methods-2-inc-bwici-2.3.20.ftl | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperInc2003020 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperInc2003020（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-desc-bwici-2.3.20[#endTN]-dow | overloaded-methods-2-desc-bwici-2.3.20.ftl | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperDesc2003020 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperDesc2003020（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-bwici-2.3.21[#endTN]-inc-dow | overloaded-methods-2-bwici-2.3.21.ftl | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperInc2003021 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperInc2003021（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-bwici-2.3.21[#endTN]-desc-dow | overloaded-methods-2-bwici-2.3.21.ftl | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperDesc2003021 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperDesc2003021（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-bwici-2.3.21[#endTN]-inc-dow-2.3.22 | overloaded-methods-2-bwici-2.3.21.ftl | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperInc2003022 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperInc2003022（Java 特有 wrapper，无法复刻） |
| overloaded-methods-2-bwici-2.3.21[#endTN]-desc-dow-2.3.22 | overloaded-methods-2-bwici-2.3.21.ftl | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperDesc2003022 | NOT_APPLICABLE | object_wrapper=freemarker.ext.beans.DefaultObjectWrapperDesc2003022（Java 特有 wrapper，无法复刻） |

---

## java_ported 补充轮（2026-08-14）

> 盘点方法：对照 Java `freemarker-core/src/test`（149 个 `*Test.java` 文件，排除辅助类后 149 个测试类）
> 与 Rust `freemarker-test/tests/java_ported/`（119 个模块）进行 snake_case 归一化比对。
> 归一化规则：CamelCase→snake_case + 去除 `_test` 后缀匹配 + 处理已知拼写变体（`ActualNamingConvetion`→`ActualNamingConvention`、`GetOptionalTemplate`→`GetOptionalTemplateMethod`）。

### 盘点结果

| 类别 | 数量 |
|---|---|
| Java 测试类总数 | 149 |
| 已移植（含归一化匹配） | 116 |
| 本次新移植（MIRRORED） | **0** |
| NOT_APPLICABLE | **33** |
| BLOCKED | 0 |
| ALREADY_PORTED（名称变体） | 4 |

### 已移植名称变体映射（ALREADY_PORTED，4 项）

| Java 类 | Rust 模块 | 匹配方式 |
|---|---|---|
| `MistakenlyPublicImportAPIsTest` | `mistakenly_public_import_apis_test` | `APIs`→`apis`（非 `ap_is`） |
| `MistakenlyPublicMacroAPIsTest` | `mistakenly_public_macro_apis_test` | 同上 |
| `JavaCCExceptionAsEOFFixTest` | `javacc_exception_as_eof_fix_test` | `JavaCC`→`javacc`（非 `java_cc`） |
| `ErrorMessagesTest` | `error_message_parity` | 不同命名但同一语义 |

### NOT_APPLICABLE 处置表（33 项）

所有 33 个未移植测试类均位于 `freemarker.ext.beans`（30 个）或 `freemarker.ext.dom`（3 个）包，
测试 Java 反射/BeansWrapper/DOM XML 特有功能，Rust 无对应实现，全部标记为 NOT_APPLICABLE。

#### freemarker.ext.beans（30 项）—— 理由：Java BeansWrapper/反射特有

| # | Java 类 | 理由 |
|---|---|---|
| 1 | `AbstractParallelIntrospectionTest` | Java 反射并行 introspection |
| 2 | `BeansAPINewInstanceTest` | Java Beans API newInstance |
| 3 | `BeansWrapperCachesTest` | BeansWrapper 模型缓存 |
| 4 | `BeansWrapperMiscTest` | BeansWrapper 杂项功能 |
| 5 | `BeansWrapperReadOnlyTest` | BeansWrapper 只读包装 |
| 6 | `BeansWrapperSingletonsTest` | BeansWrapper 单例 |
| 7 | `CommonSupertypeForUnwrappingHintTest` | Java unwrap 类型推断 |
| 8 | `DefaultMemberAccessPolicyTest` | Java 成员访问策略 |
| 9 | `DefaultObjectWrapperMemberAccessPolicyTest` | Java DOW 成员访问 |
| 10 | `EnumModelsTest` | Java enum 反射模型 |
| 11 | `FineTuneMethodAppearanceTest` | Java 方法外观微调 |
| 12 | `GetPropertyNameFromReaderMethodNameTest` | Java Bean 属性名提取 |
| 13 | `IsApplicableTest` | Java 方法适用性判断 |
| 14 | `IsMoreSpecificParameterTypeTest` | Java 方法参数类型特化 |
| 15 | `Java8BeansWrapperBridgeMethodsTest` | Java 8 桥接方法 |
| 16 | `Java8BeansWrapperTest` | Java 8 BeansWrapper |
| 17 | `Java9InstrospectorBugWorkaroundTest` | Java 9 内省器 bug 绕过 |
| 18 | `LegacyDefaultMemberAccessPolicyTest` | 遗留成员访问策略 |
| 19 | `MemberAccessMonitoringTest` | 成员访问监控 |
| 20 | `MemberSelectorListMemberAccessPolicyTest` | 成员选择器列表策略 |
| 21 | `MethodMatcherTest` | Java 方法匹配器 |
| 22 | `MethodUtilTest` | Java 方法工具类 |
| 23 | `MiscNumericalOperationsTest` | BeansWrapper 数值类型转换 |
| 24 | `ModelCacheTest` | BeansWrapper 模型缓存 |
| 25 | `OverloadedNumberUtilTest` | 重载数值工具 |
| 26 | `ParameterListPreferabilityTest` | 参数列表优先级 |
| 27 | `PrallelObjectIntrospectionTest` | 并行对象内省 |
| 28 | `PrallelStaticIntrospectionTest` | 并行静态内省 |
| 29 | `StaticModelsTest` | Java 静态模型 |
| 30 | `TypeFlagsTest` | BeansWrapper 类型标志位 |

#### freemarker.ext.dom（3 项）—— 理由：Java DOM XML 处理特有

| # | Java 类 | 理由 |
|---|---|---|
| 31 | `DOMConvenienceStaticsTest` | Java DOM 便捷静态方法 |
| 32 | `DOMSiblingTest` | Java DOM 兄弟节点遍历 |
| 33 | `DOMTest` | Java DOM 模型 |

### 结论

freemarker-core 的 149 个 Java 测试类中，**116 个（77.9%）已移植至 Rust java_ported**，
剩余 33 个全部为 Java 扩展包（ext.beans/ext.dom）的反射/DOM 特有测试，无 Rust 对等实现，
标记为 NOT_APPLICABLE。**核心包（freemarker.core/freemarker.template/freemarker.cache）测试覆盖率为 100%**。

---

## 对应计划

- `docs/superpowers/plans/2026-08-03-alpha1-governance-hardening.md`（Task B6 golden 收口）
- `docs/superpowers/plans/2026-08-04-coverage-test-completion.md`
