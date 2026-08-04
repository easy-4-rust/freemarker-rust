//! 对应 Java `freemarker.template.TemplateTransformModel`
//! （一文件一 Java 对象拆分：原 template_model.rs 合并存放 → 独立文件）

use crate::core::Environment;
use crate::error::Result;
use crate::template::TModel;

/// 变换模型 —— 对应 Java `TemplateTransformModel`（`<#transform>` 旧式指令目标
/// 与 `<@transformModel/>` 调用）。Java 语义（Environment.visitAndTransform
/// :495-543）：`getWriter(out, args)` 先产出变换自身输出（?interpret 即
/// env.include 解释模板），随后调用方把 body 写入变换 writer
/// （Interpret.TemplateProcessorModel.getWriter 返回透传 writer，body 原样输出；
/// StandardCompress 等在 close 时对 body 做压缩/转义）。
pub trait TemplateTransformModel {
    /// 变换自身输出（Java getWriter 阶段；v1 无 writer 对象，变换直接向 env 输出）
    fn transform(&self, env: &mut Environment) -> Result<()> {
        let _ = env;
        Ok(())
    }

    /// 带 body 的变换 —— 对应 Java `visitAndTransform` 的完整流程
    /// （Environment.java:495-543）：getWriter(out, args) → body 写入变换 writer →
    /// close 时变换输出。v1：捕获 body 渲染文本 → 变换 → emit。
    /// 返回 body 渲染的流控信号（`<#return>` 值 / 完成）。
    fn transform_with_body(
        &self,
        env: &mut Environment,
        params: &std::collections::HashMap<String, TModel>,
        body: &[crate::core::Element],
    ) -> Result<crate::core::environment::RunSignal> {
        let _ = params;
        self.transform(env)?;
        env.run(body)
    }
}
