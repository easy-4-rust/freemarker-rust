//! Java `freemarker.template.utility.NumberUtilTest` 的 Rust 1:1 实现
//! （NumberUtilTest.java：getSignum / isIntegerBigDecimal / toIntExact 纯函数测试）
//!
//! 引擎映射：v1 无公开 NumberUtil——按 Java 语义在测试内实现同名纯函数
//! （作用于 `freemarker::value::TNumber`，数值家族等价：
//! Double/Float→TNumber::Double/Float、Long→Long、Integer→Int、
//! Short/Byte→Int、BigDecimal→Decimal、BigInteger→BigInt）。
//! 引擎差异：Java 失败路径抛 ArithmeticException；v1 用 panic!（Rust 等价）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::value::TNumber;
use std::str::FromStr;

/// 对应 NumberUtil.getSignum（Java NumberUtil.java：NaN → ArithmeticException；
/// 无穷 → ±1）
fn get_signum(n: &TNumber) -> i32 {
    match n {
        TNumber::Float(f) => {
            assert!(!f.is_nan(), "NaN");
            if *f > 0.0 {
                1
            } else if *f < 0.0 {
                -1
            } else {
                0
            }
        }
        TNumber::Double(d) => {
            assert!(!d.is_nan(), "NaN");
            if *d > 0.0 {
                1
            } else if *d < 0.0 {
                -1
            } else {
                0
            }
        }
        other => match other.as_i64() {
            Some(v) => v.signum() as i32,
            None => panic!("无法取符号"),
        },
    }
}

/// 对应 NumberUtil.isIntegerBigDecimal：BigDecimal 是否表示整数值
/// （等价断言：doubleValue() == longValue()；bigdecimal::is_integer =
/// scale<=0 或小数部分为零，语义同 Java stripTrailingZeros().scale()<=0）
fn is_integer_big_decimal(bd: &bigdecimal::BigDecimal) -> bool {
    bd.is_integer()
}

/// 对应 NumberUtil.toIntExact：精确转 int，失败抛 ArithmeticException（v1 panic）
fn to_int_exact(n: &TNumber) -> i32 {
    match n {
        TNumber::Int(i) => *i,
        TNumber::Long(l) => i32::try_from(*l).expect("ArithmeticException"),
        TNumber::BigInt(b) => {
            let v = i64::try_from(b.clone()).expect("ArithmeticException");
            i32::try_from(v).expect("ArithmeticException")
        }
        TNumber::Decimal(bd) => {
            if !bd.is_integer() {
                panic!("ArithmeticException");
            }
            let v = bd.to_string().parse::<i64>().expect("ArithmeticException");
            i32::try_from(v).expect("ArithmeticException")
        }
        TNumber::Float(f) => {
            let d = *f as f64;
            if d.is_nan() || d.fract() != 0.0 || d < i32::MIN as f64 || d > i32::MAX as f64 {
                panic!("ArithmeticException");
            }
            d as i32
        }
        TNumber::Double(d) => {
            if d.is_nan() || d.fract() != 0.0 || *d < i32::MIN as f64 || *d > i32::MAX as f64 {
                panic!("ArithmeticException");
            }
            *d as i32
        }
    }
}

