//! 序列内建测试 —— 自 built_ins_for_sequences.rs 拆出（#[cfg(test)] 模块；
//! 由主文件 #[cfg(test)] #[path] 声明，仅测试构建时编译）。

use super::built_ins_for_sequences_sort::{collator_cmp, utf16_cmp};
use super::*;
use crate::cache::StringLoader;
use crate::template::{Configuration, DynValue, ObjectWrapper, SimpleObjectWrapper};
use crate::value::DateValue;
use indexmap::IndexMap;
use std::cmp::Ordering;
use std::sync::Arc;

/// 渲染 `${src}` 返回输出字符串（boolean_format=c、number_format=0.#########，
/// 同 golden 用例设置）
fn eval_out(root: DynValue, src: &str) -> Result<String> {
    let mut c = Configuration::new();
    c.settings.boolean_format = "c".to_string();
    c.settings.number_format = "0.#########".to_string();
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();
    loader.put("t.ftl", &format!("${{{src}}}"));
    let t = c.get_template("t.ftl")?;
    let root_model = SimpleObjectWrapper
        .wrap(&root)?
        .unwrap_or_else(TModel::nothing);
    let mut out = Vec::new();
    t.process(root_model, &mut out)?;
    Ok(String::from_utf8(out).unwrap())
}

/// 直接以 TModel 为根渲染（测试纯集合/未知日期类型等 DynValue 无法表达的模型）
fn render_model(root_model: TModel, src: &str) -> Result<String> {
    let mut c = Configuration::new();
    c.settings.boolean_format = "c".to_string();
    c.settings.number_format = "0.#########".to_string();
    let loader = Arc::new(StringLoader::default());
    c.template_loader = loader.clone();
    loader.put("t.ftl", &format!("${{{src}}}"));
    let t = c.get_template("t.ftl")?;
    let mut out = Vec::new();
    t.process(root_model, &mut out)?;
    Ok(String::from_utf8(out).unwrap())
}

/// 错误消息去位置/指令栈后缀（渲染层附加 "  [in template ...]" 位置段与
/// "\n\n----\nFTL stack trace ..." 段——断言 Java 消息主体用）
fn err_msg(e: &TemplateError) -> String {
    e.to_string()
        .split("  [in template")
        .next()
        .unwrap_or_default()
        .split("\n\n----\nFTL stack trace")
        .next()
        .unwrap_or_default()
        .to_string()
}

fn no_root() -> DynValue {
    DynValue::Map(vec![])
}

/// 含 null 项的序列（Java TemplateTestCase 的 listWithNull）
fn list_with_null_root() -> DynValue {
    DynValue::Map(vec![(
        "listWithNull".into(),
        DynValue::List(vec![
            DynValue::Str("a".into()),
            DynValue::Null,
            DynValue::Str("c".into()),
        ]),
    )])
}

/// 1992-02-21 的日期模型（'yyyy-MM-dd'）
fn date_1992() -> DateValue {
    DateValue {
        dt: chrono::DateTime::parse_from_str("1992-02-21 00:00:00 +0000", "%Y-%m-%d %H:%M:%S %z")
            .unwrap(),
        kind: DateType::Date,
        is_sql: false,
    }
}

#[test]
fn utf16_order() {
    assert_eq!(utf16_cmp("a", "b"), Ordering::Less);
    assert_eq!(utf16_cmp("ab", "a"), Ordering::Greater);
}

#[test]
fn collator_order() {
    // jar 实测（Locale.US）顺序
    assert_eq!(collator_cmp("aardvark", "Barbara"), Ordering::Less);
    assert_eq!(collator_cmp("Barbara", "beetroot"), Ordering::Less);
    assert_eq!(collator_cmp("barbara", "Barbara"), Ordering::Less);
    assert_eq!(collator_cmp("Barbara", "BARBARA"), Ordering::Less);
    assert_eq!(collator_cmp("aA", "Aa"), Ordering::Less);
    assert_eq!(collator_cmp("a", "A"), Ordering::Less);
    assert_eq!(collator_cmp("a", "ab"), Ordering::Less);
    assert_eq!(collator_cmp("ab", "a"), Ordering::Greater);
}

