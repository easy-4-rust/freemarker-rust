//! 比较表达式 —— 对应 Java `freemarker.core.ComparisonExpression`
//! （`evalToBoolean` :92-97 → `EvalUtil.compare` :183-317；`==`/`!=`/`>`/`>=`/`<`/`<=`）

use crate::core::eval::eval;
use crate::core::Expr;
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use std::cmp::Ordering;
use unicode_normalization::UnicodeNormalization;

/// 比较运算（Java ComparisonExpression.operation 字段；比较逻辑即
/// EvalUtil.compare :183-317，见 `compare_models`）
#[derive(Clone, Copy)]
pub enum CmpOp {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
}

/// 比较表达式（对应 ComparisonExpression.java；解析器经
/// `ExprKind::Eq/NotEq/Gt/Gte/Lt/Lte` 承载，dispatch 时按 variant 选定 `op` 构造）
pub struct ComparisonExpression {
    pub left: Expr,
    pub right: Expr,
    pub op: CmpOp,
}

impl ComparisonExpression {
    /// 构造（Java 构造器；Rust 侧由解析器/求值 dispatch 产生）
    pub fn new(left: Expr, right: Expr, op: CmpOp) -> Self {
        ComparisonExpression { left, right, op }
    }

    /// 求值（Java `evalToBoolean` :92-97）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval_compare(env, &self.left, &self.right, self.op)
    }
}

fn eval_compare(
    env: &mut crate::core::Environment,
    a: &Expr,
    b: &Expr,
    op: CmpOp,
) -> Result<TModel> {
    let l = eval(env, a)?;
    let r = eval(env, b)?;
    let ord = compare_models(env, &l, &r, op)?;
    Ok(TModel::from_boolean(ord))
}

/// 模型比较 —— 对照 Java `EvalUtil.compare`（EvalUtil.java:183-317）：
/// - 数字 vs 数字：按 BigDecimal 数值比较（Java ArithmeticEngine.compareNumbers）；
/// - 日期 vs 日期：类型必须一致（:239-250 报错），按时间戳比较；
/// - 字符串 vs 字符串：只允许 == 和 !=（:261-267 报错），按 NFKC 归一化后 compareTo
///   （v1 用 UTF-16 码元字典序近似，注释见下）；
/// - 布尔 vs 布尔：只允许 == 和 !=（:269-275 报错）；
/// - 跨类型：报 "Can't compare values of these types..."（:307-326，classic 模式除外——v1 不支持）。
///   供 exec.rs 的 `<#switch>` case 比较复用。
pub fn compare_models(
    _env: &mut crate::core::Environment,
    l: &TModel,
    r: &TModel,
    op: CmpOp,
) -> Result<bool> {
    let order = if l.is_number() && r.is_number() {
        // Java ArithmeticEngine.compareNumbers（:295-360）：先按符号判定（无穷可比较），
        // 同类型直接 compareTo，其余转 BigDecimal
        compare_numbers(&l.get_number()?, &r.get_number()?)
    } else if l.is_date() && r.is_date() {
        let ld = l.get_date()?;
        let rd = r.get_date()?;
        if ld.kind != rd.kind {
            // Java :240-250：Can't compare dates of different types.
            return Err(TemplateError::misc(format!(
                "Can't compare dates of different types. Left date type is {}, right date type is {}.",
                ld.kind.name(),
                rd.kind.name()
            )));
        }
        ld.dt.cmp(&rd.dt)
    } else if l.is_scalar() && r.is_scalar() {
        if !matches!(op, CmpOp::Eq | CmpOp::NotEq) {
            // Java :262-266：Can't use operator ">" on string values.
            return Err(TemplateError::misc(format!(
                "Can't use operator \"{}\" on string values.",
                cmp_op_str(op)
            )));
        }
        // Java 2.3.34（IcI >= 2.3.33）：NFKC 归一化后 compareTo（:282-286）。
        // v1 近似：UTF-16 码元字典序（encode_utf16 逐码元比较；NFKC 归一化属 P4）。
        let ls = l.get_scalar()?;
        let rs = r.get_scalar()?;
        // Java 2.3.34（IcI >= 2.3.33）：Normalizer.normalize(NFKC) 后 compareTo
        // （EvalUtil.java:282-286）——`'á' == 'a\u0301'` 规范化后相等
        let ln: String = ls.chars().nfkc().collect();
        let rn: String = rs.chars().nfkc().collect();
        utf16_cmp(&ln, &rn)
    } else if l.is_boolean() && r.is_boolean() {
        if !matches!(op, CmpOp::Eq | CmpOp::NotEq) {
            return Err(TemplateError::misc(format!(
                "Can't use operator \"{}\" on boolean values.",
                cmp_op_str(op)
            )));
        }
        let lb = l.get_boolean()?;
        let rb = r.get_boolean()?;
        lb.cmp(&rb)
    } else {
        // Java :307-326：Can't compare values of these types.
        return Err(TemplateError::misc(
            "Can't compare values of these types. Allowed comparisons are between two numbers, two strings, two dates, or two booleans.",
        ));
    };
    Ok(match op {
        CmpOp::Eq => order == Ordering::Equal,
        CmpOp::NotEq => order != Ordering::Equal,
        CmpOp::Gt => order == Ordering::Greater,
        CmpOp::Gte => order != Ordering::Less,
        CmpOp::Lt => order == Ordering::Less,
        CmpOp::Lte => order != Ordering::Greater,
    })
}

