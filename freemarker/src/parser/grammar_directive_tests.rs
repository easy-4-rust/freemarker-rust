//! 核心指令解析测试。

use super::parse;
use crate::core::{AssignOp, CallTarget, ElementKind, ExprKind};
use crate::template::{Configuration, Template};
use crate::value::TNumber;
use std::rc::Rc;

fn cfg() -> Rc<Configuration> {
    Rc::new(Configuration::new())
}

fn parse_ok(src: &str) -> Template {
    parse(&cfg(), "t", src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {e}"))
}

#[allow(dead_code)]
fn parse_err(src: &str) -> String {
    match parse(&cfg(), "t", src) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("expected parse error for {src:?}"),
    }
}

#[allow(dead_code)]
fn expr_of(src: &str) -> ExprKind {
    let t = parse_ok(&format!("${{{src}}}"));
    match &t.root[0].kind {
        ElementKind::Interpolation { expr, .. } => expr.kind.clone(),
        k => panic!("expected interpolation, got {k:?}"),
    }
}

fn num(v: TNumber) -> ExprKind {
    ExprKind::Num(v)
}

fn ident(n: &str) -> ExprKind {
    ExprKind::Ident(n.to_string())
}

fn strlit(s: &str) -> ExprKind {
    ExprKind::Str(s.to_string())
}

// -----------------------------------------------------------------------

