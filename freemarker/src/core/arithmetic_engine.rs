//! 算术引擎 —— 对应 Java `freemarker.core.ArithmeticEngine`
//! （BigDecimalEngine 语义逐行对照 freemarker-core/.../ArithmeticEngine.java；
//!   设计文档见 docs/06 §3）

use crate::error::{Result, TemplateError};
use crate::value::TNumber;
use bigdecimal::BigDecimal;
use bigdecimal::RoundingMode;
// Zero/Signed 由 bigdecimal 重导出（num_traits）：BigDecimal/BigInt 的 is_zero/abs
use bigdecimal::{Signed, Zero};
use num_bigint::{BigInt, Sign};
use std::str::FromStr;

/// Java `ArithmeticEngine.minScale` 默认值（ArithmeticEngine.java:68，除法最小 scale）
const DEFAULT_MIN_SCALE: i64 = 12;
/// Java `ArithmeticEngine.maxScale` 默认值（ArithmeticEngine.java:69，乘法结果最大 scale）
const DEFAULT_MAX_SCALE: i64 = 12;

/// 算术引擎 trait（对应 Java `ArithmeticEngine` 抽象类）
pub trait ArithmeticEngine {
    fn add(&self, a: &TNumber, b: &TNumber) -> Result<TNumber>;
    fn sub(&self, a: &TNumber, b: &TNumber) -> Result<TNumber>;
    fn mul(&self, a: &TNumber, b: &TNumber) -> Result<TNumber>;
    fn div(&self, a: &TNumber, b: &TNumber) -> Result<TNumber>;
    fn mod_op(&self, a: &TNumber, b: &TNumber) -> Result<TNumber>;
    fn negate(&self, a: &TNumber) -> Result<TNumber>;
}

/// 默认引擎（对应 Java `ArithmeticEngine.BigDecimalEngine`，2.3.34 默认）
/// 语义：所有操作数先转 BigDecimal（Java toBigDecimal），运算结果仍为 BigDecimal，
/// 再按 `OptimizerUtil.optimizeNumberRepresentation` 优化表示（docs/06 §3）。
///
/// 配置项对应 Java 的 `setMinScale`/`setMaxScale`（roundingPolicy 固定为 ROUND_HALF_UP 默认值）。
pub struct BigDecimalEngine {
    /// 除法最小 scale（ArithmeticEngine.java:68 `minScale`，默认 12）
    pub min_scale: i64,
    /// 乘法结果最大 scale（ArithmeticEngine.java:69 `maxScale`，默认 12）
    pub max_scale: i64,
}

impl Default for BigDecimalEngine {
    fn default() -> Self {
        BigDecimalEngine {
            min_scale: DEFAULT_MIN_SCALE,
            max_scale: DEFAULT_MAX_SCALE,
        }
    }
}

impl BigDecimalEngine {
    /// 对应 Java `ArithmeticEngine.toBigDecimal`（ArithmeticEngine.java:608-632）
    /// - Integer/Long → `BigDecimal.valueOf(longValue)`（scale 0）
    /// - BigInteger → `new BigDecimal(bigInteger)`（scale 0）
    /// - Float/Double → `new BigDecimal(num.toString())`（Java 注释：Double.toString 比
    ///   `new BigDecimal(double)` 更贴近直觉，0.1 精确等于 "0.1"；Rust 用 `{:?}` 最短往返表示）
    /// - NaN/无穷 → Java 抛 NumberFormatException → Rust 抛 TemplateError::misc
    fn to_big_decimal(&self, n: &TNumber) -> Result<BigDecimal> {
        match n {
            TNumber::Int(v) => Ok(BigDecimal::from(*v)),
            TNumber::Long(v) => Ok(BigDecimal::from(*v)),
            TNumber::BigInt(v) => Ok(BigDecimal::from_bigint(v.clone(), 0)),
            TNumber::Float(v) => {
                if v.is_nan() {
                    return Err(TemplateError::misc(
                        "It's impossible to convert a NaN value (Float NaN) to BigDecimal.",
                    ));
                }
                if v.is_infinite() {
                    return Err(TemplateError::misc(format!(
                        "It's impossible to convert an infinite value (Float {v}) to BigDecimal."
                    )));
                }
                BigDecimal::from_str(&format!("{v:?}"))
                    .map_err(|e| TemplateError::misc(format!("Can't parse as BigDecimal: {e}")))
            }
            TNumber::Double(v) => {
                if v.is_nan() {
                    return Err(TemplateError::misc(
                        "It's impossible to convert a NaN value (Double NaN) to BigDecimal.",
                    ));
                }
                if v.is_infinite() {
                    return Err(TemplateError::misc(format!(
                        "It's impossible to convert an infinite value (Double {v}) to BigDecimal."
                    )));
                }
                BigDecimal::from_str(&format!("{v:?}"))
                    .map_err(|e| TemplateError::misc(format!("Can't parse as BigDecimal: {e}")))
            }
            TNumber::Decimal(v) => Ok(v.clone()),
        }
    }

