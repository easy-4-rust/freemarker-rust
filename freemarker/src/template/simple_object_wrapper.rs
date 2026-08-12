//! 简单对象包装 —— 对应 Java `freemarker.template.SimpleObjectWrapper`
//! （Java SimpleObjectWrapper.java：只包装 SimpleXxx 家族；未知类型抛异常
//!   handleUnknownType :54-58；禁止 ?api :60-63 —— Rust 版无 ?api 概念）
//!
//! 映射表（与 Java 一致，见 docs/06 §4.2）：
//! | DynValue     | TModel                            |
//! |--------------|-----------------------------------|
//! | Str          | SimpleScalar                      |
//! | Int          | TNumber::Int（i32 范围）/ Long    |
//! | Float        | TNumber::Double                   |
//! | Bool         | SimpleBoolean                     |
//! | List         | SimpleSequence（元素递归 wrap）    |
//! | Map          | SimpleHash（递归 wrap）           |
//! | Date         | SimpleDate                        |
//! | Null         | None（Java wrap(null) 返回 null）  |

use crate::error::{Result, TemplateError};
use crate::template::{DynValue, ObjectWrapper, TModel};
use crate::value::TNumber;
use indexmap::IndexMap;

/// 对应 Java `SimpleObjectWrapper.instance`（单例）
pub const SIMPLE_WRAPPER: SimpleObjectWrapper = SimpleObjectWrapper;

/// 简单对象包装器（对应 Java `SimpleObjectWrapper`，无状态单例）
pub struct SimpleObjectWrapper;

impl ObjectWrapper for SimpleObjectWrapper {
    fn wrap(&self, obj: &DynValue) -> Result<Option<TModel>> {
        match obj {
            // Java wrap(null) 返回 null（ObjectWrapper.java:85-87）
            DynValue::Null => Ok(None),
            DynValue::Str(s) => Ok(Some(TModel::from_scalar(s.clone()))),
            DynValue::Int(v) => {
                // Java：Integer 直接包装；超出 int 范围（如 Long/BigInteger）→ SimpleNumber(Long)
                let n = if let Ok(i) = i32::try_from(*v) {
                    TNumber::Int(i)
                } else {
                    TNumber::Long(*v)
                };
                Ok(Some(TModel::from_number(n)))
            }
            // Java：Float/Double → SimpleNumber（Rust 侧统一为 Double，docs/06 §4.2）
            DynValue::Float(f) => Ok(Some(TModel::from_number(TNumber::Double(*f)))),
            DynValue::Bool(b) => Ok(Some(TModel::from_boolean(*b))),
            DynValue::Date(d) => Ok(Some(TModel::from_date(d.clone()))),
            DynValue::Map(pairs) => {
                let mut map = IndexMap::with_capacity(pairs.len());
                for (k, v) in pairs {
                    // Java SimpleHash 允许 null 值（等价模板侧 missing）；
                    // Rust 用 TModel::nothing() 表达 null（docs/06 §2）
                    let m = self.wrap(v)?.unwrap_or_else(TModel::nothing);
                    map.insert(k.clone(), m);
                }
                Ok(Some(TModel::from_hash(map)))
            }
            DynValue::List(items) => {
                let mut models = Vec::with_capacity(items.len());
                for item in items {
                    models.push(self.wrap(item)?.unwrap_or_else(TModel::nothing));
                }
                Ok(Some(TModel::from_sequence(models)))
            }
        }
    }

