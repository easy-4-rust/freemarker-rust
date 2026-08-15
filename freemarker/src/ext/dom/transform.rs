//! DOM 变换工具 —— 对应 Java `freemarker.ext.dom.Transform`
//! （SAX/DOM 变换工具；依赖 Java XML 库，Rust 侧无等价实现——平台不可用）

/// Java 类锚点：`Transform`（依赖 Java XML 库，Rust 平台不可用）
///
/// Java `Transform` 使用 SAX/DOM 进行 XML 变换，
/// 这依赖 Java 的 javax.xml.transform API，Rust 无直接等价物。
#[allow(dead_code)]
pub(crate) struct Transform;