    /// 对应 Java `BigDecimalEngine.divide(BigDecimal, BigDecimal)`（ArithmeticEngine.java:252-258）
    /// ```java
    /// int scale1 = left.scale();
    /// int scale2 = right.scale();
    /// int scale = Math.max(scale1, scale2);
    /// scale = Math.max(minScale, scale);
    /// return left.divide(right, scale, roundingPolicy);   // roundingPolicy = ROUND_HALF_UP（:70）
    /// ```
    /// 即 scale = max(s1, s2, minScale)，舍入方式 HALF_UP（远离零）。
    /// 用 unscaled(BigInt) 精确实现 Java BigDecimal.divide 的"精确商舍入到 scale 位"语义，
    /// 避免 f64 近似。
    fn divide_big_decimal(&self, left: &BigDecimal, right: &BigDecimal, scale: i64) -> BigDecimal {
        let (n1, s1) = left.as_bigint_and_scale();
        let (n2, s2) = right.as_bigint_and_scale();
        // 精确商 = n1/n2 * 10^(s2-s1)，舍入到 scale 位小数：
        // num/den = n1*10^shift / n2，其中 shift = scale + s2 - s1（负数时移给分母）
        let shift = scale + s2 - s1;
        let (num, den) = if shift >= 0 {
            (
                n1.into_owned() * BigInt::from(10).pow(shift as u32),
                n2.into_owned(),
            )
        } else {
            (
                n1.into_owned(),
                n2.into_owned() * BigInt::from(10).pow((-shift) as u32),
            )
        };
        let q = &num / &den; // 截断除法（Java BigInteger.divide 同语义）
        let r = &num % &den; // 余数符号同被除数
        if r.is_zero() {
            return BigDecimal::from_bigint(q, scale);
        }
        // ROUND_HALF_UP：|2r| >= |den| 时向远离零的方向进位（Java 舍入语义）。
        // 进位方向必须跟"精确商（num/den）"的符号：num 与 den 异号则结果为负。
        // （BigDecimal.divide(right, scale, ROUND_HALF_UP) 远离零舍入，ArithmeticEngine.java:257；
        //   只按 num 符号进位在除数为负时会进错方向：2/-3 会得 -0.666666666665 而非 -0.666666666667）
        let round_up = (&r + &r).abs() >= den.abs();
        if round_up {
            let negative = (num.sign() == Sign::Minus) != (den.sign() == Sign::Minus);
            let inc = if negative {
                BigInt::from(-1)
            } else {
                BigInt::from(1)
            };
            return BigDecimal::from_bigint(q + inc, scale);
        }
        BigDecimal::from_bigint(q, scale)
    }
}

impl ArithmeticEngine for BigDecimalEngine {
    /// 对应 Java `BigDecimalEngine.add`（ArithmeticEngine.java:208-213）：
    /// BigDecimal 加法，结果 scale = max(s1, s2)（BigDecimal 语义）
    fn add(&self, a: &TNumber, b: &TNumber) -> Result<TNumber> {
        let left = self.to_big_decimal(a)?;
        let right = self.to_big_decimal(b)?;
        Ok(optimize_number_representation(TNumber::Decimal(
            left + right,
        )))
    }

