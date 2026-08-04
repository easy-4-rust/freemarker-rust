//! 点访问 —— 对应 Java `freemarker.core.Dot`
//! （`_eval` :49-62：目标为哈希/命名空间 → get(key)；否则 NonHashException）

use crate::core::eval::{eval, eval_builtin};
use crate::core::{Expr, ExprKind};
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// 点访问表达式（对应 Dot.java；解析器经 `ExprKind::Dot` 承载）
pub struct Dot {
    pub target: Expr,
    pub name: String,
}

impl Dot {
    /// 构造（Java 构造器；Rust 侧由解析器产生）
    pub fn new(target: Expr, name: String) -> Self {
        Dot { target, name }
    }

    /// 求值（Java `_eval`）
    pub(crate) fn eval(&self, env: &mut crate::core::Environment) -> Result<TModel> {
        eval_dot(env, &self.target, &self.name)
    }
}

fn eval_dot(env: &mut crate::core::Environment, target: &Expr, name: &str) -> Result<TModel> {
    // `?string.xs` / `?date.xs` / `?datetime.xs` / `?string.yes.no`：Java 中 ?string/?date 等
    // 返回"格式化器"模型（TemplateHashModel.get(key)，BuiltInsForMultipleTypes.java DateFormatter.get
    // :622-627 / dateBI.DateParser.get :146-150），点访问即格式参数；本引擎在求值期把点链
    // 合并为内建参数（解析器生成同样的 Dot(BuiltIn) 嵌套，见 grammar.rs builtin() 注释）
    if let Some((inner, bname, mut names)) = dot_builtin_chain(target) {
        names.push(name.to_string());
        let args: Vec<Expr> = names
            .iter()
            .map(|n| Expr::new(ExprKind::Str(n.clone()), target.span))
            .collect();
        return eval_builtin(env, &inner, &bname, &Some(args));
    }
    let t = eval(env, target)?;
    if t.is_nothing() {
        // Java Dot._eval / DynamicKeyName._eval：目标 null → classic 兼容模式继续
        // 传播 null（noSuchVar.foo.bar 整链求值为 null）；strict 模式 InvalidReference
        if env.settings.classic_compatible {
            return Ok(TModel::nothing());
        }
        return Err(TemplateError::invalid_reference(
            crate::core::environment::expr_desc(target),
        ));
    }
    // 命名空间成员（Java Namespace extends SimpleHash，含宏；`<@ns.macro>`/`ns.var`）；
    // 成员缺失 → Java 返回 null（SimpleHash.get 无键 → null），由使用点抛错
    if let Some(ns) = env.as_namespace(&t) {
        return Ok(ns.get_member(name).unwrap_or_else(TModel::nothing));
    }
    // 节点哈希角色（Java NodeModel 实现 TemplateHashModel：键 = 子元素名/@attr/@@key/
    // XPath；NodeHashModel.get 需 env —— ns_prefixes 解析）
    if let Some(nh) = &t.node_hash {
        return Ok(nh.get(env, name)?.unwrap_or_else(TModel::nothing));
    }
    if t.is_hash() {
        let h = t.get_hash()?;
        // 键缺失 → Java SimpleHash.get 返回 null 不抛（Dot._eval 仅 target null 时抛）
        return Ok(h.get(name)?.unwrap_or_else(TModel::nothing));
    }
    // Java NonHashException（blamed = target 表达式；位置 = target 起始）：
    // `For "." left-hand operand: Expected a hash, but this has evaluated to a {type}:`
    // `==> {target}`
    Err(
        TemplateError::type_mismatch("hash", t.type_name).with_blame_at(
            ".",
            "left-hand operand",
            &crate::core::environment::expr_desc(target),
            &env.current_template_name,
            target.span,
        ),
    )
}

/// 收集 `?builtin.a.b` 点链（Java 同样生成 Dot(BuiltIn) 嵌套——格式化器哈希访问；
/// 仅 `?string`/`?date`/`?time`/`?datetime` 的格式化器支持点参数）
pub(crate) fn dot_builtin_chain(e: &Expr) -> Option<(Box<Expr>, String, Vec<String>)> {
    match &e.kind {
        ExprKind::BuiltIn {
            target,
            name,
            args: None,
        } if matches!(name.as_str(), "string" | "date" | "time" | "datetime") => {
            Some((target.clone(), name.clone(), Vec::new()))
        }
        ExprKind::Dot { target, name } => {
            let (inner, bname, mut names) = dot_builtin_chain(target)?;
            names.push(name.clone());
            Some((inner, bname, names))
        }
        _ => None,
    }
}
