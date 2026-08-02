//! 节点内建函数 —— 对应 Java `BuiltInsForNode.java`（children/parent/root/ancestors/
//! node_name/node_type/node_namespace）
//!
//! 这些内建函数操作实现了 `TemplateNodeModel` 的值，提供对 XML/HTML 等树形数据模型的导航能力。

use crate::builtins::eval_util::check_arg_count;
use crate::core::{Environment, Expr};
use crate::error::{Result, TemplateError};
use crate::template::TModel;

/// ?node_name —— 返回节点名称（元素名/属性名等）
pub fn node_name(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("node_name", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if let Some(node) = &m.node {
        let name = node.name()?;
        Ok(Some(TModel::from_scalar(name.unwrap_or_default())))
    } else {
        Err(TemplateError::type_mismatch("node", m.type_name))
    }
}

/// ?node_type —— 返回节点类型字符串（"element"/"text"/"attribute" 等）
pub fn node_type(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("node_type", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if let Some(node) = &m.node {
        let nt = node.node_type()?;
        Ok(Some(TModel::from_scalar(nt)))
    } else {
        Err(TemplateError::type_mismatch("node", m.type_name))
    }
}

/// ?node_namespace —— 返回节点命名空间 URI（无则为空串）
pub fn node_namespace(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("node_namespace", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if let Some(node) = &m.node {
        let ns = node.namespace()?;
        Ok(Some(TModel::from_scalar(ns.unwrap_or_default())))
    } else {
        Err(TemplateError::type_mismatch("node", m.type_name))
    }
}

/// ?children —— 返回子节点序列
pub fn children(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("children", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if let Some(node) = &m.node {
        let kids = node.children()?;
        Ok(Some(TModel::from_sequence(kids)))
    } else {
        Err(TemplateError::type_mismatch("node", m.type_name))
    }
}

/// ?parent —— 返回父节点；无父节点时返回 nothing
pub fn parent(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("parent", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if let Some(node) = &m.node {
        match node.parent()? {
            Some(p) => Ok(Some(p)),
            None => Ok(Some(TModel::nothing())),
        }
    } else {
        Err(TemplateError::type_mismatch("node", m.type_name))
    }
}

/// ?root —— 沿父链向上走到根节点（自身若无父节点则就是根）
pub fn root(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("root", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if m.node.is_none() {
        return Err(TemplateError::type_mismatch("node", m.type_name));
    }
    let mut current = m;
    loop {
        let parent = {
            if let Some(ref cn) = current.node {
                cn.parent()?
            } else {
                break;
            }
        };
        match parent {
            Some(p) => current = p,
            None => break,
        }
    }
    Ok(Some(current))
}

/// ?ancestors —— 返回从父节点到根节点的祖先序列，顺序为 parent, grandparent, ..., root
pub fn ancestors(
    env: &mut Environment,
    target: &Expr,
    args: Option<&[Expr]>,
) -> Result<Option<TModel>> {
    check_arg_count("ancestors", args, 0, 0)?;
    let m = crate::core::eval::eval(env, target)?;
    if m.node.is_none() {
        return Err(TemplateError::type_mismatch("node", m.type_name));
    }
    let mut result = Vec::new();
    let mut current = m;
    loop {
        let parent = {
            if let Some(ref cn) = current.node {
                cn.parent()?
            } else {
                break;
            }
        };
        match parent {
            Some(p) => {
                result.push(p.clone());
                current = p;
            }
            None => break,
        }
    }
    Ok(Some(TModel::from_sequence(result)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::TemplateNodeModel;
    use std::rc::Rc;

    /// 用于测试的简单节点模型，支持构造树形结构。
    struct TestNode {
        name: Option<String>,
        node_type: String,
        namespace: Option<String>,
        children: Vec<TModel>,
        parent: Option<TModel>,
    }

    impl TestNode {
        fn new(name: &str, node_type: &str) -> Self {
            TestNode {
                name: if name.is_empty() {
                    None
                } else {
                    Some(name.to_string())
                },
                node_type: node_type.to_string(),
                namespace: None,
                children: Vec::new(),
                parent: None,
            }
        }

        fn with_children(mut self, kids: Vec<TModel>) -> Self {
            self.children = kids;
            self
        }

        fn with_parent(mut self, p: TModel) -> Self {
            self.parent = Some(p);
            self
        }

        fn into_model(self) -> TModel {
            let mut m = TModel::nothing();
            m.node = Some(Rc::new(self));
            m.type_name = "node";
            m.kind = crate::template::ModelKind::Node;
            m
        }
    }

    impl TemplateNodeModel for TestNode {
        fn parent(&self) -> Result<Option<TModel>> {
            Ok(self.parent.clone())
        }

        fn children(&self) -> Result<Vec<TModel>> {
            Ok(self.children.clone())
        }

        fn name(&self) -> Result<Option<String>> {
            Ok(self.name.clone())
        }

        fn node_type(&self) -> Result<String> {
            Ok(self.node_type.clone())
        }

        fn namespace(&self) -> Result<Option<String>> {
            Ok(self.namespace.clone())
        }
    }

    /// 构建一棵三层树：root → child1, child2；child1 → grandchild
    fn build_test_tree() -> (TModel, TModel, TModel, TModel) {
        // 从叶子向根逐层构建（每层独立，无环引用）
        let child2 = TestNode::new("child2", "element").into_model();
        let grandchild = TestNode::new("grandchild", "element").into_model();

        // child1 含 grandchild 为子节点
        let child1 = TestNode::new("child1", "element")
            .with_children(vec![grandchild.clone()])
            .into_model();

        // root 含 child1/child2 为子节点；父引用通过 with_parent 指向 root
        let root = TestNode::new("root", "element")
            .with_children(vec![child1.clone(), child2.clone()])
            .into_model();

        // 重建子节点以设置父引用（从 root 向下传播，避免环引用）
        let child1 = TestNode::new("child1", "element")
            .with_children(vec![grandchild.clone()])
            .with_parent(root.clone())
            .into_model();
        let child2 = TestNode::new("child2", "element")
            .with_parent(root.clone())
            .into_model();
        let grandchild = TestNode::new("grandchild", "element")
            .with_parent(child1.clone())
            .into_model();

        // 重建 root 使其 children 指向带父引用的新子节点
        let root = TestNode::new("root", "element")
            .with_children(vec![child1.clone(), child2.clone()])
            .into_model();

        (root, child1, child2, grandchild)
    }

    #[test]
    fn test_node_name() {
        let (root, _, _, grandchild) = build_test_tree();
        if let Some(node) = &root.node {
            assert_eq!(node.name().unwrap(), Some("root".to_string()));
        }
        if let Some(node) = &grandchild.node {
            assert_eq!(node.name().unwrap(), Some("grandchild".to_string()));
        }
    }

    #[test]
    fn test_node_type() {
        let text_node = TestNode::new("", "text").into_model();
        let attr_node = TestNode::new("href", "attribute").into_model();
        let elem_node = TestNode::new("div", "element").into_model();

        assert_eq!(
            text_node.node.as_ref().unwrap().node_type().unwrap(),
            "text"
        );
        assert_eq!(
            attr_node.node.as_ref().unwrap().node_type().unwrap(),
            "attribute"
        );
        assert_eq!(
            elem_node.node.as_ref().unwrap().node_type().unwrap(),
            "element"
        );
    }

    #[test]
    fn test_node_namespace() {
        let node = TestNode::new("div", "element").into_model();
        // 默认 namespace 为 None
        assert_eq!(
            node.node.as_ref().unwrap().namespace().unwrap(),
            None
        );
    }

    #[test]
    fn test_children() {
        let (root, child1, child2, _grandchild) = build_test_tree();
        let kids = root.node.as_ref().unwrap().children().unwrap();
        assert_eq!(kids.len(), 2);

        // 通过名称验证子节点
        let kid_names: Vec<String> = kids
            .iter()
            .map(|k| k.node.as_ref().unwrap().name().unwrap().unwrap_or_default())
            .collect();
        assert_eq!(kid_names, vec!["child1", "child2"]);

        // child1 只有一个子节点 grandchild
        let gc = child1.node.as_ref().unwrap().children().unwrap();
        assert_eq!(gc.len(), 1);

        // child2 没有子节点
        let empty = child2.node.as_ref().unwrap().children().unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_parent() {
        let (_root, child1, _child2, grandchild) = build_test_tree();

        // grandchild 的父节点是 child1
        let parent = grandchild.node.as_ref().unwrap().parent().unwrap();
        assert!(parent.is_some());
        assert_eq!(
            parent
                .unwrap()
                .node
                .as_ref()
                .unwrap()
                .name()
                .unwrap()
                .unwrap(),
            "child1"
        );

        // child1 的父节点是 root
        let parent = child1.node.as_ref().unwrap().parent().unwrap();
        assert!(parent.is_some());
        assert_eq!(
            parent
                .unwrap()
                .node
                .as_ref()
                .unwrap()
                .name()
                .unwrap()
                .unwrap(),
            "root"
        );
    }

    #[test]
    fn test_root_from_leaf() {
        let (_root, _child1, _child2, grandchild) = build_test_tree();

        // 从叶子节点出发，沿父链走到根
        let mut current = grandchild.clone();
        loop {
            let parent = {
                if let Some(ref cn) = current.node {
                    cn.parent().unwrap()
                } else {
                    break;
                }
            };
            match parent {
                Some(p) => current = p,
                None => break,
            }
        }
        assert_eq!(
            current.node.as_ref().unwrap().name().unwrap().unwrap(),
            "root"
        );
    }

    #[test]
    fn test_root_is_self_when_no_parent() {
        let (root, _child1, _child2, _grandchild) = build_test_tree();

        // root 没有父节点
        let parent = root.node.as_ref().unwrap().parent().unwrap();
        assert!(parent.is_none());
    }

    #[test]
    fn test_ancestors_order() {
        let (_root, _child1, _child2, grandchild) = build_test_tree();

        // 从 grandchild 出发收集祖先：顺序应为 parent → grandparent → ... → root
        let mut ancestors = Vec::new();
        let mut current = grandchild.clone();
        loop {
            let parent = {
                if let Some(ref cn) = current.node {
                    cn.parent().unwrap()
                } else {
                    break;
                }
            };
            match parent {
                Some(p) => {
                    let name = p
                        .node
                        .as_ref()
                        .unwrap()
                        .name()
                        .unwrap()
                        .unwrap_or_default();
                    ancestors.push(name);
                    current = p;
                }
                None => break,
            }
        }
        assert_eq!(ancestors, vec!["child1", "root"]);
    }

    #[test]
    fn test_ancestors_of_root_is_empty() {
        let (root, _child1, _child2, _grandchild) = build_test_tree();

        // root 没有祖先
        let parent = root.node.as_ref().unwrap().parent().unwrap();
        assert!(parent.is_none());
    }
}
