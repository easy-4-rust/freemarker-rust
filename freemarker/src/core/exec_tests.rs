//! 测试 —— 自 exec.rs 拆出（#[cfg(test)] 模块；由主文件
//! #[cfg(test)] #[path] 声明，仅测试构建时编译）。

use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::StringLoader;
    use crate::template::{
        Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper, TemplateDirectiveBody,
        TemplateDirectiveModel,
    };
    use indexmap::IndexMap;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn cfg() -> (Configuration, Arc<StringLoader>) {
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        (c, loader)
    }

    /// 渲染模板，返回输出（模板名唯一，避免 TemplateCache 命中旧模板）
    fn render(
        c: &Configuration,
        loader: &Arc<StringLoader>,
        src: &str,
        root: DynValue,
    ) -> Result<String> {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let name = format!("t{n}.ftl");
        loader.put(&name, src);
        let t = c.get_template(&name)?;
        let root_model = SimpleObjectWrapper
            .wrap(&root)?
            .unwrap_or_else(TModel::nothing);
        let mut out = Vec::new();
        t.process(root_model, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    fn no_root() -> DynValue {
        DynValue::Map(vec![])
    }

    #[test]
    fn if_elseif_else() {
        let (c, l) = cfg();
        let src = r#"<#if x == 1>one<#elseif x == 2>two<#else>other</#if>"#;
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("x".into(), DynValue::Int(1))])
            )
            .unwrap(),
            "one"
        );
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("x".into(), DynValue::Int(2))])
            )
            .unwrap(),
            "two"
        );
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("x".into(), DynValue::Int(9))])
            )
            .unwrap(),
            "other"
        );
    }

    #[test]
    fn list_loop_with_index_and_sep_else() {
        let (c, l) = cfg();
        let src = r#"<#list xs as x>${x_index}:${x}<#sep>,</#sep><#else>none</#list>"#;
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![(
                    "xs".into(),
                    DynValue::List(vec![DynValue::Int(1), DynValue::Int(2), DynValue::Int(3)])
                )])
            )
            .unwrap(),
            "0:1,1:2,2:3"
        );
        // 空序列 → else
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("xs".into(), DynValue::List(vec![]))])
            )
            .unwrap(),
            "none"
        );
    }

    #[test]
    fn list_items_and_else() {
        let (c, l) = cfg();
        // #items 循环变量名被解析器丢弃（grammar.rs 已知限制），用可观察输出验证阶段逻辑
        let src = "<#list xs>PRE<#items as x>ITEM</#items><#else>NONE</#list>";
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![(
                    "xs".into(),
                    DynValue::List(vec![DynValue::Int(1), DynValue::Int(2)])
                )])
            )
            .unwrap(),
            "PREITEMITEM"
        );
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("xs".into(), DynValue::List(vec![]))])
            )
            .unwrap(),
            "NONE"
        );
    }

    #[test]
    fn list_without_var_items_kv_on_hash() {
        let (c, l) = cfg();
        // Java FTL.jj Items :2943-2953：`<#list hash>`（无 as）+ `<#items as k, v>`
        // 的 iterCtx.hashListing 由 #items 置位 → 按键/值对列出（listhash 用例模式）
        let src = r#"<#setting boolean_format="Y,N"><#list m><#items as k, v>${k}=${v};</#items></#list>"#;
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![(
                    "m".into(),
                    DynValue::Map(vec![
                        ("a".into(), DynValue::Int(1)),
                        ("b".into(), DynValue::Int(2)),
                    ])
                )])
            )
            .unwrap(),
            "a=1;b=2;"
        );
        // 空哈希 → 列表体不执行，<#else> 生效
        let src = "<#list m><#items as k, v>${k}=${v};</#items><#else>Empty</#list>";
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("m".into(), DynValue::Map(vec![]))])
            )
            .unwrap(),
            "Empty"
        );
        // 无 <#items as k, v>（单变量 items / 无 items）→ 哈希不可列出（Java
        // CollOrSeqListing 的 TemplateHashModelEx 分支同样报错；v1 消息简化）
        let src = "<#list m><#items as k>${k}</#items></#list>";
        let err = render(
            &c,
            &l,
            src,
            DynValue::Map(vec![(
                "m".into(),
                DynValue::Map(vec![("a".into(), DynValue::Int(1))]),
            )]),
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("must be a sequence or collection"),
            "hash with 1-var #items rejected: {err}"
        );
    }

    #[test]
    fn list_collection_and_range() {
        let (c, l) = cfg();
        assert_eq!(
            render(&c, &l, "<#list 1..3 as i>${i}</#list>", no_root()).unwrap(),
            "123"
        );
        assert_eq!(
            render(
                &c,
                &l,
                "<#list (1..*5) as i>${i}<#if i?has_next>,</#if></#list>",
                no_root()
            )
            .unwrap(),
            "1,2,3,4,5"
        );
    }

    #[test]
    fn assign_8_operators() {
        let (c, l) = cfg();
        // 5+2=7, 7-1=6, 6*3=18, 18/2=9, 9%3=0
        let src = r#"<#assign x = 5><#assign x += 2><#assign x -= 1><#assign x *= 3><#assign x /= 2><#assign x %= 3>${x}"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "0");
        let src = r#"<#assign y = 1><#assign y ++>${y}<#assign y -->${y}"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "21");
        // 字符串 += 拼接（AddConcat 语义）
        let src = r#"<#assign s = "a"><#assign s += "b">${s}"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "ab");
    }

    #[test]
    fn global_and_local() {
        let (c, l) = cfg();
        let src = r#"<#global g = 1><#macro m><#local g = 2>${g}</#macro><@m/>${g}"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "21");
    }

    #[test]
    fn macro_default_nested_return() {
        let (c, l) = cfg();
        // 默认参数 + nested + 返回值
        let src = r#"<#macro greet name="world">Hello ${name}!<#nested></#macro>
<@greet>!</@greet>
<@greet name="rust"/>
<#function double x><#return x * 2></#function>
${double(21)}"#;
        let out = render(&c, &l, src, no_root()).unwrap();
        // Java 空白剥离：行首块结束标签 </#function> 后的换行被剥除（TextBlock.openingCharsToStrip）
        assert_eq!(out, "Hello world!!\nHello rust!42");
    }

    #[test]
    fn macro_catch_all_and_positional() {
        let (c, l) = cfg();
        let src = r#"<#macro m a b...>${a}|${b?join(",")}</#macro><@m 1 2 3/>"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "1|2,3");
        // 命名 catch-all（未声明参数进入 rest 哈希；默认参数 a=1 被显式覆盖）
        let src = r#"<#macro n a=1 rest...>${a}:${rest?keys?join(",")}</#macro><@n x=9 a=7/>"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "7:x");
    }

    #[test]
    fn macro_missing_required_param_errors() {
        let (c, l) = cfg();
        let err = render(&c, &l, r#"<#macro m a>${a}</#macro><@m/>"#, no_root()).unwrap_err();
        assert!(err.to_string().contains("required parameter"), "{err}");
    }

    #[test]
    fn nested_parameter_binding() {
        let (c, l) = cfg();
        // <@m ; bp>body</@m> + <#nested v> → bp 绑定 v
        let src = r#"<#macro m><#nested 42></#macro><@m ; bp>got=${bp}</@m>"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "got=42");
    }

    #[test]
    fn switch_fallthrough_and_default() {
        let (c, l) = cfg();
        let src = r#"<#switch x><#case 1>one<#case 2>two<#default>other</#switch>"#;
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("x".into(), DynValue::Int(1))])
            )
            .unwrap(),
            "onetwoother"
        );
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("x".into(), DynValue::Int(9))])
            )
            .unwrap(),
            "other"
        );
    }

    #[test]
    fn attempt_recover() {
        let (c, l) = cfg();
        let src = r#"<#attempt>before${missing}after<#recover>caught</#attempt>done"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "caughtdone");
    }

    #[test]
    fn include_and_import() {
        let (c, l) = cfg();
        l.put("sub/part.ftl", "part:${x}");
        l.put("main.ftl", r#"<#include "sub/part.ftl">"#);
        let t = c.get_template("main.ftl").unwrap();
        let root = SimpleObjectWrapper
            .wrap(&DynValue::Map(vec![("x".into(), DynValue::Int(7))]))
            .unwrap()
            .unwrap();
        let mut out = Vec::new();
        t.process(root, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "part:7");

        // import：命名空间宏
        l.put(
            "lib.ftl",
            r#"<#macro libMsg>lib!</#macro><#assign libVar = 1>"#,
        );
        let src = r#"<#import "lib.ftl" as lib><@lib.libMsg/>${lib.libVar}"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "lib!1");
    }

    #[test]
    fn escape_html() {
        let (c, l) = cfg();
        let src = r#"<#escape x as x?html>${x}</#escape>"#;
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("x".into(), DynValue::Str("<a>&".into()))])
            )
            .unwrap(),
            "&lt;a&gt;&amp;"
        );
        // noescape 取消
        let src = r#"<#escape x as x?html>${x}<#noescape>${x}</#noescape></#escape>"#;
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("x".into(), DynValue::Str("<a>".into()))])
            )
            .unwrap(),
            "&lt;a&gt;<a>"
        );
    }

    #[test]
    fn string_interpolation_and_booleans() {
        let (c, l) = cfg();
        // Java：默认 boolean_format "true,false" 是遗留默认 → 插值报错；?c 显式输出
        let src = r#"${"msg=" + msg} ${b?c}"#;
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![
                    ("msg".into(), DynValue::Str("hi".into())),
                    ("b".into(), DynValue::Bool(true)),
                ])
            )
            .unwrap(),
            "msg=hi true"
        );
        // boolean_format 设置生效
        let src = r#"<#setting boolean_format="yes,no">${b}"#;
        assert_eq!(
            render(
                &c,
                &l,
                src,
                DynValue::Map(vec![("b".into(), DynValue::Bool(true))])
            )
            .unwrap(),
            "yes"
        );
    }

    #[test]
    fn break_continue_in_loop() {
        let (c, l) = cfg();
        let src = r#"<#list 1..10 as i><#if i == 3><#break></#if>${i}</#list>"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "12");
        let src = r#"<#list 1..4 as i><#if i == 2><#continue></#if>${i}</#list>"#;
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "134");
    }

    #[test]
    fn stop_terminates() {
        let (c, l) = cfg();
        let err = render(&c, &l, "a<#stop \"boom\">b", no_root()).unwrap_err();
        match err {
            TemplateError::Stop { message } => {
                // Java StopException.getMessage() = "boom" + FTL stack trace 段
                // （jar 实测 stop 基线）——消息主体断言去栈段
                assert!(
                    message.as_deref().is_some_and(|m| m.starts_with("boom")),
                    "{message:?}"
                );
            }
            other => panic!("expected Stop, got {other:?}"),
        }
    }

    #[test]
    fn type_mismatch_error() {
        let (c, l) = cfg();
        let err = render(&c, &l, "<#if 1>y</#if>", no_root()).unwrap_err();
        assert!(matches!(err, TemplateError::TypeMismatch { .. }), "{err}");
        assert!(err.to_string().contains("boolean"), "{err}");
    }

    #[test]
    fn trim_and_compress() {
        let (c, l) = cfg();
        assert_eq!(
            render(&c, &l, "<#trim>  a \n b  </#trim>", no_root()).unwrap(),
            "a \n b"
        );
        assert_eq!(
            render(
                &c,
                &l,
                "<#compress>  a   \n\n   b \n  c  </#compress>",
                no_root()
            )
            .unwrap(),
            "a\nb\nc"
        );
    }

    #[test]
    fn whitespace_stripping_applies() {
        let (c, l) = cfg();
        // 剥离在解析期直接改写文本（Java TextBlock.postParseCleanup：text = substring）。
        // openingCharsToStrip 只剥到首个换行（含）为止，换行后的缩进保留；
        // 模板首元素为 <#if>（无前一同行终端）→ 剥离成立。
        let src = "<#if true>\n  yes\n</#if>";
        assert_eq!(render(&c, &l, src, no_root()).unwrap(), "  yes\n");
        // Java PropertySetting：配置级设置（whitespace_stripping）在模板内修改 → 解析错误
        // （"The setting name is recognized, but changing this setting from inside a
        //   template isn't supported."，PropertySetting.java:71-82）
        let src = "<#setting whitespace_stripping=false><#if true>\n  yes\n</#if>";
        let err = render(&c, &l, src, no_root()).unwrap_err();
        assert!(
            err.to_string().contains("isn't supported"),
            "config-level setting rejected at parse: {err}"
        );
    }

    #[test]
    fn custom_directive_via_root() {
        let (c, l) = cfg();
        let d = TModel::from_directive(UpperDirective);
        let mut root_map = IndexMap::new();
        root_map.insert("upper".to_string(), d);
        let root = TModel::from_hash(root_map);
        l.put("t.ftl", r#"<@upper x="hi">body</@upper>"#);
        let t = c.get_template("t.ftl").unwrap();
        let mut out = Vec::new();
        t.process(root, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "HI+body");
    }

    struct UpperDirective;
    impl TemplateDirectiveModel for UpperDirective {
        fn execute(
            &self,
            env: &mut crate::core::Environment,
            params: &HashMap<String, TModel>,
            _loop_vars: &mut [TModel],
            body: Option<&dyn TemplateDirectiveBody>,
        ) -> Result<()> {
            let x = params.get("x").cloned().unwrap_or_else(TModel::nothing);
            let s = x.get_scalar()?;
            env.emit(&s.to_uppercase())?;
            if let Some(b) = body {
                env.emit("+")?;
                b.render(env)?;
            }
            Ok(())
        }
    }

    #[test]
    fn loop_builtins() {
        let (c, l) = cfg();
        let src = r#"<#list ["a","b","c"] as x>${x?index}/${x?counter}/${x?is_first?c}/${x?is_last?c}/${x?has_next?c};</#list>"#;
        assert_eq!(
            render(&c, &l, src, no_root()).unwrap(),
            "0/1/true/false/true;1/2/false/false/true;2/3/false/true/false;"
        );
    }
}