    fn unwrap(&self, model: &TModel) -> Result<DynValue> {
        // 对应 Java DeepUnwrap.unwrap（DeepUnwrap.java:100-176）的递归规则；
        // 判定顺序 scalar → number → boolean → date → hash → sequence/collection → nothing
        if model.is_nothing() {
            // 对应 null 模型（Java：model == nullModel → null，DeepUnwrap.java:119-121）
            return Ok(DynValue::Null);
        }
        if let Some(s) = &model.scalar {
            return Ok(DynValue::Str(s.as_string()?));
        }
        if let Some(n) = &model.number {
            return unwrap_number(&n.as_number()?);
        }
        if let Some(b) = &model.boolean {
            return Ok(DynValue::Bool(b.as_boolean()?));
        }
        if let Some(d) = &model.date {
            return Ok(DynValue::Date(d.as_date()?));
        }
        if let Some(h) = &model.hash_ex {
            // 对应 TemplateHashModelEx 分支（DeepUnwrap.java:152-170）：keys() 顺序展开
            // 为 Map。Java 侧是 LinkedHashMap（插入序）；Rust 侧 SimpleHash 为 HashMap
            // 无插入序 → 按键排序保证展开结果确定性
            let mut keys = h.keys()?;
            keys.sort();
            let mut pairs = Vec::with_capacity(keys.len());
            for key in keys {
                let value = match h.get(&key)? {
                    Some(m) => self.unwrap(&m)?,
                    None => DynValue::Null,
                };
                pairs.push((key, value));
            }
            return Ok(DynValue::Map(pairs));
        }
        if let Some(seq) = &model.sequence {
            // 对应 TemplateSequenceModel 分支（DeepUnwrap.java:134-142）：size 次 get + 递归
            let size = seq.size()?;
            let mut items = Vec::with_capacity(size);
            for i in 0..size {
                items.push(self.unwrap(&seq.get(i)?)?);
            }
            return Ok(DynValue::List(items));
        }
        if let Some(coll) = &model.collection {
            // 对应 TemplateCollectionModel 分支（DeepUnwrap.java:143-151）：迭代器消费
            let mut items = Vec::new();
            for item in coll.iterator()? {
                items.push(self.unwrap(&item?)?);
            }
            return Ok(DynValue::List(items));
        }
        // 对应 DeepUnwrap.java:175：抛 TemplateModelException
        Err(TemplateError::misc(format!(
            "Cannot deep-unwrap model of type {}",
            model.type_name
        )))
    }
}

/// 数值展开：整数 → Int(i64)，非整数 → Float(f64)
/// （对应 DeepUnwrap 的 number → Number 原样返回；Rust 侧 DynValue 只有 Int/Float 两个
///   数值变体，故按"值是否为整数"分流；i64 范围外的整数无法精确表示 → 报错）
fn unwrap_number(n: &TNumber) -> Result<DynValue> {
    match n {
        TNumber::Int(v) => Ok(DynValue::Int(*v as i64)),
        TNumber::Long(v) => Ok(DynValue::Int(*v)),
        TNumber::BigInt(v) => match i64::try_from(v.clone()) {
            Ok(i) => Ok(DynValue::Int(i)),
            Err(_) => Err(TemplateError::misc(format!(
                "Cannot deep-unwrap integer {}: out of i64 range",
                v
            ))),
        },
        TNumber::Decimal(d) => {
            if d.is_integer() {
                // 整数 Decimal → 精确 BigInt（with_scale(0) 取整，is_integer 时无精度损失；
                // bigdecimal 未重导出 num_traits::ToBigInt，故用 unscaled 路径）
                let bi = d.with_scale(0).as_bigint_and_scale().0.into_owned();
                match i64::try_from(bi) {
                    Ok(i) => Ok(DynValue::Int(i)),
                    Err(_) => Err(TemplateError::misc(format!(
                        "Cannot deep-unwrap integer {}: out of i64 range",
                        d
                    ))),
                }
            } else {
                Ok(DynValue::Float(decimal_to_f64(d)?))
            }
        }
        TNumber::Float(v) => integer_float_to_dyn(*v as f64, *v as i64),
        TNumber::Double(v) => integer_float_to_dyn(*v, *v as i64),
    }
}