#[test]
fn java_date_type_names() {
    assert_eq!(java_date_type_name(DateType::Date), "DATE");
    assert_eq!(java_date_type_name(DateType::Time), "TIME");
    assert_eq!(java_date_type_name(DateType::DateTime), "DATETIME");
    assert_eq!(java_date_type_name(DateType::Unknown), "UNKNOWN");
}

// ---- modelsEqual（Java SequenceBuiltins.modelsEqual :937-954）----

#[test]
fn models_equal_missing_and_mixed_types() {
    // null/缺失 → false（Java left/rightNullReturnsFalse）
    let nothing = TModel::nothing();
    let a = TModel::from_scalar("a".to_string());
    assert!(!models_equal(0, &nothing, &a, None).unwrap());
    assert!(!models_equal(0, &a, &nothing, None).unwrap());
    // 数字按值、字符串按内容、布尔相同
    assert!(models_equal(
        0,
        &TModel::from_number(TNumber::from_i64(1)),
        &TModel::from_number(TNumber::Decimal(bigdecimal::BigDecimal::from(1))),
        None
    )
    .unwrap());
    assert!(models_equal(
        0,
        &TModel::from_scalar("x".to_string()),
        &TModel::from_scalar("x".to_string()),
        None
    )
    .unwrap());
    assert!(models_equal(
        0,
        &TModel::from_boolean(true),
        &TModel::from_boolean(true),
        None
    )
    .unwrap());
    // 其余类型组合 → false（typeMismatchMeansNotEqual）
    assert!(!models_equal(
        0,
        &TModel::from_number(TNumber::from_i64(1)),
        &TModel::from_scalar("1".to_string()),
        None
    )
    .unwrap());
    assert!(!models_equal(
        0,
        &TModel::from_sequence(vec![]),
        &TModel::from_scalar("a".to_string()),
        None
    )
    .unwrap());
}

#[test]
fn models_equal_dates() {
    // 同型日期比毫秒
    let d1 = date_1992();
    let d2 = DateValue {
        dt: d1.dt,
        kind: DateType::Date,
        is_sql: false,
    };
    assert!(models_equal(
        0,
        &TModel::from_date(d1.clone()),
        &TModel::from_date(d2),
        None
    )
    .unwrap());
    // 异型 → "Can't compare dates of different types"（Java EvalUtil.compare :240-250）
    let dtm = DateValue {
        dt: d1.dt,
        kind: DateType::DateTime,
        is_sql: false,
    };
    let err = models_equal(3, &TModel::from_date(d1), &TModel::from_date(dtm), None).unwrap_err();
    assert_eq!(
            err.to_string(),
            "This error has occurred when comparing sequence item at 0-based index 3 to the searched item:\nCan't compare dates of different types. Left date type is DATE, right date type is DATETIME."
        );
}

#[test]
fn seq_index_of_date_mismatch_message() {
    // golden 断言（assertFails "dates of different types"）：日期 vs 日期时间比较报错
    let root = DynValue::Map(vec![(
        "x".into(),
        DynValue::List(vec![
            DynValue::Date(date_1992()),
            DynValue::Str("foo".into()),
        ]),
    )]);
    assert_eq!(
        eval_out(root.clone(), "x?seq_index_of('foo')").unwrap(),
        "1"
    );
    assert_eq!(
        eval_out(
            root.clone(),
            "x?seq_index_of('1992-02-21'?date('yyyy-MM-dd'))"
        )
        .unwrap(),
        "0"
    );
    let err = eval_out(
        root.clone(),
        "x?seq_index_of('1992-02-21 00:00:00'?datetime('yyyy-MM-dd HH:mm:ss'))",
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("dates of different types"),
        "{err}"
    );
}