/// 数字比较（Java ArithmeticEngine.compareNumbers 的 v1 复刻：符号优先，
/// 避免无穷/NaN 转 BigDecimal 失败——Java 注释 "Infinity > 0" 不会失败）
pub(crate) fn compare_numbers(a: &crate::value::TNumber, b: &crate::value::TNumber) -> Ordering {
    use crate::value::TNumber as N;
    let sa = number_signum(a);
    let sb = number_signum(b);
    if sa != sb {
        return sa.cmp(&sb);
    }
    if sa == 0 && sb == 0 {
        return Ordering::Equal;
    }
    match (a, b) {
        (N::Float(x), N::Float(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (N::Double(x), N::Double(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (N::Int(x), N::Int(y)) => x.cmp(y),
        (N::Long(x), N::Long(y)) => x.cmp(y),
        (N::BigInt(x), N::BigInt(y)) => x.cmp(y),
        (N::Float(x), N::Double(y)) => (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal),
        (N::Double(x), N::Float(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal),
        _ => a.as_big_decimal().cmp(&b.as_big_decimal()),
    }
}

/// 数值符号（-1/0/1；Java NumberUtil.getSignum）
fn number_signum(n: &crate::value::TNumber) -> i32 {
    use crate::value::TNumber as N;
    match n {
        N::Int(v) => v.signum(),
        N::Long(v) => v.signum() as i32,
        N::BigInt(v) => match v.sign() {
            num_bigint::Sign::Minus => -1,
            num_bigint::Sign::NoSign => 0,
            num_bigint::Sign::Plus => 1,
        },
        N::Decimal(d) => match d.sign() {
            num_bigint::Sign::Minus => -1,
            num_bigint::Sign::NoSign => 0,
            num_bigint::Sign::Plus => 1,
        },
        N::Float(v) => {
            if *v > 0.0 {
                1
            } else if *v < 0.0 {
                -1
            } else {
                0
            }
        }
        N::Double(v) => {
            if *v > 0.0 {
                1
            } else if *v < 0.0 {
                -1
            } else {
                0
            }
        }
    }
}

fn cmp_op_str(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::NotEq => "!=",
        CmpOp::Gt => ">",
        CmpOp::Gte => ">=",
        CmpOp::Lt => "<",
        CmpOp::Lte => "<=",
    }
}

/// UTF-16 码元字典序（近似 Java String.compareTo 的 UTF-16 char 比较；
/// 常见 BMP 文本与 Rust str 字节序一致，非 BMP 字符差异属 P4 对齐项）
fn utf16_cmp(a: &str, b: &str) -> Ordering {
    let au: Vec<u16> = a.encode_utf16().collect();
    let bu: Vec<u16> = b.encode_utf16().collect();
    for (x, y) in au.iter().zip(bu.iter()) {
        match x.cmp(y) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    au.len().cmp(&bu.len())
}
