//! 赋值指令 —— 对应 Java `freemarker.core.Assignment`
//! （accept :80-168 + AssignmentInstruction/BlockAssignment/GlobalAssignment/LocalAssignment
//! 共享的 exec_assign 实现；Java Assignment.java:100-110 的 NAMESPACE/LOCAL/GLOBAL 作用域）

use crate::core::arithmetic_engine::ArithmeticEngine;
use crate::core::environment::{expr_desc, model_to_string};
use crate::core::eval;
use crate::core::exec::ExecOutcome;
use crate::core::{AssignOp, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;
use std::rc::Rc;

/// `<#assign name = expr>`（含 += 等操作符与 `in nsExp` 子句；对应 Assignment.java）
pub struct Assignment {
    pub target: String,
    pub expr: Expr,
    pub op: AssignOp,
    pub namespace: Option<Expr>,
}

impl Assignment {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(target: String, expr: Expr, op: AssignOp, namespace: Option<Expr>) -> Self {
        Assignment {
            target,
            expr,
            op,
            namespace,
        }
    }

    pub(crate) fn exec(&self, env: &mut crate::core::Environment) -> Result<ExecOutcome> {
        exec_assign(
            env,
            &self.target,
            &self.expr,
            &self.op,
            self.namespace.as_ref(),
            AssignScope::Namespace,
        )
    }
}

/// 赋值作用域（Java Assignment.java:100-110 NAMESPACE/LOCAL/GLOBAL）
pub(crate) enum AssignScope {
    Namespace,
    Global,
    Local,
}

/// 执行（Java Assignment.accept :80-168）：
/// `=` 直接赋值（缺失 → InvalidReference）；`+=` 先取旧值再字符串拼接/数值相加
/// （AddConcatExpression._eval）；`-=`/`*=`/`/=`/`%=` 数值运算（ArithmeticExpression._eval）；
/// `++`/`--` 数值 ±1（Java :147-157，ONE = 1）
pub(crate) fn exec_assign(
    env: &mut crate::core::Environment,
    target: &str,
    expr: &crate::core::Expr,
    op: &AssignOp,
    namespace: Option<&crate::core::Expr>,
    scope: AssignScope,
) -> Result<ExecOutcome> {
    // Java Assignment.accept :102-122：`in nsExp` 子句——nsExp 为任意表达式，
    // eval 后检查类型（NonNamespaceException）/ null（InvalidReference）
    let target_ns: Option<Rc<crate::core::environment::Namespace>> = match namespace {
        None => None,
        Some(ns_exp) => {
            let m = eval::eval(env, ns_exp)?;
            if m.is_nothing() {
                return Err(TemplateError::invalid_reference(
                    crate::core::environment::expr_desc(ns_exp),
                ));
            }
            Some(env.as_namespace(&m).ok_or_else(|| {
                // Java NonNamespaceException（Assignment.java:115-118）：
                // "For \"#assign\" namespace: Expected a namespace, but this has evaluated to a ..."
                TemplateError::misc(format!(
                    "For \"#assign\" namespace: Expected a namespace, but this has evaluated to a {}: ==> {}",
                    m.type_name,
                    crate::core::environment::expr_desc(ns_exp)
                ))
            })?)
        }
    };
    let value = if *op == AssignOp::Equals {
        // Java :99-110（Assignment.java:136-142）：= 右侧为 null → classic 兼容模式
        // 赋空串（TemplateScalarModel.EMPTY_STRING）；strict → InvalidReference
        let v = eval::eval(env, expr)?;
        if v.is_nothing() {
            if env.settings.classic_compatible {
                TModel::from_scalar(String::new())
            } else {
                return Err(TemplateError::invalid_reference(expr_desc(expr)));
            }
        } else {
            v
        }
    } else {
        // Java :112-157：先取旧值（缺失 → Assignment.java:156-162 的
        // "The target variable of the assignment, ... was null or missing ..."；
        // += 在 classic 兼容模式下缺失目标视为空串，Assignment.java:140-144）
        let old = match get_old_value(env, target, &target_ns, &scope)? {
            Some(old) => old,
            None if *op == AssignOp::PlusEq && env.settings.classic_compatible => {
                TModel::from_scalar(String::new())
            }
            None => {
                // Java Assignment.java:156-162 + InvalidReferenceException 的 Tip 段
                // （目标名以 $ 开头时追加 "must not start with \"$\"" 提示；
                // 作用域描述按 scope 变化：template namespace / global scope / local scope）
                let scope_desc = match scope {
                    AssignScope::Namespace => "template namespace",
                    AssignScope::Global => "global scope",
                    AssignScope::Local => "local scope",
                };
                let mut msg = format!(
                    "The target variable of the assignment, \"{target}\", was null or missing in the {scope_desc}, and the \"{}\" operator must get its value from there before assigning to it.",
                    assign_op_str(op)
                );
                if target.starts_with('$') {
                    msg.push_str("\n\n----\nTip: Variable references must not start with \"$\", unless the \"$\" is really part of the variable name.\n----");
                }
                return Err(TemplateError::misc(msg));
            }
        };
        match op {
            AssignOp::PlusEq => {
                // Java :132-147：AddConcat 语义（字符串拼接或数值相加）；
                // 右侧为 null → classic 兼容模式视为空串（Assignment.java:147-151）
                let new = eval::eval(env, expr)?;
                let new = if new.is_nothing() {
                    if env.settings.classic_compatible {
                        TModel::from_scalar(String::new())
                    } else {
                        return Err(TemplateError::invalid_reference(expr_desc(expr)));
                    }
                } else {
                    new
                };
                eval_add_concat(env, &old, &new)?
            }
            AssignOp::PlusPlus => {
                // Java :147-150：lhoNumber + 1
                let n = old
                    .get_number()
                    .map_err(|_| assign_non_number_err(target, &old))?;
                let one = crate::value::TNumber::Int(1);
                let engine = crate::core::BigDecimalEngine::default();
                TModel::from_number(engine.add(&n, &one)?)
            }
            AssignOp::MinusMinus => {
                // Java :151-154：lhoNumber - 1
                let n = old
                    .get_number()
                    .map_err(|_| assign_non_number_err(target, &old))?;
                let one = crate::value::TNumber::Int(1);
                let engine = crate::core::BigDecimalEngine::default();
                TModel::from_number(engine.sub(&n, &one)?)
            }
            AssignOp::MinusEq | AssignOp::TimesEq | AssignOp::DivideEq | AssignOp::ModuloEq => {
                // Java :155-157：ArithmeticExpression._eval(lhoNumber, op, rhoNumber)
                // 左值错误 → NonNumericalException（"Expected a number, but assignment
                // target variable ..."）；右值错误 → "For \"#assign\" assignment source:
                // Expected a number, but this has evaluated to a string: ==> 'a'"
                let l = old
                    .get_number()
                    .map_err(|_| assign_non_number_err(target, &old))?;
                // Java :155-157：右值 null → evalToNumber → NonNumericalException
                // （消息同 "null or missing"）
                let rm = eval::eval(env, expr)?;
                if rm.is_nothing() {
                    return Err(TemplateError::invalid_reference(expr_desc(expr)));
                }
                let r = rm.get_number().map_err(|_| {
                    TemplateError::misc(format!(
                        "For \"#assign\" assignment source: Expected a number, but this has evaluated to a string: ==> {}",
                        assign_source_desc(expr)
                    ))
                })?;
                let engine = crate::core::BigDecimalEngine::default();
                TModel::from_number(match op {
                    AssignOp::MinusEq => engine.sub(&l, &r)?,
                    AssignOp::TimesEq => engine.mul(&l, &r)?,
                    AssignOp::DivideEq => engine.div(&l, &r)?,
                    AssignOp::ModuloEq => engine.mod_op(&l, &r)?,
                    _ => unreachable!(),
                })
            }
            AssignOp::Equals => unreachable!(),
        }
    };
    // Java :159-165：写入目标
    match scope {
        AssignScope::Local => {
            env.set_local_variable(target, value)?;
        }
        AssignScope::Global => {
            env.set_global_variable(target, value);
        }
        AssignScope::Namespace => match &target_ns {
            Some(ns) => ns.put_var(target.to_string(), value),
            None => env.set_variable(target, value),
        },
    }
    Ok(ExecOutcome::Done)
}

/// 取旧值（Java Assignment :114-122：LOCAL → getLocalVariable；NAMESPACE/GLOBAL → 命名空间 get）
fn get_old_value(
    env: &mut crate::core::Environment,
    target: &str,
    target_ns: &Option<Rc<crate::core::environment::Namespace>>,
    scope: &AssignScope,
) -> Result<Option<TModel>> {
    match scope {
        AssignScope::Local => Ok(env.get_local_variable(target)),
        AssignScope::Global => Ok(env
            .get_global_namespace()
            .get_member(target)
            .and_then(normalize_old)),
        AssignScope::Namespace => match target_ns {
            Some(ns) => Ok(ns.get_member(target).and_then(normalize_old)),
            // 当前命名空间（Java :114-119：namespace.get(variableName)）
            None => Ok(env
                .get_current_namespace()
                .get_member(target)
                .and_then(normalize_old)),
        },
    }
}

/// 旧值为宏（`<#assign x += 1>` 目标若是宏名）→ 视为缺失（v1；Java 会抛类型错误）
fn normalize_old(m: TModel) -> Option<TModel> {
    if m.is_macro() {
        None
    } else {
        Some(m)
    }
}

/// 拼接/相加（Java AddConcatExpression._eval，Assignment :144 调用）：
/// 双数字 → 数值相加；双序列 → ConcatenatedSequence 懒惰拼接（:79-83）；
/// 双哈希且无法转字符串 → 哈希合并、右值胜出（:124-131）；否则字符串拼接
/// （字符串优先于哈希——FTL 字符串常兼为哈希，:85-102）
fn eval_add_concat(
    env: &mut crate::core::Environment,
    old: &TModel,
    new: &TModel,
) -> Result<TModel> {
    if old.is_number() && new.is_number() {
        let engine = crate::core::BigDecimalEngine::default();
        return Ok(TModel::from_number(
            engine.add(&old.get_number()?, &new.get_number()?)?,
        ));
    }
    if let (Some(l), Some(r)) = (&old.sequence, &new.sequence) {
        // Java :79-83：ConcatenatedSequence（懒惰拼接，不物化）
        return Ok(concatenated_sequence_model(l.clone(), r.clone()));
    }
    let both_hash = old.is_hash() && new.is_hash();
    // Java :85-102：先试字符串转换（双哈希时不可转 → null → 哈希合并）
    match (model_to_string(env, old), model_to_string(env, new)) {
        (Ok(ls), Ok(rs)) => Ok(TModel::from_scalar(ls + &rs)),
        _ if both_hash => merged_hash_model(old, new),
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}

/// 序列拼接模型 —— 对应 Java `ConcatenatedSequence`（AddConcatExpression.java:79-83）：
/// size = 左 + 右；get(i) 委派；迭代器基于 get/size 惰性生成
pub(crate) struct ConcatenatedSeq {
    left: Rc<dyn crate::template::TemplateSequenceModel>,
    right: Rc<dyn crate::template::TemplateSequenceModel>,
}

fn concatenated_sequence_model(
    left: Rc<dyn crate::template::TemplateSequenceModel>,
    right: Rc<dyn crate::template::TemplateSequenceModel>,
) -> TModel {
    let inner = Rc::new(ConcatenatedSeq { left, right });
    let seq: Rc<dyn crate::template::TemplateSequenceModel> = inner.clone();
    let coll: Rc<dyn crate::template::TemplateCollectionModel> = inner;
    TModel {
        sequence: Some(seq),
        collection: Some(coll),
        type_name: "sequence",
        kind: crate::template::ModelKind::Sequence,
        ..TModel::nothing()
    }
}

impl crate::template::TemplateSequenceModel for ConcatenatedSeq {
    fn get(&self, index: usize) -> Result<TModel> {
        let l = self.left.size()?;
        if index < l {
            self.left.get(index)
        } else {
            self.right.get(index - l)
        }
    }
    fn size(&self) -> Result<usize> {
        Ok(self.left.size()? + self.right.size()?)
    }
}

impl crate::template::TemplateCollectionModel for ConcatenatedSeq {
    fn iterator(&self) -> Result<Box<dyn Iterator<Item = Result<TModel>>>> {
        let left = self.left.clone();
        let right = self.right.clone();
        let l = left.size()?;
        let n = l + right.size()?;
        let mut idx = 0usize;
        Ok(Box::new(std::iter::from_fn(move || {
            if idx >= n {
                return None;
            }
            let i = idx;
            idx += 1;
            Some(if i < l { left.get(i) } else { right.get(i - l) })
        })))
    }
}

/// 哈希合并 —— 对应 Java `ConcatenatedHashEx`/`ConcatenatedHash`
/// （AddConcatExpression.java:462-…）：get 右优先（right ?? left）；
/// 键序左在前、右键追加（碰撞保留左索引、右值胜出——IndexMap 语义一致）。
/// 双 ex 哈希直接物化合并；否则惰性右优先查找包装
fn merged_hash_model(left: &TModel, right: &TModel) -> Result<TModel> {
    if let (Some(l), Some(r)) = (&left.hash_ex, &right.hash_ex) {
        let mut m: indexmap::IndexMap<String, TModel> = indexmap::IndexMap::new();
        for ex in [l, r] {
            for (k, v) in ex.entries()? {
                m.insert(k, v);
            }
        }
        return Ok(TModel::from_hash(m));
    }
    let left_h = left.hash.clone().ok_or_else(|| {
        TemplateError::misc(format!(
            "Cannot concatenate a {} value with a hash",
            left.type_name
        ))
    })?;
    let right_h = right.hash.clone().ok_or_else(|| {
        TemplateError::misc(format!(
            "Cannot concatenate a {} value with a hash",
            right.type_name
        ))
    })?;
    let inner = Rc::new(CombinedHash {
        left: left_h,
        right: right_h,
    });
    let h: Rc<dyn crate::template::TemplateHashModel> = inner;
    Ok(TModel {
        hash: Some(h),
        type_name: "hash",
        kind: crate::template::ModelKind::Hash,
        ..TModel::nothing()
    })
}

/// 右优先查找的惰性合并哈希（Java `ConcatenatedHash.get`）
struct CombinedHash {
    left: Rc<dyn crate::template::TemplateHashModel>,
    right: Rc<dyn crate::template::TemplateHashModel>,
}

impl crate::template::TemplateHashModel for CombinedHash {
    fn get(&self, key: &str) -> Result<Option<TModel>> {
        if let Some(v) = self.right.get(key)? {
            return Ok(Some(v));
        }
        self.left.get(key)
    }
    fn is_empty(&self) -> Result<bool> {
        Ok(self.left.is_empty()? && self.right.is_empty()?)
    }
}

/// 赋值操作符文本（Java `Assignment.getOperatorTypeAsString`，Assignment.java:67-78）
fn assign_op_str(op: &AssignOp) -> &'static str {
    match op {
        AssignOp::Equals => "=",
        AssignOp::PlusEq => "+=",
        AssignOp::MinusEq => "-=",
        AssignOp::TimesEq => "*=",
        AssignOp::DivideEq => "/=",
        AssignOp::ModuloEq => "%=",
        AssignOp::PlusPlus => "++",
        AssignOp::MinusMinus => "--",
    }
}

/// 赋值目标非数值错误（Java `NonNumericalException`，Assignment.java:166-169：
/// "Expected a number, but assignment target variable \"foo\" has evaluated to a string"）
fn assign_non_number_err(target: &str, old: &TModel) -> TemplateError {
    TemplateError::misc(format!(
        "Expected a number, but assignment target variable \"{target}\" has evaluated to a {}.",
        old.type_name
    ))
}

/// 赋值源表达式描述（字符串按 FTL 单引号保留原样——Java 的 blamed expression
/// 保留源码文本，错误消息须含 "'a'" 之类；其余委托 expr_desc）
fn assign_source_desc(e: &crate::core::Expr) -> String {
    use crate::core::ExprKind as K;
    match &e.kind {
        K::Str(s) => format!("'{}'", s),
        _ => expr_desc(e),
    }
}