#[test]
fn if_elseif_else_flattening() {
    let t = parse_ok("<#if x>a</#if>");
    let ElementKind::If { cond, then, else_ } = &t.root[0].kind else {
        panic!("expected If, got {:?}", t.root[0].kind);
    };
    assert_eq!(cond.kind, ident("x"));
    assert_eq!(then.len(), 1);
    assert!(else_.is_none());
    assert!(matches!(then[0].kind, ElementKind::Text { ref text, .. } if text == "a"));

    let t = parse_ok("<#if x>a<#elseif y>b<#else>c</#if>");
    let ElementKind::If { cond, then, else_ } = &t.root[0].kind else {
        panic!("expected If");
    };
    assert_eq!(cond.kind, ident("x"));
    assert!(matches!(then[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
    let else_ = else_.as_ref().expect("else branch");
    let ElementKind::If { cond, then, else_ } = &else_[0].kind else {
        panic!("expected nested If for elseif, got {:?}", else_[0].kind);
    };
    assert_eq!(cond.kind, ident("y"));
    assert!(matches!(then[0].kind, ElementKind::Text { ref text, .. } if text == "b"));
    let else_ = else_.as_ref().expect("nested else branch");
    assert!(matches!(else_[0].kind, ElementKind::Text { ref text, .. } if text == "c"));
}

#[test]
fn list_with_items_sep_else() {
    let t = parse_ok("<#list xs as x>${x}</#list>");
    let ElementKind::List {
        seq,
        var,
        var2,
        body,
        else_,
    } = &t.root[0].kind
    else {
        panic!("expected List");
    };
    assert_eq!(seq.kind, ident("xs"));
    assert_eq!(var, "x");
    assert!(var2.is_none());
    assert!(matches!(body[0].kind, ElementKind::Interpolation { .. }));
    assert!(else_.is_none());

    // items/sep 是就地元素（Java Items/Sep 模型），不再抽入 List 字段
    let t = parse_ok("<#list xs><#items as x>${x}</#items><#sep>,</#sep><#else>none</#list>");
    let ElementKind::List {
        var, body, else_, ..
    } = &t.root[0].kind
    else {
        panic!("expected List");
    };
    assert_eq!(var, "");
    let ElementKind::Items {
        var,
        body: items_body,
        ..
    } = &body[0].kind
    else {
        panic!("expected Items at body[0], got {:?}", body[0].kind);
    };
    assert_eq!(var, "x");
    assert!(matches!(
        items_body[0].kind,
        ElementKind::Interpolation { .. }
    ));
    let ElementKind::Sep { body: sep_body } = &body[1].kind else {
        panic!("expected Sep at body[1], got {:?}", body[1].kind);
    };
    assert!(matches!(sep_body[0].kind, ElementKind::Text { ref text, .. } if text == ","));
    let else_ = else_.as_ref().expect("else");
    assert!(matches!(else_[0].kind, ElementKind::Text { ref text, .. } if text == "none"));
}

#[test]
fn list_hash_listing_two_vars() {
    // `as k, v`：双循环变量（hashListing；Java IteratorBlock.loopVar2Name）
    let t = parse_ok("<#list h as k, v>${k}=${v}</#list>");
    let ElementKind::List { var, var2, .. } = &t.root[0].kind else {
        panic!("expected List");
    };
    assert_eq!(var, "k");
    assert_eq!(var2.as_deref(), Some("v"));
    // 键值同名 → 报错（Java 消息）
    let msg = parse_err("<#list h as k, k>x</#list>");
    assert!(msg.contains("must differ"), "{msg}");
    // items 双变量
    let t = parse_ok("<#list h><#items as k, v>${k}=${v}</#items></#list>");
    let ElementKind::List { body, .. } = &t.root[0].kind else {
        panic!("expected List");
    };
    let ElementKind::Items { var, var2, .. } = &body[0].kind else {
        panic!("expected Items");
    };
    assert_eq!(var, "k");
    assert_eq!(var2.as_deref(), Some("v"));
}

#[test]
fn list_validation_errors() {
    // 无 as 也无 items → 报错（Java 消息）
    let msg = parse_err("<#list xs></#list>");
    assert!(
        msg.contains("#list must have either \"as loopVar\""),
        "{msg}"
    );
    // as var + items → 报错
    let msg = parse_err("<#list xs as x><#items as y></#items></#list>");
    assert!(msg.contains("must not have \"as loopVar\""), "{msg}");
    // #items 在 list 外 → 报错（Java 消息）
    let msg = parse_err("<#items as x>y</#items>");
    assert!(msg.contains("#items must be inside a #list"), "{msg}");
    // #sep 在 list 外 → 报错
    let msg = parse_err("<#sep>x</#sep>");
    assert!(msg.contains("#sep must be inside a #list"), "{msg}");
    // #items 嵌套 #items → 报错
    let msg = parse_err("<#list xs><#items as x><#items as y></#items></#items></#list>");
    assert!(msg.contains("Can't nest #items"), "{msg}");
    // #foreach 内 #items → 报错（Java：foreach 不支持嵌套 items，消息逐字）
    let msg = parse_err("<#foreach x in xs><#items as y></#items></#foreach>");
    assert!(
        msg.contains("#foreach doesn't support nested #items."),
        "{msg}"
    );
}

#[test]
fn list3_sequential_items_and_sep_auto_close() {
    // list3：同一 #list 中顺序多个 #items 合法（Java FTL.jj：END_ITEMS 后
    // iterCtx.loopVarName = null，:2966-2968；运行时由
    // IterationContext.alreadyEntered 校验，IteratorBlock.java:250-254）
    let t = parse_ok("<#list xs><#items as x>${x}</#items><#items as y>${y}</#items></#list>");
    let ElementKind::List { body, .. } = &t.root[0].kind else {
        panic!("expected List");
    };
    assert!(matches!(body[0].kind, ElementKind::Items { .. }));
    assert!(matches!(body[1].kind, ElementKind::Items { .. }));
    // list3 用例形态：switch 不同分支内的多个 #items（macro hits）
    let t = parse_ok(
            "<#list xs><#switch s><#case \"a\"><#items as x>${x}</#items><#break><#default><#items as x>${x}</#items></#switch></#list>",
        );
    assert_eq!(t.root.len(), 1);
    // 真正嵌套（items 体内部再开 items）仍报错
    let msg = parse_err("<#list xs><#items as x><#items as y></#items></#items></#list>");
    assert!(msg.contains("Can't nest #items"), "{msg}");
    // #sep 自动闭合（Java Sep() 的 END_SEP 可选，FTL.jj 2988-2990）：
    // `</#list>`（list-bis 第 20/24/35/48 行形态）
    let t = parse_ok("<#list xs as x>${x}<#sep>, </#list>");
    let ElementKind::List { body, .. } = &t.root[0].kind else {
        panic!("expected List");
    };
    assert!(matches!(body[1].kind, ElementKind::Sep { .. }));
    // `</#items>`（list3 第 69 行形态）
    let t = parse_ok("<#list xs><#items as x>${x}<#sep>, </#items></#list>");
    assert_eq!(t.root.len(), 1);
    // `<#else>`（list3 第 67-68 行形态：else 归属外层 list）
    let t = parse_ok("<#list xs as x>${x}<#sep>, <#else>empty</#list>");
    let ElementKind::List { else_, .. } = &t.root[0].kind else {
        panic!("expected List");
    };
    assert!(else_.is_some(), "sep 自动闭合后 else 仍归外层 list");
    // `</#if>`（sep 在 if 体内，Java 同样允许 —— Sep 的 MixedContentElements
    // 在非元素 token 处终止）
    let t = parse_ok("<#list xs as x><#if x == 'a'>${x}<#sep>X</#if>${x}</#list>");
    assert_eq!(t.root.len(), 1);
    // 显式 `</#sep>` 照常
    let t = parse_ok("<#list xs as x>${x}<#sep>, </#sep></#list>");
    assert_eq!(t.root.len(), 1);
    // 未闭合 #sep 且模板结束 → 外层 #list 报未闭合
    let msg = parse_err("<#list xs as x>${x}<#sep>, ");
    assert!(msg.contains("Unclosed"), "{msg}");
}

#[test]
fn legacy_interpolation_format_spec() {
    // `#{ expr ; mNMN }`（Java NumericalOutput，FTL.jj 2627-2703；arithmetic 用例）
    let t = parse_ok("#{ x + y ; m2M3}");
    let ElementKind::Interpolation {
        legacy_min_frac,
        legacy_max_frac,
        ..
    } = &t.root[0].kind
    else {
        panic!("expected Interpolation");
    };
    assert_eq!(*legacy_min_frac, Some(2));
    assert_eq!(*legacy_max_frac, Some(3));
    // 无格式串 → (0, 50)（Java NumericalOutput(exp, autoEscOF) 默认）
    let t = parse_ok("#{y/x}");
    let ElementKind::Interpolation {
        legacy_min_frac,
        legacy_max_frac,
        ..
    } = &t.root[0].kind
    else {
        panic!("expected Interpolation");
    };
    assert_eq!(*legacy_min_frac, Some(0));
    assert_eq!(*legacy_max_frac, Some(50));
    // `${...}` 不是旧式插值（legacy_min_frac = None）
    let t = parse_ok("${x}");
    let ElementKind::Interpolation {
        legacy_min_frac, ..
    } = &t.root[0].kind
    else {
        panic!("expected Interpolation");
    };
    assert_eq!(*legacy_min_frac, None);
    // `m3` → min=max=3；`M4` → min=0、max=4（Java :2687-2698 缺省规则）
    for (src, min, max) in [("#{y ; m3}", 3u32, 3u32), ("#{y ; M4}", 0, 4)] {
        let t = parse_ok(src);
        let ElementKind::Interpolation {
            legacy_min_frac,
            legacy_max_frac,
            ..
        } = &t.root[0].kind
        else {
            panic!("expected Interpolation for {src}");
        };
        assert_eq!(*legacy_min_frac, Some(min), "min for {src}");
        assert_eq!(*legacy_max_frac, Some(max), "max for {src}");
    }
    // 格式串错误（Java 消息逐字，ProbeP2 jar 实测）
    let msg = parse_err("#{y ; xyz}");
    assert!(msg.contains("Invalid format specifier xyz"), "{msg}");
    let msg = parse_err("#{y ; m9M3}");
    assert!(
        msg.contains("Invalid format specification, min cannot be greater than max!"),
        "{msg}"
    );
    let msg = parse_err("#{y ; m60}");
    assert!(
        msg.contains("Cannot specify more than 50 fraction digits"),
        "{msg}"
    );
    let msg = parse_err("#{y ; 12}");
    assert!(
        msg.contains("Encountered \"12\", but was expecting pattern: <ID>"),
        "{msg}"
    );
    // 数字解析失败 → "Invalid number in the format specifier"（Java :2645-2687：
    // StringTokenizer 按 m/M 切分，"m2x" → ["m", "2x"] → parseInt("2x") 失败）
    let msg = parse_err("#{y ; m2x}");
    assert!(
        msg.contains("Invalid number in the format specifier m2x"),
        "{msg}"
    );
    let msg = parse_err("#{y ; m99999999999999}");
    assert!(
        msg.contains("Invalid number in the format specifier m99999999999999"),
        "{msg}"
    );
    // 尾随 m/M（无数字）：Java StringTokenizer 语义下忽略（m2m → min=max=2、
    // m2M → min=max=2，jar 实测输出 "2.00"）
    for src in ["#{y ; m2m}", "#{y ; m2M}"] {
        let t = parse_ok(src);
        let ElementKind::Interpolation {
            legacy_min_frac,
            legacy_max_frac,
            ..
        } = &t.root[0].kind
        else {
            panic!("expected Interpolation");
        };
        assert_eq!(*legacy_min_frac, Some(2), "min for {src}");
        assert_eq!(*legacy_max_frac, Some(2), "max for {src}");
    }
    // 字面量校验（numberLiteralOnly，消息逐字）
    let msg = parse_err("#{ \"a\" }");
    assert!(
        msg.contains("Found string literal: \"a\". Expecting: number"),
        "{msg}"
    );
    let msg = parse_err("#{ [1, 2] }");
    assert!(
        msg.contains("Found list literal: [1, 2]. Expecting number"),
        "{msg}"
    );
    let msg = parse_err("#{ {\"a\": 1} }");
    assert!(
        msg.contains("Found hash literal: {\"a\": 1}. Expecting number"),
        "{msg}"
    );
    let msg = parse_err("#{ true }");
    assert!(
        msg.contains("Found: true literal. Expecting number"),
        "{msg}"
    );
    // `${...}` 不受影响
    assert!(parse_ok("${ \"a\" }").root.len() == 1);
}

#[test]
fn assign_variants() {
    let t = parse_ok("<#assign x = 1>");
    let ElementKind::Assign {
        target,
        expr,
        op,
        namespace,
    } = &t.root[0].kind
    else {
        panic!("expected Assign");
    };
    assert_eq!(target, "x");
    assert_eq!(expr.kind, num(TNumber::Int(1)));
    assert_eq!(*op, AssignOp::Equals);
    assert!(namespace.is_none());

    for (src, expected_op) in [
        ("<#assign x += 1>", AssignOp::PlusEq),
        ("<#assign x -= 1>", AssignOp::MinusEq),
        ("<#assign x *= 2>", AssignOp::TimesEq),
        ("<#assign x /= 2>", AssignOp::DivideEq),
        ("<#assign x %= 2>", AssignOp::ModuloEq),
        ("<#assign x++>", AssignOp::PlusPlus),
        ("<#assign x-->", AssignOp::MinusMinus),
    ] {
        let t = parse_ok(src);
        let ElementKind::Assign { op, .. } = &t.root[0].kind else {
            panic!("expected Assign for {src}");
        };
        assert_eq!(*op, expected_op, "op for {src}");
    }

    // 命名空间
    let t = parse_ok("<#assign x = 1 in ns>");
    let ElementKind::Assign { namespace, .. } = &t.root[0].kind else {
        panic!("expected Assign");
    };
    // namespace 是 Option<Expr>（运行期求值取名字符串）；`in ns` 解析为 Ident("ns")
    let ns_name = namespace.as_ref().map(|e| match &e.kind {
        ExprKind::Ident(n) => n.clone(),
        _ => String::new(),
    });
    assert_eq!(ns_name, Some("ns".to_string()));

    // 块赋值
    let t = parse_ok("<#assign x>body</#assign>");
    assert!(matches!(t.root[0].kind, ElementKind::BlockAssign { .. }));

    // global / local
    let t = parse_ok("<#global x = 1>");
    assert!(matches!(t.root[0].kind, ElementKind::Global { .. }));
    let t = parse_ok("<#macro m><#local x = 1></#macro>");
    let ElementKind::Macro { def } = &t.root[0].kind else {
        panic!("expected Macro");
    };
    assert!(matches!(def.body[0].kind, ElementKind::Local { .. }));
    // local 在宏外 → 报错（Java 消息）
    let msg = parse_err("<#local x = 1>");
    assert!(
        msg.contains("Local variable assigned outside a macro"),
        "{msg}"
    );
}

#[test]
fn macro_and_function() {
    let t = parse_ok("<#macro m a b=2>body</#macro>");
    let ElementKind::Macro { def } = &t.root[0].kind else {
        panic!("expected Macro");
    };
    assert_eq!(def.name, "m");
    assert!(!def.is_function);
    assert_eq!(def.params.len(), 2);
    assert_eq!(def.params[0].name, "a");
    assert!(def.params[0].default.is_none());
    assert!(!def.params[0].optional);
    assert_eq!(def.params[1].name, "b");
    assert!(def.params[1].default.is_some());
    assert!(def.params[1].optional);
    // 宏表注册
    assert!(t.macros.contains_key("m"));

    // 字符串名 + catch-all 参数
    let t = parse_ok(r#"<#macro "catch-all" foo bar...>x</#macro>"#);
    let ElementKind::Macro { def } = &t.root[0].kind else {
        panic!("expected Macro");
    };
    assert_eq!(def.name, "catch-all");
    assert_eq!(def.params.len(), 2);
    assert!(def.params[1].catch_all);
    assert!(def.params[1].optional);

    // function
    let t = parse_ok("<#function f x>${x}</#function>");
    let ElementKind::Macro { def } = &t.root[0].kind else {
        panic!("expected Macro");
    };
    assert!(def.is_function);
    assert_eq!(def.name, "f");

    // 参数顺序校验（默认值参数后不能再有必选参数）
    let msg = parse_err("<#macro m a=1 b>x</#macro>");
    assert!(
        msg.contains("parameters without a default value must all occur before"),
        "{msg}"
    );
    // 宏嵌套 → 报错
    let msg = parse_err("<#macro a><#macro b></#macro></#macro>");
    assert!(msg.contains("can't be nested"), "{msg}");
}

#[test]
fn user_directive_calls() {
    // 命名参数 + 自闭合
    let t = parse_ok("<@m x=1/>");
    let ElementKind::Call {
        callee,
        args,
        body,
        body_params,
    } = &t.root[0].kind
    else {
        panic!("expected Call");
    };
    assert_eq!(*callee, CallTarget::Name("m".to_string()));
    assert_eq!(args.len(), 1);
    assert_eq!(args[0].0, "x");
    assert_eq!(args[0].1.kind, num(TNumber::Int(1)));
    assert!(body.is_none() && body_params.is_empty());

    // 命名空间调用
    let t = parse_ok("<@ns.m/>");
    let ElementKind::Call { callee, .. } = &t.root[0].kind else {
        panic!("expected Call");
    };
    assert_eq!(
        *callee,
        CallTarget::Namespaced {
            ns: "ns".to_string(),
            name: "m".to_string(),
        }
    );

    // 位置参数（契约 args 以空名存储）
    let t = parse_ok("<@m 1 2/>");
    let ElementKind::Call { args, .. } = &t.root[0].kind else {
        panic!("expected Call");
    };
    assert_eq!(args.len(), 2);
    assert_eq!(args[0].0, "");
    assert_eq!(args[1].0, "");

    // body + 多 body 参数（<@m x; a, b>）
    let t = parse_ok("<@m x; a, b>body</@m>");
    let ElementKind::Call {
        body, body_params, ..
    } = &t.root[0].kind
    else {
        panic!("expected Call");
    };
    assert_eq!(body_params, &["a".to_string(), "b".to_string()]);
    let body = body.as_ref().expect("body");
    assert!(matches!(body[0].kind, ElementKind::Text { ref text, .. } if text == "body"));

    // 结束标签名不匹配 → 报错（Java：Expecting </@> or </@m>）
    let msg = parse_err("<@m>body</@n>");
    assert!(msg.contains("Expecting </@> or </@m>"), "{msg}");
}

#[test]
fn nested_switch_attempt_break() {
    let t = parse_ok("<#macro m><#nested></#macro>");
    let ElementKind::Macro { def } = &t.root[0].kind else {
        panic!("expected Macro");
    };
    assert!(matches!(def.body[0].kind, ElementKind::Nested { .. }));

    let t = parse_ok("<#macro m><#nested x y></#macro>");
    let ElementKind::Macro { def } = &t.root[0].kind else {
        panic!("expected Macro");
    };
    let ElementKind::Nested { args, .. } = &def.body[0].kind else {
        panic!("expected Nested");
    };
    assert_eq!(args.len(), 2);

    // nested 在宏外 → 报错（Java 消息）
    let msg = parse_err("<#nested>");
    assert!(
        msg.contains("Cannot use a \"nested\" instruction outside a macro"),
        "{msg}"
    );

    let t = parse_ok("<#switch v><#case 1>a<#default>b</#switch>");
    let ElementKind::Switch {
        expr,
        cases,
        default,
        default_pos,
    } = &t.root[0].kind
    else {
        panic!("expected Switch");
    };
    assert_eq!(expr.kind, ident("v"));
    assert_eq!(cases.len(), 1);
    assert_eq!(default_pos, &Some(1));
    assert_eq!(cases[0].value.kind, num(TNumber::Int(1)));
    assert!(matches!(cases[0].body[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
    let default = default.as_ref().expect("default");
    assert!(matches!(default[0].kind, ElementKind::Text { ref text, .. } if text == "b"));

    // 重复 default → 报错
    let msg = parse_err("<#switch v><#default>a<#default>b</#switch>");
    assert!(msg.contains("You already had a #default"), "{msg}");
    // 空 switch 合法（Java switch.ftl 用例 `[<#switch 213></#switch>]` 渲染为空）
    let t = parse_ok("<#switch v></#switch>");
    assert!(matches!(t.root[0].kind, ElementKind::Switch { .. }));

    let t = parse_ok("<#attempt>a<#recover>b</#attempt>");
    let ElementKind::Attempt { try_, recover } = &t.root[0].kind else {
        panic!("expected Attempt");
    };
    assert!(matches!(try_[0].kind, ElementKind::Text { ref text, .. } if text == "a"));
    assert!(matches!(recover[0].kind, ElementKind::Text { ref text, .. } if text == "b"));

    // break 需要循环/switch 上下文（Java 消息含 `<#break>` 标签原文）
    let msg = parse_err("<#break>");
    assert!(msg.contains("<#break> must be nested"), "{msg}");
    let t = parse_ok("<#list xs as x><#break><#continue></#list>");
    let ElementKind::List { body, .. } = &t.root[0].kind else {
        panic!("expected List");
    };
    assert!(matches!(body[0].kind, ElementKind::Break));
    assert!(matches!(body[1].kind, ElementKind::Continue));
}

#[test]
fn return_stop_flush() {
    let t = parse_ok("<#macro m><#return></#macro>");
    let ElementKind::Macro { def } = &t.root[0].kind else {
        panic!("expected Macro");
    };
    assert!(matches!(
        def.body[0].kind,
        ElementKind::Return { expr: None }
    ));

    let t = parse_ok("<#function f><#return x></#function>");
    let ElementKind::Macro { def } = &t.root[0].kind else {
        panic!("expected Macro");
    };
    let ElementKind::Return { expr } = &def.body[0].kind else {
        panic!("expected Return");
    };
    assert_eq!(expr.as_ref().unwrap().kind, ident("x"));

    // macro 返回值 / function 不返回值 → 报错
    let msg = parse_err("<#macro m><#return x></#macro>");
    assert!(msg.contains("A macro cannot return a value"), "{msg}");
    let msg = parse_err("<#function f><#return></#function>");
    assert!(msg.contains("A function must return a value"), "{msg}");
    let msg = parse_err("<#return>");
    assert!(
        msg.contains("only occur inside a macro or function"),
        "{msg}"
    );

    let t = parse_ok("<#stop>");
    assert!(matches!(t.root[0].kind, ElementKind::Stop { msg: None }));
    let t = parse_ok(r#"<#stop "msg">"#);
    let ElementKind::Stop { msg } = &t.root[0].kind else {
        panic!("expected Stop");
    };
    assert_eq!(msg.as_ref().unwrap().kind, strlit("msg"));

    let t = parse_ok("<#flush>");
    assert!(matches!(t.root[0].kind, ElementKind::Flush));
}

#[test]
fn include_import_setting_escape_compress() {
    let t = parse_ok(r#"<#include "x.ftl" parse=true encoding="utf-8">"#);
    let ElementKind::Include { path, attrs } = &t.root[0].kind else {
        panic!("expected Include");
    };
    assert_eq!(path.kind, strlit("x.ftl"));
    assert_eq!(attrs.len(), 2);
    assert_eq!(attrs[0].0, "parse");
    assert_eq!(attrs[0].1.kind, ExprKind::Bool(true));

    // 未知 include 参数 → 报错（Java 消息）
    let msg = parse_err(r#"<#include "x.ftl" foo=1>"#);
    assert!(
        msg.contains("Unsupported named #include parameter"),
        "{msg}"
    );

    let t = parse_ok(r#"<#import "lib.ftl" as ns>"#);
    let ElementKind::Import { path, ns } = &t.root[0].kind else {
        panic!("expected Import");
    };
    assert_eq!(path.kind, strlit("lib.ftl"));
    assert_eq!(ns, "ns");

    let t = parse_ok(r#"<#setting locale="en">"#);
    let ElementKind::Setting { key, value } = &t.root[0].kind else {
        panic!("expected Setting");
    };
    assert_eq!(key, "locale");
    assert_eq!(value.kind, strlit("en"));

    let t = parse_ok("<#escape x as x?html>a</#escape>");
    assert!(matches!(t.root[0].kind, ElementKind::Escape { .. }));
    let t = parse_ok("<#noescape>a</#noescape>");
    assert!(matches!(t.root[0].kind, ElementKind::NoEscape(_)));
    let t = parse_ok("<#compress>a</#compress>");
    assert!(matches!(t.root[0].kind, ElementKind::Compress(_)));
    let t = parse_ok("<#autoesc>a</#autoesc>");
    assert!(matches!(t.root[0].kind, ElementKind::AutoEsc(_)));
    let t = parse_ok("<#noautoesc>a</#noautoesc>");
    assert!(matches!(t.root[0].kind, ElementKind::NoAutoEsc(_)));
    let t = parse_ok(r#"<#outputformat "HTML">a</#outputformat>"#);
    assert!(matches!(t.root[0].kind, ElementKind::OutputFormat { .. }));
}