#[test]
fn seq_index_of_missing_var_arg_flows_as_null() {
    // golden "These should throw exception, but for BC they don't"：
    // 缺失变量参数 → null → 不报错（Java MethodCall 参数求值返回 null）
    let root = list_with_null_root();
    assert_eq!(
        eval_out(root.clone(), "listWithNull?seq_contains(noSuchVar)?c").unwrap(),
        "false"
    );
    assert_eq!(
        eval_out(root.clone(), "listWithNull?seq_index_of(noSuchVar)").unwrap(),
        "-1"
    );
    assert_eq!(
        eval_out(root.clone(), "listWithNull?seq_last_index_of(noSuchVar)").unwrap(),
        "-1"
    );
    // null 项跳过（Java leftNullReturnsFalse）
    assert_eq!(
        eval_out(root.clone(), "listWithNull?seq_contains('c')?c").unwrap(),
        "true"
    );
    assert_eq!(
        eval_out(root.clone(), "listWithNull?seq_index_of('c')").unwrap(),
        "2"
    );
    assert_eq!(
        eval_out(root.clone(), "listWithNull?seq_last_index_of('a')").unwrap(),
        "0"
    );
}

#[test]
fn seq_index_of_from_index() {
    // 负数 fromIndex → 0；>= size → -1（Java findInSeq :477-492）
    let root = DynValue::Map(vec![(
        "names".into(),
        DynValue::List(vec![
            DynValue::Str("Joe".into()),
            DynValue::Str("Fred".into()),
            DynValue::Str("Joe".into()),
            DynValue::Str("Susan".into()),
        ]),
    )]);
    assert_eq!(
        eval_out(root.clone(), "names?seq_index_of('Joe', -2)").unwrap(),
        "0"
    );
    assert_eq!(
        eval_out(root.clone(), "names?seq_index_of('Joe', 1)").unwrap(),
        "2"
    );
    assert_eq!(
        eval_out(root.clone(), "names?seq_index_of('Joe', 4)").unwrap(),
        "-1"
    );
    // seq_last_index_of：fromIndex >= size → 从尾；< 0 → -1
    assert_eq!(
        eval_out(root.clone(), "names?seq_last_index_of('Joe', 1)").unwrap(),
        "0"
    );
    assert_eq!(
        eval_out(root.clone(), "names?seq_last_index_of('Joe', 4)").unwrap(),
        "2"
    );
    assert_eq!(
        eval_out(root.clone(), "names?seq_last_index_of('Susan', 2)").unwrap(),
        "-1"
    );
    // fromIndex 非整数 → intValue() 向零截断（jar 实测 2.5 → 2、-0.5 → 0）
    assert_eq!(
        eval_out(no_root(), "[1,2,3,4]?seq_index_of(4, 2.5)").unwrap(),
        "3"
    );
    assert_eq!(
        eval_out(no_root(), "[1,2,3,4]?seq_index_of(1, -0.5)").unwrap(),
        "0"
    );
    // fromIndex 缺失变量 → "expects a number as argument #2, but received a Null."
    let err = eval_out(no_root(), "[1,2,3]?seq_index_of(1, noSuchVar)").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "?seq_index_of(...) expects a number as argument #2, but received a Null."
    );
}

#[test]
fn seq_index_of_arg_count() {
    let err = eval_out(no_root(), "[1,2,3]?seq_index_of(1, 0, 0)").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "?seq_index_of(...) expects 1 or 2 arguments but has received 3."
    );
    let err = eval_out(no_root(), "[1,2,3]?seq_contains(1, 2)").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "?seq_contains(...) expects 1 argument but has received 2."
    );
}

#[test]
fn seq_builtins_on_collection() {
    // Java seq_index_ofBI.BIMethod :389-413：序列优先，否则集合迭代
    let root_model = TModel::from_hash(IndexMap::from([(
        "coll".to_string(),
        TModel::from_collection(vec![
            TModel::from_scalar("a".to_string()),
            TModel::from_scalar("b".to_string()),
            TModel::from_scalar("c".to_string()),
        ]),
    )]));
    assert_eq!(
        render_model(root_model.clone(), "coll?seq_index_of('b')").unwrap(),
        "1"
    );
    assert_eq!(
        render_model(root_model.clone(), "coll?seq_index_of('a', 1)").unwrap(),
        "-1"
    );
    assert_eq!(
        render_model(root_model.clone(), "coll?seq_last_index_of('a', 2)").unwrap(),
        "0"
    );
    assert_eq!(
        render_model(root_model.clone(), "coll?seq_contains('a')?c").unwrap(),
        "true"
    );
    // ?first 支持集合（Java firstBI.calculateResultForColletion :180-187）
    assert_eq!(render_model(root_model.clone(), "coll?first").unwrap(), "a");
}

