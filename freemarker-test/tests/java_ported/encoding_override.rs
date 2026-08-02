//! Java `freemarker.core.EncodingOverrideTest` —— 跳过（空 mod）
//! （对应 Java: EncodingOverrideTest —— `<#ftl encoding=...>` / 文件头编码
//! 覆盖机制）
//!
//! 不可移植原因：Java 该测试依赖模板文件的实际字节编码（encodingOverride-*.ftl
//! 资源文件按 Latin-1/UTF-8 保存）与 getTemplate→Template.getEncoding 的编码
//! 重读机制（WrongEncodingException → 换编码重读）；v1 引擎模板源统一为
//! UTF-8 字符串（StringLoader），无字节级编码覆盖机制 —— 无对应测试可翻译。