#[cfg(test)]
mod golden {
    use crate::cache::StringLoader;
    use crate::core::Environment;
    use crate::error::{Result, TemplateError};
    use crate::template::{Configuration, TModel, TemplateDirectiveBody, TemplateDirectiveModel};
    use indexmap::IndexMap;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Java templatesuite 仓库内副本（freemarker-test/tests/suite/：templates/ 134 个
    // 模板 + expected/ 94 个期望输出；与 Java 仓库逐字节一致，extract_suite.py 提取）
    const SUITE_DIR: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../freemarker-test/tests/suite"
    );

    fn read(src: &str) -> String {
        std::fs::read_to_string(src).unwrap_or_else(|e| panic!("cannot read {src}: {e}"))
    }

    /// 剥掉 expected 文件开头的 `/* ... */` 许可证注释块（Java 侧同样先剥除后比较）
    fn strip_license_comment(s: &str) -> String {
        let s = s.trim_start();
        if let Some(rest) = s.strip_prefix("/*") {
            if let Some(i) = rest.find("*/") {
                let out = &rest[i + 2..];
                return out.strip_prefix('\n').unwrap_or(out).to_string();
            }
        }
        s.to_string()
    }

    fn render_golden(name: &str, src: &str, root: TModel) -> String {
        let mut c = Configuration::new();
        let loader = Arc::new(StringLoader::default());
        c.template_loader = loader.clone();
        loader.put(name, src);
        let t = c.get_template(name).unwrap();
        let mut out = Vec::new();
        t.process(root, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn golden_helloworld() {
        let src = read(&format!("{SUITE_DIR}/templates/helloworld.ftl"));
        let expected =
            strip_license_comment(&read(&format!("{SUITE_DIR}/expected/helloworld.txt")));
        // 数据模型：`exec` 方法（对应 harness 提供的 Jython 模型，返回 "Hello, world!\n"）
        let mut m = IndexMap::new();
        m.insert("exec".to_string(), TModel::from_method(ExecHelloWorld));
        let out = render_golden("golden-hello.ftl", &src, TModel::from_hash(m));
        assert_eq!(out, expected, "helloworld golden mismatch");
    }

    struct ExecHelloWorld;
    impl crate::template::TemplateMethodModelEx for ExecHelloWorld {
        fn exec(&self, _env: &mut Environment, _args: Vec<TModel>) -> Result<TModel> {
            Ok(TModel::from_scalar("Hello, world!\n".to_string()))
        }
    }

    #[test]
    fn golden_boolean() {
        let src = read(&format!("{SUITE_DIR}/templates/boolean.ftl"));
        let expected = strip_license_comment(&read(&format!("{SUITE_DIR}/expected/boolean.txt")));
        // 数据模型对应 TemplateTestCase.java:261-274（boolean 测试）
        let mut m = IndexMap::new();
        m.insert(
            "message".to_string(),
            TModel::from_scalar("Hello, world!".into()),
        );
        m.insert("boolean1".to_string(), TModel::from_boolean(false));
        m.insert("boolean2".to_string(), TModel::from_boolean(true));
        m.insert("boolean3".to_string(), TModel::from_boolean(true));
        m.insert("boolean4".to_string(), TModel::from_boolean(true));
        m.insert("boolean5".to_string(), TModel::from_boolean(false));
        m.insert(
            "list1".to_string(),
            TModel::from_sequence(vec![
                TModel::from_scalar("false".into()),
                TModel::from_scalar("0".into()),
                TModel::from_boolean(false),
                TModel::from_boolean(true),
                TModel::from_boolean(true),
                TModel::from_boolean(true),
                TModel::from_boolean(false),
            ]),
        );
        m.insert("list2".to_string(), TModel::from_sequence(vec![]));
        m.insert(
            "hash1".to_string(),
            TModel::from_hash({
                let mut h = IndexMap::new();
                h.insert(
                    "temp".to_string(),
                    TModel::from_scalar("Hello, world.".into()),
                );
                h.insert("boolean".to_string(), TModel::from_boolean(false));
                h
            }),
        );
        m.insert("hash2".to_string(), TModel::from_hash(IndexMap::new()));
        m.insert(
            "assert".to_string(),
            TModel::from_directive(AssertDirective),
        );
        let out = render_golden("golden-boolean.ftl", &src, TModel::from_hash(m));
        assert_eq!(out, expected, "boolean golden mismatch");
    }

    /// 对应 harness 的 AssertDirective（参数 test 为布尔；假则报错）
    struct AssertDirective;
    impl TemplateDirectiveModel for AssertDirective {
        fn execute(
            &self,
            _env: &mut crate::core::Environment,
            params: &HashMap<String, TModel>,
            _loop_vars: &mut [TModel],
            _body: Option<&dyn TemplateDirectiveBody>,
        ) -> Result<()> {
            let test = params
                .get("test")
                .ok_or_else(|| TemplateError::misc("Missing required parameter \"test\""))?;
            let b = test.eval_boolean()?;
            if !b {
                return Err(TemplateError::misc("Assertion failed"));
            }
            Ok(())
        }
    }

    #[test]
    fn golden_variables() {
        let src = read(&format!("{SUITE_DIR}/templates/variables.ftl"));
        let expected = strip_license_comment(&read(&format!("{SUITE_DIR}/expected/variables.txt")));
        let mut m = IndexMap::new();
        m.insert(
            "message".to_string(),
            TModel::from_scalar("Hello, world!".into()),
        );
        let out = render_golden("golden-vars.ftl", &src, TModel::from_hash(m));
        assert_eq!(out, expected, "variables golden mismatch");
    }

    #[test]
    fn golden_if() {
        let mut src = read(&format!("{SUITE_DIR}/templates/if.ftl"));
        // 末尾的 `<@assertFails ...?interpret .../>` 段仅验证错误行为、不产生输出
        // （?interpret 动态解释模板属 P4），截去后输出与 Java 逐字节一致
        if let Some(i) = src.find("<#-- parsing errors -->") {
            src.truncate(i);
        }
        let expected = strip_license_comment(&read(&format!("{SUITE_DIR}/expected/if.txt")));
        let mut m = IndexMap::new();
        m.insert(
            "message".to_string(),
            TModel::from_scalar("Hello, world!".into()),
        );
        let out = render_golden("golden-if.ftl", &src, TModel::from_hash(m));
        if out != expected {
            std::fs::write("/tmp/if_mine.txt", &out).unwrap();
            std::fs::write("/tmp/if_expected.txt", &expected).unwrap();
        }
        assert_eq!(out, expected, "if golden mismatch");
    }
}
