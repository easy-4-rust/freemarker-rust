# NOT_APPLICABLE 登记清单（Java 测试逻辑中无法 Rust 1:1 实现的部分）

> 依据 rust-java-migration-testing 技能：依赖 JVM 特有机制（反射/DOM/XPath/线程/
> 编码覆盖/jython）的 Java 测试无法在 Rust 引擎 1:1 实现，逐类登记原因。
> 对应 java-tests/ 资源镜像 + tests/java_ported/ 的 Rust 1:1 实现。

## freemarker-core/src/test（192 类，740 @Test）

### ext/beans（44 文件，20 测试类，78 @Test）—— 全部 NOT_APPLICABLE
JVM `java.lang.reflect`/`Introspector` 内省、方法重载解析（bridge method/默认方法/
继承优先级）、成员访问策略（MemberSelectorList/DefaultObjectWrapperMemberAccessPolicy）、
枚举/静态模型包装、BeanModel 语义。Rust 无 JVM 反射等价物。
代表类：BeansWrapperBasics、BeansWrapperMiscTest、BeansWrapperReadOnlyTest、
DefaultObjectWrapperMemberAccessPolicyTest、Java8BeansWrapperTest（默认方法/bridge 方法）、
EnumModelsTest、StaticModelsTest、MethodMatcherTest、IsApplicableTest、
IsMoreSpecificParameterTypeTest、ParameterListPreferabilityTest、ModelCacheTest、
TypeFlagsTest、OverloadedNumberUtilTest、ErrorMessagesTest、FineTuneMethodAppearanceTest、
GetPropertyNameFromReaderMethodNameTest、MiscNumericalOperationsTest、
BeansAPINewInstanceTest、BeansWrapperSingletonsTest、BeansWrapperCachesTest、
CommonSupertypeForUnwrappingHintTest、DefaultMemberAccessPolicyTest、
LegacyDefaultMemberAccessPolicyTest、Java9InstrospectorBugWorkaroundTest、
MemberAccessMonitoringTest、MethodUtilTest(2)、AbstractParallelIntrospectionTest、
PrallelObjectIntrospectionTest、PrallelStaticIntrospectionTest、
Java8BridgeMethodsWithDefaultMethodBean*、BridgeMethodsBean*、ManyObjectsOfDifferentClasses、
ManyStaticsOfDifferentClasses、GetlessMethodsAsPropertyGettersRule

### ext/dom（3 测试类，23 @Test）—— 全部 NOT_APPLICABLE
org.w3c.dom/JAXP/DOM 包装 + Jaxen XPath（NodeModel 家族）。Rust 无 DOM/XPath 等价物
（若引入 XML 库可重新评估）。
代表类：DOMTest、DOMSiblingTest（含资源 DOMSiblingTest.xml）、DOMConvenienceStaticsTest

### 反射/平台依赖（约 20 @Test）—— NOT_APPLICABLE
- core/OptInTemplateClassResolverTest（Java 类加载安全策略 TemplateClassResolver）
- template/StaticObjectWrappersTest（Java 静态类包装）
- core/LegacyFMParserConstructorsTest（Java FMParser 直接构造 + createExpressionParser）
- core/ThreadInterruptingSupportTest（Java 线程中断机制）
- core/TemplateProcessingTracerTest（Java tracer API）
- core/EncodingOverrideTest（Java 模板编码覆盖机制；v1 见 get_template_encoded）
- core/EnvironmentCustomStateTest（Environment.setCustomState/getCustomState Java API）
- core/CombinedMarkupOutputFormatTest（CombinedMarkupOutputFormat 类 Java API 级测试）
- core/ConfigurableTest（java.lang.reflect.Field 遍历断言 _KEY/_KEY_SNAKE_CASE/_KEY_CAMEL_CASE
  字段一致性；JVM 反射特有。其中 testGetSettingNamesAreSorted 已翻译为
  settings_names_sorted 断言，见 configurable_test.rs）

### 引擎缺口类 —— NOT_APPLICABLE（v1 引擎无对应机制）
- core/GetOptionalTemplateTest（`.getOptionalTemplate` 特殊变量/API；v1 无模板对象 API）
- core/CallerTemplateNameTest（`.callerTemplateName` 特殊变量；v1 无调用者模板追踪）
- core/IncludeAndImportConfigurableLayersTest（TemplateConfiguration +
  ConditionalTemplateConfigurationFactory + FileNameGlobMatcher per-template 配置层；
  v1 引擎无 per-template 配置）
- core/TemplateLevelSettings（Template.setBooleanFormat 等 per-template 设置；
  v1 无模板级设置，仅有 Configuration 级 settings）
- core/TemplateConfigurationTest / TemplateConfigurationWithTemplateCacheTest 的
  TemplateConfiguration（per-template 配置）相关方法（其余模板层方法已翻译，
  见 template_configuration_test.rs / template_configuration_with_template_cache_test.rs）
- core/DirectiveCallPlaceTest（DirectiveCallPlace 调用点身份内省 + 自定义数据缓存
  isNestedOutputCacheable；依赖 Java 指令对象身份语义，无等价物）

### AST/快照类 —— NOT_APPLICABLE（无引擎等价物）
- core/ASTTest（ASTPrinter 快照 .ast）
- core/CanonicalFormTest（canonical form 快照 .ftl.out）

### 引擎设置缺口（部分断言）—— 见测试文件内注释
- TemplateNameFormat.DEFAULT_2_4_0（v1 固定 2.3.34 默认 DEFAULT_2_3_0）
- setObjectWrapper（BeansWrapper/DefaultObjectWrapper 配置）
- ?api 内建（BeanWrapper 特有）

## freemarker-jython25/src/test（13 文件）

### jython 特有（2 @Test）—— NOT_APPLICABLE
- ObjectBuilderSettingsTest.jythonWrapperTest（JythonWrapper 单例解析）
- DefaultObjectWrapperTest.testDisabledJythonWrapping（PyString → JythonSequenceModel）

## 备注
- 其余可移植类在 tests/java_ported/ 以 Rust 1:1 实现（测试方法同名同逻辑，
  错误消息逐字对齐；引擎差异断言在测试文件内以 `// 引擎差异：` 注释标注）。
- 更新上游测试资产时保持 java-tests/ 资源镜像 diff 清洁（见 java-tests/README.md）。