    /// 对应 Java `BigDecimalEngine.subtract`（ArithmeticEngine.java:215-220）
    fn sub(&self, a: &TNumber, b: &TNumber) -> Result<TNumber> {
        let left = self.to_big_decimal(a)?;
        let right = self.to_big_decimal(b)?;
        Ok(optimize_number_representation(TNumber::Decimal(
            left - right,
        )))
    }

    /// 对应 Java `BigDecimalEngine.multiply`（ArithmeticEngine.java:222-231）：
    /// 乘法结果 scale = s1 + s2；若 scale > maxScale 则 setScale(maxScale, ROUND_HALF_UP)
    fn mul(&self, a: &TNumber, b: &TNumber) -> Result<TNumber> {
        let left = self.to_big_decimal(a)?;
        let right = self.to_big_decimal(b)?;
        let mut result = left * right;
        let (_, scale) = result.as_bigint_and_scale();
        if scale > self.max_scale {
            result = result.with_scale_round(self.max_scale, RoundingMode::HalfUp);
        }
        Ok(optimize_number_representation(TNumber::Decimal(result)))
    }

    /// 对应 Java `BigDecimalEngine.divide`（ArithmeticEngine.java:233-238 → :252-258）
    /// scale = max(s1, s2, minScale)，ROUND_HALF_UP；除零抛错（Java ArithmeticException）
    fn div(&self, a: &TNumber, b: &TNumber) -> Result<TNumber> {
        let left = self.to_big_decimal(a)?;
        let right = self.to_big_decimal(b)?;
        if right.is_zero() {
            return Err(TemplateError::misc("Division by zero"));
        }
        let (_, s1) = left.as_bigint_and_scale();
        let (_, s2) = right.as_bigint_and_scale();
        let scale = s1.max(s2).max(self.min_scale);
        let result = self.divide_big_decimal(&left, &right, scale);
        Ok(optimize_number_representation(TNumber::Decimal(result)))
    }

    /// 对应 Java `BigDecimalEngine.modulus`（ArithmeticEngine.java:240-245）：
    /// ```java
    /// long left = first.longValue();   // 截断转 long（BigDecimal.longValue 向零截断）
    /// long right = second.longValue();
    /// return Long.valueOf(left % right);
    /// ```
    /// 即"转 long 后取余"，结果恒为 Long；余数符号同被除数（Java % 语义）。
    /// 注意：不是 BigDecimal.remainder —— 那是 ConservativeEngine 的 BIGDECIMAL 分支
    /// （:530-532 直接抛 "Can't calculate remainder on BigDecimals"）。
    fn mod_op(&self, a: &TNumber, b: &TNumber) -> Result<TNumber> {
        let left = to_long_value(a).ok_or_else(|| {
            TemplateError::misc(format!(
                "Can't calculate modulus: operand {} can't be represented as long",
                a.to_plain_string()
            ))
        })?;
        let right = to_long_value(b).ok_or_else(|| {
            TemplateError::misc(format!(
                "Can't calculate modulus: operand {} can't be represented as long",
                b.to_plain_string()
            ))
        })?;
        if right == 0 {
            return Err(TemplateError::misc("Division by zero"));
        }
        Ok(TNumber::Long(left % right))
    }

    /// Java `BigDecimalEngine` 无 negate（FTL 无一元负号，字面量在解析期折叠）。
    /// Rust 侧语义等价于 BigDecimalEngine 下的 `0 - a`（全部走 BigDecimal 再优化表示）。
    fn negate(&self, a: &TNumber) -> Result<TNumber> {
        let bd = self.to_big_decimal(a)?;
        Ok(optimize_number_representation(TNumber::Decimal(-bd)))
    }
}