#[test]
fn first_last_empty_return_null() {
    // Java firstBI/lastBI：空 → null → 下游 InvalidReferenceException
    let err = eval_out(no_root(), "[]?first").unwrap_err();
    assert!(
        err.to_string().contains("has evaluated to null or missing"),
        "{err}"
    );
    let err = eval_out(no_root(), "[]?last").unwrap_err();
    assert!(
        err.to_string().contains("has evaluated to null or missing"),
        "{err}"
    );
}

#[test]
fn chunk_semantics() {
    // 非整数 size 截断（Java intValue()；jar 实测 2.9 → 2 块）
    assert_eq!(
        eval_out(no_root(), "[1,2,3,4]?chunk(2.9)?size").unwrap(),
        "2"
    );
    // 截断后 < 1 → "must be at least 1."
    let err = eval_out(no_root(), "[1,2,3]?chunk(0.5)?size").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "The 1st argument to ?chunk (...) must be at least 1."
    );
    // 非数字 size → 参数类型错误（Java newMethodArgMustBeNumberException）
    let err = eval_out(no_root(), "[1,2,3]?chunk('x')?size").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "?chunk(...) expects a number as argument #1, but received a string."
    );
    // size 缺失变量 → "received a Null."（jar 实测）
    let err = eval_out(no_root(), "[1,2,3]?chunk(noSuchVar)?size").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "?chunk(...) expects a number as argument #1, but received a Null."
    );
    // filler 缺失变量 → null → 不补齐（Java :78）
    assert_eq!(
        eval_out(no_root(), "[1,2]?chunk(1, noSuchVar)?size").unwrap(),
        "2"
    );
    // filler 补齐
    let root = DynValue::Map(vec![(
        "rows".into(),
        DynValue::List(vec![
            DynValue::List(vec![DynValue::Int(1)]),
            DynValue::List(vec![DynValue::Int(2), DynValue::Str("-".into())]),
        ]),
    )]);
    let _ = root;
    assert_eq!(
        eval_out(no_root(), "([1,2,3]?chunk(2, '-')?first)?size").unwrap(),
        "2"
    );
    assert_eq!(
        eval_out(no_root(), "([1,2,3]?chunk(2, '-')?last)?size").unwrap(),
        "2"
    );
    assert_eq!(
        eval_out(no_root(), "([1,2,3]?chunk(2)?last)?size").unwrap(),
        "1"
    );
}

#[test]
fn sort_by_messages_match_java() {
    let err = eval_out(no_root(), "[{'a':1}]?sort_by()").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "?sort_by(...) expects 1 argument but has received none."
    );
    let err = eval_out(no_root(), "[{'a':1}]?sort_by(42)").unwrap_err();
    assert_eq!(
            err_msg(&err),
            "The argument to ?sort_by(key) must be a string (the name of the subvariable), or a sequence of strings (the \"path\" to the subvariable)."
        );
    let err = eval_out(no_root(), "[{'a':1}]?sort_by([1, 2])").unwrap_err();
    assert_eq!(
            err_msg(&err),
            "The argument to ?sort_by(key), when it's a sequence, must be a sequence of strings, but the item at index 0 is not a string."
        );
    let err = eval_out(no_root(), "[{'a':1}]?sort_by('b')").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "?sort_by(...) failed at sequence index 0: The \"b\" subvariable was null or missing."
    );
    let err = eval_out(no_root(), "[1,2]?sort_by('a')").unwrap_err();
    assert_eq!(
            err_msg(&err),
            "?sort_by(...) failed at sequence index 0: Sequence items must be hashes when using ?sort_by.  subvariable is not a hash, so ?sort_by can't proceed with getting the \"a\" subvariable."
        );
    // 键类型不一致（Java newInconsistentSortKeyTypeException :670-688）
    let err = eval_out(no_root(), "[1, 'a']?sort").unwrap_err();
    assert_eq!(
            err_msg(&err),
            "?sort failed at sequence index 1 (0-based): All values in the sequence must be numbers, because the first value was that. However, the value of the current item isn't a number but a string."
        );
    let err = eval_out(no_root(), "[{'a':'x'},{'a':1}]?sort_by('a')").unwrap_err();
    assert_eq!(
            err_msg(&err),
            "?sort_by(...) failed at sequence index 1 (0-based): All key values in the sequence must be strings, because the first key value was that. However, the key value of the current item isn't a string but a number."
        );
}