/// Float/Double 整数且 i64 可表示 → Int；否则 → Float
fn integer_float_to_dyn(f: f64, truncated: i64) -> Result<DynValue> {
    if f.is_finite() && f.fract() == 0.0 && truncated as f64 == f {
        Ok(DynValue::Int(truncated))
    } else {
        Ok(DynValue::Float(f))
    }
}

fn decimal_to_f64(d: &bigdecimal::BigDecimal) -> Result<f64> {
    d.to_string()
        .parse::<f64>()
        .map_err(|e| TemplateError::misc(format!("Cannot convert {d} to double: {e}")))
}

/// 构造测试用的日期值（date/time/date-time 由 DateType 标记）
#[cfg(test)]
fn make_date(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    min: u32,
    sec: u32,
    kind: crate::value::DateType,
) -> crate::value::DateValue {
    use chrono::{FixedOffset, TimeZone};
    let dt = FixedOffset::east_opt(0)
        .unwrap()
        .with_ymd_and_hms(year, month, day, hour, min, sec)
        .single()
        .expect("valid date");
    crate::value::DateValue::new(dt, kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Environment;
    use crate::value::DateType;

    fn wrapper() -> SimpleObjectWrapper {
        SimpleObjectWrapper
    }

    // ------------------------------------------------------------------
    // wrap：标量映射
    // ------------------------------------------------------------------
    #[test]
    fn wrap_null_is_none() {
        assert!(wrapper().wrap(&DynValue::Null).unwrap().is_none());
    }

    #[test]
    fn wrap_scalars() {
        let w = wrapper();
        // Str → scalar 槽
        let m = w.wrap(&DynValue::Str("hello".into())).unwrap().unwrap();
        assert!(m.is_scalar() && !m.is_number() && !m.is_boolean());
        assert_eq!(m.get_scalar().unwrap(), "hello");
        assert_eq!(m.type_name, "string");

        // Bool
        let m = w.wrap(&DynValue::Bool(true)).unwrap().unwrap();
        assert!(m.is_boolean());
        assert!(m.get_boolean().unwrap());

        // Date → date 槽 + kind 保留
        let d = make_date(2024, 1, 2, 3, 4, 5, DateType::DateTime);
        let m = w.wrap(&DynValue::Date(d.clone())).unwrap().unwrap();
        assert!(m.is_date());
        assert_eq!(m.get_date().unwrap().dt, d.dt);
        assert_eq!(m.get_date().unwrap().kind, DateType::DateTime);
    }

    #[test]
    fn wrap_int_i32_boundary() {
        let w = wrapper();
        // 2147483647 在 i32 范围 → Int
        let m = w.wrap(&DynValue::Int(2147483647)).unwrap().unwrap();
        match m.get_number().unwrap() {
            TNumber::Int(v) => assert_eq!(v, i32::MAX),
            other => panic!("expected Int(i32::MAX), got {other:?}"),
        }
        // 2147483648 超 i32 → Long
        let m = w.wrap(&DynValue::Int(2147483648)).unwrap().unwrap();
        match m.get_number().unwrap() {
            TNumber::Long(v) => assert_eq!(v, 2147483648),
            other => panic!("expected Long(2147483648), got {other:?}"),
        }
        // 负边界
        let m = w.wrap(&DynValue::Int(-2147483648)).unwrap().unwrap();
        match m.get_number().unwrap() {
            TNumber::Int(v) => assert_eq!(v, i32::MIN),
            other => panic!("expected Int(i32::MIN), got {other:?}"),
        }
    }

    #[test]
    fn wrap_float_becomes_double() {
        let m = wrapper().wrap(&DynValue::Float(1.5)).unwrap().unwrap();
        match m.get_number().unwrap() {
            TNumber::Double(v) => assert_eq!(v, 1.5),
            other => panic!("expected Double(1.5), got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // wrap：嵌套容器递归
    // ------------------------------------------------------------------
    #[test]
    fn wrap_nested_map_and_list() {
        let w = wrapper();
        let value = DynValue::Map(vec![
            ("name".into(), DynValue::Str("alice".into())),
            ("age".into(), DynValue::Int(30)),
            (
                "tags".into(),
                DynValue::List(vec![
                    DynValue::Str("a".into()),
                    DynValue::Int(1),
                    DynValue::Null,
                ]),
            ),
        ]);
        let m = w.wrap(&value).unwrap().unwrap();
        assert!(m.is_hash());
        assert!(m.is_hash_ex());
        // hash 深层取值
        let hash = m.get_hash().unwrap();
        let tags = hash.get("tags").unwrap().unwrap();
        assert!(tags.is_sequence());
        assert_eq!(tags.get_sequence().unwrap().size().unwrap(), 3);
        let tags_seq = tags.get_sequence().unwrap();
        assert_eq!(tags_seq.get(0).unwrap().get_scalar().unwrap(), "a");
        assert_eq!(
            tags_seq.get(1).unwrap().get_number().unwrap().as_i64(),
            Some(1)
        );
        // Null 元素 → nothing
        assert!(tags_seq.get(2).unwrap().is_nothing());
        // 顶层 List
        let m = w
            .wrap(&DynValue::List(vec![
                DynValue::Int(1),
                DynValue::Str("x".into()),
            ]))
            .unwrap()
            .unwrap();
        assert!(m.is_sequence());
        assert_eq!(m.get_sequence().unwrap().size().unwrap(), 2);
    }

    #[test]
    fn wrap_map_null_value_becomes_nothing() {
        let m = wrapper()
            .wrap(&DynValue::Map(vec![("k".into(), DynValue::Null)]))
            .unwrap()
            .unwrap();
        let v = m.get_hash().unwrap().get("k").unwrap().unwrap();
        assert!(v.is_nothing(), "null 值应包装为 nothing");
    }

    // ------------------------------------------------------------------
    // unwrap：类型映射
    // ------------------------------------------------------------------
    #[test]
    fn unwrap_scalar_number_boolean_date() {
        let w = wrapper();
        assert_eq!(
            w.unwrap(&TModel::from_scalar("hi".into())).unwrap(),
            DynValue::Str("hi".into())
        );
        assert_eq!(
            w.unwrap(&TModel::from_boolean(false)).unwrap(),
            DynValue::Bool(false)
        );
        // 整数 → Int
        assert_eq!(
            w.unwrap(&TModel::from_number(TNumber::Int(7))).unwrap(),
            DynValue::Int(7)
        );
        // 非整数 → Float
        assert_eq!(
            w.unwrap(&TModel::from_number(TNumber::Decimal(
                "1.5".parse().unwrap()
            )))
            .unwrap(),
            DynValue::Float(1.5)
        );
        // Long → Int（i64 承载）
        assert_eq!(
            w.unwrap(&TModel::from_number(TNumber::Long(2147483648)))
                .unwrap(),
            DynValue::Int(2147483648)
        );
        // Date → Date（kind 保留）
        let d = make_date(2024, 5, 6, 7, 8, 9, DateType::Date);
        let got = w.unwrap(&TModel::from_date(d.clone())).unwrap();
        assert_eq!(got, DynValue::Date(d));
    }

    #[test]
    fn unwrap_nothing_is_null() {
        assert_eq!(
            wrapper().unwrap(&TModel::nothing()).unwrap(),
            DynValue::Null
        );
    }

    #[test]
    fn unwrap_float_integer_value_becomes_int() {
        // Float(3.0) 是整数表示 → Int(3)
        assert_eq!(
            wrapper()
                .unwrap(&TModel::from_number(TNumber::Float(3.0)))
                .unwrap(),
            DynValue::Int(3)
        );
        // 非整数浮点 → Float
        assert_eq!(
            wrapper()
                .unwrap(&TModel::from_number(TNumber::Float(2.5)))
                .unwrap(),
            DynValue::Float(2.5)
        );
        // Double 整数（i64 范围内）→ Int
        assert_eq!(
            wrapper()
                .unwrap(&TModel::from_number(TNumber::Double(1e10)))
                .unwrap(),
            DynValue::Int(10000000000)
        );
        // Double 整数但超出 i64 → Float（保留可表示性）
        assert_eq!(
            wrapper()
                .unwrap(&TModel::from_number(TNumber::Double(1e30)))
                .unwrap(),
            DynValue::Float(1e30)
        );
    }

    #[test]
    fn unwrap_unsupported_model_type_errors() {
        // method / directive 无对应展开规则 → 报错并描述类型（DeepUnwrap.java:175）
        struct DummyMethod;
        impl crate::template::TemplateMethodModelEx for DummyMethod {
            fn exec(&self, _env: &mut Environment, _args: Vec<TModel>) -> Result<TModel> {
                // 测试中不会被调用（unwrap 在取用 method 槽之前就报错）
                Ok(TModel::nothing())
            }
        }
        let m = TModel::from_method(DummyMethod);
        match wrapper().unwrap(&m) {
            Err(TemplateError::Misc { message }) => {
                assert!(message.contains("method"), "{message}");
                assert!(message.contains("Cannot deep-unwrap"), "{message}");
            }
            other => panic!("expected Misc error, got {other:?}"),
        }
    }

    #[test]
    fn unwrap_bigint_out_of_i64_errors() {
        use num_bigint::BigInt;
        use std::str::FromStr;
        let m = TModel::from_number(TNumber::BigInt(
            BigInt::from_str("12345678901234567890123").unwrap(),
        ));
        assert!(wrapper().unwrap(&m).is_err());
    }

    // ------------------------------------------------------------------
    // wrap/unwrap 往返等价（DeepUnwrap 递归一致性，docs/06 §6.4）
    // ------------------------------------------------------------------
    #[test]
    fn roundtrip_nested_values() {
        let w = wrapper();
        // Map 键按排序展开 → 输入也用排序键保证往返一致
        let value = DynValue::Map(vec![
            ("age".into(), DynValue::Int(30)),
            ("name".into(), DynValue::Str("alice".into())),
            (
                "profile".into(),
                DynValue::Map(vec![
                    ("city".into(), DynValue::Str("beijing".into())),
                    // 注意：unwrap 展开哈希按键排序（SimpleHash 为 HashMap 无插入序），
                    // 输入键须已排序（city < member < score）才能往返一致
                    ("member".into(), DynValue::Bool(true)),
                    ("score".into(), DynValue::Float(98.5)),
                ]),
            ),
            (
                "tags".into(),
                DynValue::List(vec![
                    DynValue::Str("a".into()),
                    DynValue::Int(1),
                    DynValue::Null,
                    DynValue::List(vec![DynValue::Int(2), DynValue::Int(3)]),
                ]),
            ),
        ]);
        let model = w.wrap(&value).unwrap().unwrap();
        assert_eq!(w.unwrap(&model).unwrap(), value);
    }

    #[test]
    fn roundtrip_date_and_null() {
        let w = wrapper();
        let d = make_date(2023, 12, 31, 23, 59, 58, DateType::DateTime);
        let model = w.wrap(&DynValue::Date(d.clone())).unwrap().unwrap();
        assert_eq!(w.unwrap(&model).unwrap(), DynValue::Date(d));
        assert_eq!(w.unwrap(&TModel::nothing()).unwrap(), DynValue::Null);
    }

    #[test]
    fn roundtrip_scalar_values() {
        let w = wrapper();
        for v in [
            DynValue::Str("x".into()),
            DynValue::Int(42),
            DynValue::Int(-7),
            DynValue::Float(2.5),
            DynValue::Bool(true),
        ] {
            let model = w.wrap(&v).unwrap().unwrap();
            assert_eq!(w.unwrap(&model).unwrap(), v);
        }
    }
}
