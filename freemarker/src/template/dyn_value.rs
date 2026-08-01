//! 动态值 —— Rust 侧桥接用户输入/输出的运行时值
//! 无 Java 对应类：Java 的 `ObjectWrapper.wrap/unwrap` 操作任意 Java 对象（有统一根类型
//! `java.lang.Object`）；Rust 无统一对象根，故以本枚举作为模板数据入口（docs/06 §4.1）。
//! 注意：本类型只承载模板可直接映射的标量/容器，结构体/枚举等需用户侧自行转成这些变体。

use crate::value::DateValue;

/// Rust 侧动态值（对应 Java 侧普通对象：String/Number/Boolean/Map/List/null/Date）
/// - `Int(i64)`：Java `Integer`/`Long`/`BigInteger` 的 Rust 侧统一表示
/// - `Float(f64)`：Java `Float`/`Double`（含 BigDecimal 非整数近似）的 Rust 侧统一表示
/// - `Map(Vec<(String, DynValue)>)`：Java `Map<String, Object>`；用 Vec 保留键顺序
/// - `List(Vec<DynValue>)`：Java `List<Object>` / 数组
/// - `Null`：Java `null`
/// - `Date(DateValue)`：Java `java.util.Date` + 类型标记（date/time/date-time）
#[derive(Debug, Clone)]
pub enum DynValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Map(Vec<(String, DynValue)>),
    List(Vec<DynValue>),
    Null,
    Date(DateValue),
}

impl DynValue {
    /// 对应 Java `obj == null` 判定
    pub fn is_null(&self) -> bool {
        matches!(self, DynValue::Null)
    }
}

impl PartialEq for DynValue {
    /// 值相等（Date 比较 dt + kind；Float 遵循 f64 语义，NaN != NaN）
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (DynValue::Str(a), DynValue::Str(b)) => a == b,
            (DynValue::Int(a), DynValue::Int(b)) => a == b,
            (DynValue::Float(a), DynValue::Float(b)) => a == b,
            (DynValue::Bool(a), DynValue::Bool(b)) => a == b,
            (DynValue::Map(a), DynValue::Map(b)) => a == b,
            (DynValue::List(a), DynValue::List(b)) => a == b,
            (DynValue::Null, DynValue::Null) => true,
            (DynValue::Date(a), DynValue::Date(b)) => a.dt == b.dt && a.kind == b.kind,
            _ => false,
        }
    }
}