/// Java testGetSignum
#[test]
fn test_get_signum() {
    // Double：
    assert_eq!(get_signum(&TNumber::Double(f64::INFINITY)), 1);
    assert_eq!(get_signum(&TNumber::Double(3.0)), 1);
    assert_eq!(get_signum(&TNumber::Double(0.0)), 0);
    assert_eq!(get_signum(&TNumber::Double(-3.0)), -1);
    assert_eq!(get_signum(&TNumber::Double(f64::NEG_INFINITY)), -1);
    assert!(std::panic::catch_unwind(|| get_signum(&TNumber::Double(f64::NAN))).is_err());

    // Float（v1 统一 Double，Java Float 用例等价）：
    assert_eq!(get_signum(&TNumber::Float(f32::INFINITY)), 1);
    assert_eq!(get_signum(&TNumber::Float(3.0)), 1);
    assert_eq!(get_signum(&TNumber::Float(0.0)), 0);
    assert_eq!(get_signum(&TNumber::Float(-3.0)), -1);
    assert_eq!(get_signum(&TNumber::Float(f32::NEG_INFINITY)), -1);
    assert!(std::panic::catch_unwind(|| get_signum(&TNumber::Float(f32::NAN))).is_err());

    // Long：
    assert_eq!(get_signum(&TNumber::Long(3)), 1);
    assert_eq!(get_signum(&TNumber::Long(0)), 0);
    assert_eq!(get_signum(&TNumber::Long(-3)), -1);

    // Integer（Short/Byte 用例 v1 同为 Int）：
    assert_eq!(get_signum(&TNumber::Int(3)), 1);
    assert_eq!(get_signum(&TNumber::Int(0)), 0);
    assert_eq!(get_signum(&TNumber::Int(-3)), -1);

    // BigDecimal：
    assert_eq!(
        get_signum(&TNumber::Decimal(bigdecimal::BigDecimal::from(3))),
        1
    );
    assert_eq!(
        get_signum(&TNumber::Decimal(bigdecimal::BigDecimal::from(0))),
        0
    );
    assert_eq!(
        get_signum(&TNumber::Decimal(bigdecimal::BigDecimal::from(-3))),
        -1
    );

    // BigInteger：
    assert_eq!(get_signum(&TNumber::BigInt(num_bigint::BigInt::from(3))), 1);
    assert_eq!(get_signum(&TNumber::BigInt(num_bigint::BigInt::from(0))), 0);
    assert_eq!(
        get_signum(&TNumber::BigInt(num_bigint::BigInt::from(-3))),
        -1
    );
}

/// Java testIsBigDecimalInteger：整数 BigDecimal 判定
/// （Java 断言 n.doubleValue()==n.longValue() 与 isIntegerBigDecimal 等价）
#[test]
fn test_is_big_decimal_integer() {
    let n1 = bigdecimal::BigDecimal::from_str("1.125").unwrap();
    let n2 = bigdecimal::BigDecimal::from_str("1.125").unwrap()
        - bigdecimal::BigDecimal::from_str("0.005").unwrap();
    let n3 = bigdecimal::BigDecimal::from_str("123").unwrap();
    let n4 = bigdecimal::BigDecimal::from_str("6000").unwrap();
    let n5 = bigdecimal::BigDecimal::from_str("1.12345").unwrap()
        - bigdecimal::BigDecimal::from_str("0.12345").unwrap();
    let n6 = bigdecimal::BigDecimal::from_str("0").unwrap();
    let n7 = bigdecimal::BigDecimal::from_str("0.001").unwrap()
        - bigdecimal::BigDecimal::from_str("0.001").unwrap();
    let n8 = bigdecimal::BigDecimal::from_str("60000.5").unwrap()
        - bigdecimal::BigDecimal::from_str("0.5").unwrap();
    let n9 = bigdecimal::BigDecimal::from_str("6")
        .unwrap()
        .with_scale(-3); // movePointRight(3).setScale(-3) → 6000

    // Java 的构造期自检（precision/scale）用 Rust 断言近似表达：
    assert_eq!(
        n1.as_bigint_and_exponent(),
        (num_bigint::BigInt::from(1125), 3)
    );
    assert_eq!(
        n5.as_bigint_and_exponent(),
        (num_bigint::BigInt::from(100000), 5)
    );

    let ns = [
        n1.clone(),
        n2.clone(),
        n3.clone(),
        n4.clone(),
        n5.clone(),
        n6.clone(),
        n7.clone(),
        n8.clone(),
        n9.clone(),
        -n1,
        -n2,
        -n3,
        -n4,
        -n5,
        -n6,
        -n7,
        -n8,
        -n9,
    ];
    for n in ns {
        // Java：assertEquals(n.doubleValue() == n.longValue(), isIntegerBigDecimal(n))
        let as_double = n.to_string().parse::<f64>().unwrap();
        // BigDecimal.longValue() 截断取整（num_traits ToPrimitive 语义）
        let as_long = bigdecimal::ToPrimitive::to_i64(&n).unwrap_or(i64::MIN);
        assert_eq!(
            as_double == as_long as f64,
            is_integer_big_decimal(&n),
            "n={n}"
        );
    }
}