#[test]
fn sort_orders() {
    // 字符串序（en_US Collator 近似：忽略大小写的主强度，小写 < 大写的第三强度；
    // 与 golden 用例及 jar 实测一致）
    assert_eq!(
        eval_out(
            no_root(),
            "(['whale','Barbara','zeppelin','aardvark','beetroot']?sort)?join(',')"
        )
        .unwrap(),
        "aardvark,Barbara,beetroot,whale,zeppelin"
    );
    assert_eq!(
        eval_out(no_root(), "(['a','A','aa','aA','Aa','AA']?sort)?join(',')").unwrap(),
        "a,A,aa,aA,Aa,AA"
    );
    assert_eq!(
        eval_out(
            no_root(),
            "(['Barbara','barbara','BARBARA']?sort)?join(',')"
        )
        .unwrap(),
        "barbara,Barbara,BARBARA"
    );
    // 数字序（跨数值类型）
    assert_eq!(
        eval_out(
            no_root(),
            "[123?byte, 543, -324, -34?float, 0.11, 0, 111?int, 0.1?double, 1, 5]?sort?join(',')"
        )
        .unwrap(),
        "-324,-34,0,0.1,0.11,1,5,111,123,543"
    );
    // 布尔序（false < true，Java BooleanKVPComparator）
    assert_eq!(
        eval_out(no_root(), "([true,false,false,true]?sort)?first?c").unwrap(),
        "false"
    );
    // 日期序（DateKVPComparator 按毫秒）
    assert_eq!(
            eval_out(
                no_root(),
                "(['1999-01-20'?date('yyyy-MM-dd'), '1998-02-20'?date('yyyy-MM-dd')]?sort)?first?string('yyyy-MM-dd')"
            )
            .unwrap(),
            "1998-02-20"
        );
    // 空序列 → 原模型（?size == 0）
    assert_eq!(eval_out(no_root(), "([]?sort)?size").unwrap(), "0");
}

#[test]
fn min_max_semantics() {
    assert_eq!(eval_out(no_root(), "[3,1,2]?max").unwrap(), "3");
    assert_eq!(eval_out(no_root(), "[3,1,2]?min").unwrap(), "1");
    // 空 → null → 下游 InvalidReferenceException（Java MinOrMaxBI :999-1011）
    let err = eval_out(no_root(), "[]?max").unwrap_err();
    assert!(
        err.to_string().contains("has evaluated to null or missing"),
        "{err}"
    );
    // null 元素跳过（jar 实测：跳过 null 后比较 'a' vs 'c' 字符串 → 报错）
    let err = eval_out(list_with_null_root(), "listWithNull?max").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "Can't use operator \"greater-than\" on string values."
    );
    // 字符串：大小比较报错（Java EvalUtil.compare :262-266，operatorString null
    // → "greater-than"/"less-than"）
    let err = eval_out(no_root(), "['a','b']?max").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "Can't use operator \"greater-than\" on string values."
    );
    let err = eval_out(no_root(), "['a','b']?min").unwrap_err();
    assert_eq!(
        err_msg(&err),
        "Can't use operator \"less-than\" on string values."
    );
    // 类型不匹配（Java :307-326，typeMismatchMeansNotEqual=false → 报错）
    let err = eval_out(no_root(), "[1,'a']?max").unwrap_err();
    assert!(
        err.to_string()
            .contains("Can't compare values of these types"),
        "{err}"
    );
    // 日期异型（Java :240-250，大写类型名）
    let err = eval_out(
            no_root(),
            "['1992-02-21'?date('yyyy-MM-dd'), '1992-02-21 00:00:00'?datetime('yyyy-MM-dd HH:mm:ss')]?max",
        )
        .unwrap_err();
    assert!(
        err.to_string()
            .contains("Left date type is DATETIME, right date type is DATE"),
        "{err}"
    );
    // 右无界范围拒绝（Java :975 checkNotRightUnboundedNumericalRange）
    let err = eval_out(no_root(), "(1..)?max").unwrap_err();
    assert!(err.to_string().contains("right-unbounded"), "{err}");
}

