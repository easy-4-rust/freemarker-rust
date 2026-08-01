//! 动态值类型 —— 模板表达式中的数值/日期/布尔表示
//! （对应 Java `Number`/`Boolean`/`Date` 家族；语义见 docs/06 §3）

use bigdecimal::BigDecimal;
use chrono::{DateTime, FixedOffset};
use num_bigint::BigInt;
use std::str::FromStr;

/// 日期类型（对应 Java `TemplateDateModel.TYPE_*`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateType {
    Date,
    Time,
    DateTime,
    /// 未知类型（对应 TemplateDateModel.UNKNOWN：?string.xs 等格式化须先 ?date/?time/?datetime）
    Unknown,
}

impl DateType {
    pub fn name(&self) -> &'static str {
        match self {
            DateType::Date => "date",
            DateType::Time => "time",
            DateType::DateTime => "date-time",
            DateType::Unknown => "unknown date type",
        }
    }
}

/// 日期值（对应 `java.util.Date` + 时区）
#[derive(Debug, Clone)]
pub struct DateValue {
    pub dt: DateTime<FixedOffset>,
    pub kind: DateType,
    /// 是否 `java.sql.Date`/`java.sql.Time` 风格值（对应 Java `isSQLDateOrTimeClass`）：
    /// SQL 值在 ISO/XS 格式中默认不显示时区偏移（zonelessInput=true，见
    /// ISOLikeTemplateDateFormat.formatToPlainText :178-191）
    pub is_sql: bool,
}

impl DateValue {
    /// 普通（非 SQL）日期值
    pub fn new(dt: DateTime<FixedOffset>, kind: DateType) -> DateValue {
        DateValue {
            dt,
            kind,
            is_sql: false,
        }
    }
}

/// 数值表示（对应 Java `Number` 层级）
/// 字面量映射：`1`→Int、`1L`→Long、`1F`→Float、`1D`→Double、`1.5`→Decimal、大整数→BigInt
#[derive(Debug, Clone)]
pub enum TNumber {
    Int(i32),
    Long(i64),
    BigInt(BigInt),
    Float(f32),
    Double(f64),
    Decimal(BigDecimal),
}

impl TNumber {
    pub fn from_i64(v: i64) -> TNumber {
        if let Ok(i) = i32::try_from(v) {
            TNumber::Int(i)
        } else {
            TNumber::Long(v)
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            TNumber::Int(_) | TNumber::Long(_) | TNumber::BigInt(_)
        )
    }

    /// 是否"整数表示"（BigDecimal 无小数部分也视为整数，用于 OptimizerUtil 降级）
    pub fn is_integer_value(&self) -> bool {
        match self {
            TNumber::Int(_) | TNumber::Long(_) | TNumber::BigInt(_) => true,
            TNumber::Decimal(d) => d.is_integer(),
            TNumber::Float(f) => f.fract() == 0.0 && f.is_finite(),
            TNumber::Double(f) => f.fract() == 0.0 && f.is_finite(),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            TNumber::Int(v) => Some(*v as f64),
            TNumber::Long(v) => Some(*v as f64),
            TNumber::BigInt(v) => v.to_string().parse().ok(),
            TNumber::Float(v) => Some(*v as f64),
            TNumber::Double(v) => Some(*v),
            TNumber::Decimal(v) => v.to_string().parse().ok(),
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|v| v as f32)
    }

    /// 转为 BigDecimal（算术引擎基础转换；对应 Java `new BigDecimal(number)` 语义）
    pub fn as_big_decimal(&self) -> BigDecimal {
        match self {
            TNumber::Int(v) => BigDecimal::from(*v),
            TNumber::Long(v) => BigDecimal::from(*v),
            TNumber::BigInt(v) => BigDecimal::from_bigint(v.clone(), 0),
            // Java ArithmeticEngine.toBigDecimal（:608-625）：Float/Double 用
            // **toString 最短往返表示**（`new BigDecimal(num.toString())`，"Why toString?
            // It's partly for backward compatibility. But it's also better for Double (and
            // Float) values than new BigDecimal(someDouble), which is overly precise"）
            // —— 0.05f → "0.05" 与 Decimal(0.05) 相等（比较/算术语义）。
            // 注意：数字**格式化**（format_decimal 快路径）另用加宽 double 的最短表示。
            TNumber::Float(v) => BigDecimal::from_str(&format!("{v}")).unwrap_or_default(),
            TNumber::Double(v) => BigDecimal::from_str(&format!("{v}")).unwrap_or_default(),
            TNumber::Decimal(v) => v.clone(),
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            TNumber::Int(v) => Some(*v as i64),
            TNumber::Long(v) => Some(*v),
            TNumber::BigInt(v) => i64::try_from(v.clone()).ok(),
            TNumber::Decimal(v) => v.to_string().parse().ok(),
            TNumber::Float(v) if v.fract() == 0.0 => Some(*v as i64),
            TNumber::Double(v) if v.fract() == 0.0 => Some(*v as i64),
            _ => None,
        }
    }

    /// 数值 → 字符串（canonical 形式；对应 Java `BigDecimal.toPlainString` 风格）
    /// 注意：这是内部表示，`?c`/`?string` 走 CFormat（docs/08 §2）
    pub fn to_plain_string(&self) -> String {
        match self {
            TNumber::Int(v) => v.to_string(),
            TNumber::Long(v) => v.to_string(),
            TNumber::BigInt(v) => v.to_string(),
            TNumber::Float(v) => format!("{v}"),
            TNumber::Double(v) => format!("{v}"),
            TNumber::Decimal(v) => v.to_string(),
        }
    }
}

impl PartialEq for TNumber {
    fn eq(&self, other: &Self) -> bool {
        // 数值相等比较（对应 Java `Number.equals` 语义近似；跨类型按数值比较）
        if self.is_integer() && other.is_integer() {
            return self.as_i64() == other.as_i64();
        }
        if self.is_integer_value() && other.is_integer_value() {
            return self.as_i64() == other.as_i64();
        }
        match (self.as_f64(), other.as_f64()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn int_long_decimal_equality() {
        assert_eq!(TNumber::Int(1), TNumber::Long(1));
        assert_eq!(TNumber::Int(1), TNumber::Decimal(BigDecimal::from(1)));
        assert_ne!(TNumber::Int(1), TNumber::Int(2));
    }

    #[test]
    fn big_decimal_roundtrip() {
        let d = BigDecimal::from_str("1.50").unwrap();
        let n = TNumber::Decimal(d);
        assert_eq!(n.as_big_decimal().to_string(), "1.50");
        assert!(!n.is_integer_value());
    }

    #[test]
    fn integer_forms_detected() {
        assert!(TNumber::Int(5).is_integer_value());
        assert!(TNumber::Decimal(BigDecimal::from(5)).is_integer_value());
        assert!(!TNumber::Decimal(BigDecimal::from_str("5.5").unwrap()).is_integer_value());
    }
}