/// Java testToIntExcact：精确转 int
#[test]
fn test_to_int_exact() {
    for n in [i32::MIN, i8::MIN as i32, -1, 0, 1, i8::MAX as i32, i32::MAX] {
        if n != i32::MIN && n != i32::MAX {
            assert_eq!(to_int_exact(&TNumber::Int(n)), n); // Byte/Short 用例 v1 同 Int
            assert_eq!(to_int_exact(&TNumber::Double(n as f64)), n); // Float 用例 v1 同 Double
        }
        assert_eq!(to_int_exact(&TNumber::Int(n)), n);
        assert_eq!(to_int_exact(&TNumber::Long(n as i64)), n);
        assert_eq!(to_int_exact(&TNumber::Double(n as f64)), n);
        assert_eq!(
            to_int_exact(&TNumber::Decimal(bigdecimal::BigDecimal::from(n))),
            n
        );
        assert_eq!(
            to_int_exact(&TNumber::Decimal(
                bigdecimal::BigDecimal::from(n as i64 * 10) / bigdecimal::BigDecimal::from(10)
            )),
            n
        );
        assert_eq!(
            to_int_exact(&TNumber::BigInt(num_bigint::BigInt::from(n))),
            n
        );
    }

    // 越界 Long：
    assert!(
        std::panic::catch_unwind(|| to_int_exact(&TNumber::Long(i32::MIN as i64 - 1))).is_err()
    );
    assert!(
        std::panic::catch_unwind(|| to_int_exact(&TNumber::Long(i32::MAX as i64 + 1))).is_err()
    );

    // Float/Double 非整数与越界：
    assert!(std::panic::catch_unwind(|| to_int_exact(&TNumber::Double(1.00001))).is_err());
    assert!(std::panic::catch_unwind(|| to_int_exact(&TNumber::Double(
        (i32::MIN as i64 - 1) as f64
    )))
    .is_err());
    assert!(std::panic::catch_unwind(|| to_int_exact(&TNumber::Double(
        (i32::MAX as i64 + 1) as f64
    )))
    .is_err());
    // Java Float 用例（1.00001f / MIN-129 / MAX 上界）v1 以 Double 路径覆盖：
    assert!(std::panic::catch_unwind(|| to_int_exact(&TNumber::Float(1.00001))).is_err());

    // BigDecimal 非整数与越界：
    assert!(std::panic::catch_unwind(|| to_int_exact(&TNumber::Decimal(
        bigdecimal::BigDecimal::from_str("100.000001").unwrap()
    )))
    .is_err());
    assert!(std::panic::catch_unwind(|| to_int_exact(&TNumber::Decimal(
        bigdecimal::BigDecimal::from(i32::MIN as i64 - 1)
    )))
    .is_err());
    assert!(std::panic::catch_unwind(|| to_int_exact(&TNumber::Decimal(
        bigdecimal::BigDecimal::from(i32::MAX as i64 + 1)
    )))
    .is_err());

    // BigInteger 越界：
    assert!(
        std::panic::catch_unwind(|| to_int_exact(&TNumber::BigInt(num_bigint::BigInt::from(
            i32::MIN as i64 - 1
        ))))
        .is_err()
    );
    assert!(
        std::panic::catch_unwind(|| to_int_exact(&TNumber::BigInt(num_bigint::BigInt::from(
            i32::MAX as i64 + 1
        ))))
        .is_err()
    );
}