#[test]
fn min_max_on_collection() {
    let root_model = TModel::from_hash(IndexMap::from([(
        "coll".to_string(),
        TModel::from_collection(vec![
            TModel::from_number(TNumber::from_i64(3)),
            TModel::from_number(TNumber::from_i64(1)),
            TModel::from_number(TNumber::from_i64(2)),
        ]),
    )]));
    assert_eq!(render_model(root_model.clone(), "coll?max").unwrap(), "3");
    assert_eq!(render_model(root_model.clone(), "coll?min").unwrap(), "1");
}

#[test]
fn seq_contains_unknown_date_errors() {
    // Java EvalUtil.compare :227-238：未知日期类型比较报错
    let unknown = TModel::from_date(DateValue {
        dt: date_1992().dt,
        kind: DateType::Unknown,
        is_sql: false,
    });
    let root_model = TModel::from_hash(IndexMap::from([("u".to_string(), unknown)]));
    let err = render_model(root_model, "[u]?seq_contains(u)?c").unwrap_err();
    assert!(
        err.to_string()
            .contains("value of the comparison is a date-like value where it's not known"),
        "{err}"
    );
}

// ---- ?sequence ----

#[test]
fn sequence_on_sequence() {
    // 已是序列 → 原样返回
    assert_eq!(eval_out(no_root(), "[1,2,3]?sequence?size").unwrap(), "3");
    assert_eq!(eval_out(no_root(), "([1,2]?sequence)?first").unwrap(), "1");
}

#[test]
fn sequence_on_string() {
    // 字符串 → 字符序列
    assert_eq!(eval_out(no_root(), "'abc'?sequence?size").unwrap(), "3");
    assert_eq!(
        eval_out(no_root(), "('abc'?sequence)?join(',')").unwrap(),
        "a,b,c"
    );
    assert_eq!(eval_out(no_root(), "('abc'?sequence)?first").unwrap(), "a");
    assert_eq!(eval_out(no_root(), "('abc'?sequence)?last").unwrap(), "c");
}

#[test]
fn sequence_on_collection() {
    // 集合 → 原样返回
    let root_model = TModel::from_hash(IndexMap::from([(
        "coll".to_string(),
        TModel::from_collection(vec![
            TModel::from_scalar("x".to_string()),
            TModel::from_scalar("y".to_string()),
            TModel::from_scalar("z".to_string()),
        ]),
    )]));
    // ?seq_contains 适用于序列和集合
    assert_eq!(
        render_model(root_model.clone(), "coll?sequence?seq_contains('y')?c").unwrap(),
        "true"
    );
    assert_eq!(
        render_model(root_model.clone(), "coll?sequence?seq_contains('w')?c").unwrap(),
        "false"
    );
}

#[test]
fn sequence_on_number_errors() {
    // 数字 → 报错
    let err = eval_out(no_root(), "42?sequence").unwrap_err();
    assert!(
        err.to_string()
            .contains("?sequence is not applicable to a number value"),
        "{err}"
    );
}

#[test]
fn sequence_on_empty_string() {
    // 空字符串 → 空字符序列
    assert_eq!(eval_out(no_root(), "''?sequence?size").unwrap(), "0");
}