/// Java `Number.longValue()` 语义（截断向零；NaN→0、±Inf→饱和，Rust `as i64` 同行为）。
/// 返回 None 表示无法精确表示（Java 会静默截断/回绕，Rust 选择报错，避免静默丢值）。
fn to_long_value(n: &TNumber) -> Option<i64> {
    match n {
        TNumber::Int(v) => Some(*v as i64),
        TNumber::Long(v) => Some(*v),
        TNumber::BigInt(v) => i64::try_from(v.clone()).ok(),
        TNumber::Decimal(d) => i64::try_from(decimal_to_bigint_trunc(d)).ok(),
        TNumber::Float(v) => Some(*v as i64),
        TNumber::Double(v) => Some(*v as i64),
    }
}

/// BigDecimal → BigInt：`with_scale(0)` 精确取整（scale>0 截断向零，等价 Java longValue/BigInteger
/// 转换的截断语义；scale<0 乘 10 的幂）。无需 num_traits::ToBigInt（bigdecimal 未重导出）。
fn decimal_to_bigint_trunc(d: &BigDecimal) -> BigInt {
    d.with_scale(0).as_bigint_and_scale().0.into_owned()
}

/// 对应 Java `freemarker.template.utility.OptimizerUtil.optimizeNumberRepresentation`
/// （OptimizerUtil.java:70-96）：
/// ```java
/// BigDecimal：scale()==0 → unscaledValue()（BigInteger），否则 → doubleValue()
/// BigInteger：int 范围 → Integer；long 范围 → Long
/// ```
/// Rust 调整（docs/06 §3 设计决策）：
/// - BigDecimal 按"值是否为整数"（is_integer）判定（1.00/100.0 也算整数，对应任务
///   测试 1.00→Int(1)、100.0→Int(100)），整数 → Int/Long/BigInt；
/// - 非整数 BigDecimal 保留为 Decimal（Java 会转 Double，Rust 为保精度不转，
///   见 docs/06 §3「不要用 f64 近似实现 Decimal 精确运算」）。
pub fn optimize_number_representation(n: TNumber) -> TNumber {
    match n {
        TNumber::Decimal(d) => {
            if d.is_integer() {
                // 整数 Decimal（含尾零 1.00/100.0）→ 精确 BigInt → Int/Long/BigInt
                number_from_bigint(decimal_to_bigint_trunc(&d))
            } else {
                TNumber::Decimal(d)
            }
        }
        TNumber::BigInt(bi) => number_from_bigint(bi),
        other => other,
    }
}

/// BigInteger → Int（i32 范围）→ Long（i64 范围）→ BigInt（Java OptimizerUtil.java:84-94）
fn number_from_bigint(bi: BigInt) -> TNumber {
    if let Ok(i) = i32::try_from(bi.clone()) {
        TNumber::Int(i)
    } else if let Ok(l) = i64::try_from(bi.clone()) {
        TNumber::Long(l)
    } else {
        TNumber::BigInt(bi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: i32) -> TNumber {
        TNumber::Int(v)
    }
    fn long(v: i64) -> TNumber {
        TNumber::Long(v)
    }
    fn dec(s: &str) -> TNumber {
        TNumber::Decimal(BigDecimal::from_str(s).unwrap())
    }
    fn engine() -> BigDecimalEngine {
        BigDecimalEngine::default()
    }

    /// 断言为 Int 变体且值为 v
    fn assert_int(n: &TNumber, v: i32) {
        assert!(
            matches!(n, TNumber::Int(x) if *x == v),
            "expected Int({v}), got {n:?}"
        );
    }
    /// 断言为 Long 变体且值为 v
    fn assert_long(n: &TNumber, v: i64) {
        assert!(
            matches!(n, TNumber::Long(x) if *x == v),
            "expected Long({v}), got {n:?}"
        );
    }
    /// 断言为 Decimal 变体且 plain 字符串为 s
    fn assert_dec(n: &TNumber, s: &str) {
        match n {
            TNumber::Decimal(d) => assert_eq!(d.to_string(), s, "decimal mismatch for {s}"),
            other => panic!("expected Decimal({s}), got {other:?}"),
        }
    }
    /// 断言为 BigInt 变体且值为 s
    fn assert_bigint(n: &TNumber, s: &str) {
        match n {
            TNumber::BigInt(b) => assert_eq!(b.to_string(), s, "bigint mismatch"),
            other => panic!("expected BigInt({s}), got {other:?}"),
        }
    }
    /// 断言为除零错误
    fn assert_div_by_zero(r: Result<TNumber>) {
        match r {
            Err(TemplateError::Misc { message }) => {
                assert_eq!(message, "Division by zero")
            }
            other => panic!("expected 'Division by zero' error, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // 加法（Java: scale = max(s1, s2)，结果优化表示）
    // ------------------------------------------------------------------
    #[test]
    fn add_int_and_int() {
        assert_int(&engine().add(&int(1), &int(2)).unwrap(), 3);
        assert_int(&engine().add(&int(-1), &int(-2)).unwrap(), -3);
    }

    #[test]
    fn add_int_and_decimal_scale_is_max() {
        // 1 + 2.50：scale = max(0, 2) = 2 → 3.50（非整数保留 Decimal）
        assert_dec(&engine().add(&int(1), &dec("2.50")).unwrap(), "3.50");
        // 1.0 + 2.00：scale = max(1, 2) = 2 → 3.00 → 整数 → Int(3)
        assert_int(&engine().add(&dec("1.0"), &dec("2.00")).unwrap(), 3);
        // 1.50 + 2.55：scale 2 → 4.05
        assert_dec(&engine().add(&dec("1.50"), &dec("2.55")).unwrap(), "4.05");
    }

    #[test]
    fn add_overflow_upgrades_to_long() {
        // i32 溢出 → Long（Java BigDecimalEngine 全程 BigDecimal，无溢出；
        // 优化表示时 2147483648 超 i32 → Long）
        assert_long(&engine().add(&int(i32::MAX), &int(1)).unwrap(), 2147483648);
        assert_long(
            &engine().add(&int(i32::MIN), &int(-1)).unwrap(),
            -2147483649,
        );
    }

    #[test]
    fn add_long_result_keeps_long_variant() {
        // 4000000000 超 i32 → Long
        assert_long(
            &engine()
                .add(&long(2_000_000_000), &long(2_000_000_000))
                .unwrap(),
            4_000_000_000,
        );
    }

    #[test]
    fn add_float_double_goes_through_decimal_string() {
        // Java: new BigDecimal(Double.toString(0.1)) → 0.1 + 0.2 = 0.3（非 0.30000000000000004）
        assert_dec(
            &engine()
                .add(&TNumber::Double(0.1), &TNumber::Double(0.2))
                .unwrap(),
            "0.3",
        );
        // Float 同路径：1.5 + 2.5 = 4.0 → 整数 → Int(4)
        assert_int(
            &engine()
                .add(&TNumber::Float(1.5), &TNumber::Float(2.5))
                .unwrap(),
            4,
        );
    }

    // ------------------------------------------------------------------
    // 减法（Java: scale = max(s1, s2)）
    // ------------------------------------------------------------------
    #[test]
    fn sub_int_and_decimal() {
        assert_dec(&engine().sub(&int(1), &dec("0.5")).unwrap(), "0.5");
        assert_int(&engine().sub(&int(1), &int(3)).unwrap(), -2);
        assert_dec(&engine().sub(&dec("2.00"), &dec("0.5")).unwrap(), "1.50");
    }

    #[test]
    fn sub_overflow_upgrades() {
        assert_long(&engine().sub(&int(i32::MIN), &int(1)).unwrap(), -2147483649);
        // i64 溢出 → BigInt
        assert_bigint(
            &engine().sub(&long(i64::MIN), &long(1)).unwrap(),
            "-9223372036854775809",
        );
    }

    // ------------------------------------------------------------------
    // 乘法（Java: scale = s1 + s2；scale > maxScale(12) 时 clamp + HALF_UP）
    // ------------------------------------------------------------------
    #[test]
    fn mul_int_and_decimal() {
        assert_dec(&engine().mul(&int(3), &dec("1.5")).unwrap(), "4.5");
        // 2.0 × 3.0：scale 2 → 6.00 → 整数 → Int(6)
        assert_int(&engine().mul(&dec("2.0"), &dec("3.0")).unwrap(), 6);
    }

    #[test]
    fn mul_clamps_scale_at_max_scale() {
        // scale 16 > 12 → setScale(12, HALF_UP)：0.0123456789012345 → 0.012345678901
        assert_dec(
            &engine()
                .mul(&dec("0.123456789012345"), &dec("0.1"))
                .unwrap(),
            "0.012345678901",
        );
    }

    #[test]
    fn mul_overflow_upgrades() {
        // 100000 × 100000 = 1e10 超 i32 → Long
        assert_long(
            &engine().mul(&int(100_000), &int(100_000)).unwrap(),
            10_000_000_000,
        );
        // 4e9 × 4e9 = 1.6e19 超 i64 → BigInt
        assert_bigint(
            &engine()
                .mul(&long(4_000_000_000), &long(4_000_000_000))
                .unwrap(),
            "16000000000000000000",
        );
    }

    // ------------------------------------------------------------------
    // 除法（Java: scale = max(s1, s2, minScale=12)，ROUND_HALF_UP，ArithmeticEngine.java:252-258）
    // ------------------------------------------------------------------
    #[test]
    fn div_scale_and_rounding_from_source() {
        // 1/3：scale = max(0,0,12) = 12 → 0.333333333333
        assert_dec(&engine().div(&int(1), &int(3)).unwrap(), "0.333333333333");
        // 1.0/3：scale = max(1,0,12) = 12 → 0.333333333333
        assert_dec(
            &engine().div(&dec("1.0"), &int(3)).unwrap(),
            "0.333333333333",
        );
        // 10/4：精确 2.5，scale 12 → 2.500000000000
        assert_dec(&engine().div(&int(10), &int(4)).unwrap(), "2.500000000000");
        // 2/3：0.666...6667（第 13 位 6 → 进位）
        assert_dec(&engine().div(&int(2), &int(3)).unwrap(), "0.666666666667");
        // 1/6：0.1666666666666... → 0.166666666667
        assert_dec(&engine().div(&int(1), &int(6)).unwrap(), "0.166666666667");
        // 1/2：精确 → 0.500000000000
        assert_dec(&engine().div(&int(1), &int(2)).unwrap(), "0.500000000000");
    }

    #[test]
    fn div_negative_rounding_half_up_away_from_zero() {
        // -1/3：-0.3333333333333...，HALF_UP 远离零，第 13 位 3 → 不进位
        assert_dec(&engine().div(&int(-1), &int(3)).unwrap(), "-0.333333333333");
        // -1/6：-0.1666666666666...，第 13 位 6 → 进位 → -0.166666666667
        assert_dec(&engine().div(&int(-1), &int(6)).unwrap(), "-0.166666666667");
        // -2/3 → -0.666666666667
        assert_dec(&engine().div(&int(-2), &int(3)).unwrap(), "-0.666666666667");
    }

    #[test]
    fn div_negative_divisor_rounds_away_from_zero() {
        // 除数为负：HALF_UP 仍远离零（进位方向跟精确商符号，ArithmeticEngine.java:252-258）。
        // 2/-3 = -0.666666666666...，第 13 位 6 → 进位 → -0.666666666667
        // （若进位方向只跟被除数符号，会错误得 -0.666666666665）
        assert_dec(&engine().div(&int(2), &int(-3)).unwrap(), "-0.666666666667");
        // -2/-3 = 0.666666666666... → 0.666666666667
        assert_dec(&engine().div(&int(-2), &int(-3)).unwrap(), "0.666666666667");
        // -2/3（被除数为负）→ -0.666666666667
        assert_dec(&engine().div(&int(-2), &int(3)).unwrap(), "-0.666666666667");
        // 1/-3 → -0.333333333333（不进位）
        assert_dec(&engine().div(&int(1), &int(-3)).unwrap(), "-0.333333333333");
        // 10/-4 = -2.5 非整数 → Decimal（scale 12）
        assert_dec(
            &engine().div(&int(10), &int(-4)).unwrap(),
            "-2.500000000000",
        );
        // Decimal 除数：1.0/-6 → scale = max(1,0,12) = 12 → -0.166666666667
        assert_dec(
            &engine().div(&dec("1.0"), &int(-6)).unwrap(),
            "-0.166666666667",
        );
    }

    #[test]
    fn div_exact_integer_result_is_optimized() {
        // 6/2 = 3.000000000000 → 整数 → Int(3)
        assert_int(&engine().div(&int(6), &int(2)).unwrap(), 3);
        // 10/5 → Int(2)
        assert_int(&engine().div(&int(10), &int(5)).unwrap(), 2);
        // 1.00/3：scale = max(2,0,12) = 12
        assert_dec(
            &engine().div(&dec("1.00"), &int(3)).unwrap(),
            "0.333333333333",
        );
    }

    #[test]
    fn div_bigint_mixed_with_int() {
        // BigInt 混合：10/3（BigDecimal 精确运算）
        assert_dec(
            &engine()
                .div(&TNumber::BigInt(BigInt::from(10)), &int(3))
                .unwrap(),
            "3.333333333333",
        );
        // BigInt 超出 i64：100000000000000000000/2 → 整数 → 优化后仍 BigInt
        let huge = TNumber::BigInt(BigInt::from_str("100000000000000000000").unwrap());
        let r = engine().div(&huge, &int(2)).unwrap();
        assert_bigint(&r, "50000000000000000000");
    }

    #[test]
    fn div_by_zero_errors() {
        assert_div_by_zero(engine().div(&int(1), &int(0)));
        assert_div_by_zero(engine().div(&dec("1.5"), &dec("0.0")));
        assert_div_by_zero(engine().div(&long(1), &long(0)));
    }

    #[test]
    fn div_nan_infinity_errors_like_java() {
        // Java：new BigDecimal(Double.toString(NaN)) → NumberFormatException
        assert!(engine().div(&TNumber::Double(f64::NAN), &int(1)).is_err());
        assert!(engine()
            .add(&TNumber::Double(f64::INFINITY), &int(1))
            .is_err());
    }

    // ------------------------------------------------------------------
    // 模运算（Java BigDecimalEngine.modulus：longValue 截断后 %，结果恒 Long，:240-245）
    // ------------------------------------------------------------------
    #[test]
    fn mod_long_based_semantics() {
        assert_long(&engine().mod_op(&int(10), &int(3)).unwrap(), 1);
        assert_long(&engine().mod_op(&int(7), &int(2)).unwrap(), 1);
        assert_long(&engine().mod_op(&int(0), &int(5)).unwrap(), 0);
        // Java %：余数符号同被除数
        assert_long(&engine().mod_op(&int(-10), &int(3)).unwrap(), -1);
        assert_long(&engine().mod_op(&int(10), &int(-3)).unwrap(), 1);
        assert_long(&engine().mod_op(&int(-10), &int(-3)).unwrap(), -1);
    }

    #[test]
    fn mod_truncates_fractional_operands_to_long() {
        // Java longValue() 截断：5.5 % 2 = 5 % 2 = 1
        assert_long(&engine().mod_op(&TNumber::Double(5.5), &int(2)).unwrap(), 1);
        // Decimal 截断：7.9 % 2 = 7 % 2 = 1
        assert_long(&engine().mod_op(&dec("7.9"), &int(2)).unwrap(), 1);
    }

    #[test]
    fn mod_by_zero_errors() {
        assert_div_by_zero(engine().mod_op(&int(1), &int(0)));
        assert_div_by_zero(engine().mod_op(&dec("1.5"), &dec("0")));
    }

    #[test]
    fn mod_out_of_long_range_errors() {
        // BigInt 超出 i64：Java 会静默回绕，Rust 报错（文档注明）
        let huge = TNumber::BigInt(BigInt::from_str("12345678901234567890123").unwrap());
        match engine().mod_op(&huge, &int(2)) {
            Err(TemplateError::Misc { message }) => {
                assert!(
                    message.contains("can't be represented as long"),
                    "{message}"
                );
            }
            other => panic!("expected Misc error, got {other:?}"),
        }
    }

    // ------------------------------------------------------------------
    // 取负（等价 BigDecimalEngine 下 0 - a，再优化表示）
    // ------------------------------------------------------------------
    #[test]
    fn negate_preserves_value() {
        assert_int(&engine().negate(&int(5)).unwrap(), -5);
        assert_int(&engine().negate(&int(-5)).unwrap(), 5);
        assert_dec(&engine().negate(&dec("1.5")).unwrap(), "-1.5");
        // i32::MIN 取负 → 2147483648 超 i32 → Long
        assert_long(&engine().negate(&int(i32::MIN)).unwrap(), 2147483648);
    }

    #[test]
    fn negate_bigint_and_decimal() {
        assert_bigint(
            &engine()
                .negate(&TNumber::BigInt(
                    BigInt::from_str("12345678901234567890123").unwrap(),
                ))
                .unwrap(),
            "-12345678901234567890123",
        );
        // Double 走 BigDecimal 路径 → Decimal（BigDecimalEngine 全程 BigDecimal 哲学）
        assert_dec(&engine().negate(&TNumber::Double(2.5)).unwrap(), "-2.5");
    }

    // ------------------------------------------------------------------
    // OptimizerUtil.optimizeNumberRepresentation
    // ------------------------------------------------------------------
    #[test]
    fn optimize_decimal_integer_to_int_long() {
        assert_int(&optimize_number_representation(dec("1.00")), 1);
        assert_int(&optimize_number_representation(dec("100.0")), 100);
        assert_int(&optimize_number_representation(dec("0.0")), 0);
        assert_int(&optimize_number_representation(dec("-1.00")), -1);
        assert_int(&optimize_number_representation(dec("1e3")), 1000);
        // i32 边界
        assert_int(&optimize_number_representation(dec("2147483647")), i32::MAX);
        assert_int(
            &optimize_number_representation(dec("-2147483648")),
            i32::MIN,
        );
        // 超 i32 → Long
        assert_long(
            &optimize_number_representation(dec("2147483648")),
            2147483648,
        );
        assert_long(
            &optimize_number_representation(dec("9223372036854775807")),
            i64::MAX,
        );
        // 超 i64 → BigInt
        assert_bigint(
            &optimize_number_representation(dec("9223372036854775808")),
            "9223372036854775808",
        );
    }

    #[test]
    fn optimize_decimal_non_integer_kept() {
        // 非整数 Decimal 保留（Java 转 Double；Rust 保精度，docs/06 §3）
        assert_dec(&optimize_number_representation(dec("1.5")), "1.5");
        assert_dec(
            &optimize_number_representation(dec("0.333333333333")),
            "0.333333333333",
        );
    }

    #[test]
    fn optimize_bigint_downgrade() {
        assert_int(
            &optimize_number_representation(TNumber::BigInt(BigInt::from(42))),
            42,
        );
        assert_long(
            &optimize_number_representation(TNumber::BigInt(BigInt::from(2147483648i64))),
            2147483648,
        );
        let huge = TNumber::BigInt(BigInt::from_str("12345678901234567890123").unwrap());
        assert_bigint(
            &optimize_number_representation(huge.clone()),
            "12345678901234567890123",
        );
    }

    #[test]
    fn optimize_non_decimal_passthrough() {
        assert_int(&optimize_number_representation(int(5)), 5);
        assert_long(&optimize_number_representation(long(5)), 5);
        assert!(matches!(
            optimize_number_representation(TNumber::Double(1.5)),
            TNumber::Double(v) if v == 1.5
        ));
        assert!(matches!(
            optimize_number_representation(TNumber::Float(1.5)),
            TNumber::Float(v) if v == 1.5
        ));
    }
}
