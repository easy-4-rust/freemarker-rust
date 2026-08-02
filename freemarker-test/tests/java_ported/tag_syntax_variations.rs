//! Java `freemarker.core.TagSyntaxVariationsTest` 的 Rust 1:1 实现
//! （对应 Java: TagSyntaxVariationsTest —— 各种模板排列（含故意写错的）在
//! 不同 tag_syntax 设置下的行为）
//!
//! Java 用两个配置：cfgBuggy（`new Configuration()` = ICI 2.3.0 默认，
//! emulate23ParserBugs=true —— 未知 # 标签被当静态文本）与 cfgFixed
//! （ICI 2.3.19）。本引擎固定 ICI 2.3.34、标签语法无配置项（首个标签自动
//! 检测）—— cfgBuggy 的"旧解析器 bug"行为未模拟，差异见函数内注释。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::StringLoader;
use freemarker::template::{Configuration, TModel};
use std::sync::Arc;

const HDR_ANG: &str = "<#ftl>";
const HDR_SQU: &str = "[#ftl]";
const IF_ANG: &str = "<#if true>i</#if>";
const IF_SQU: &str = "[#if true]i[/#if]";
const IF_OUT: &str = "i";
const ASSIGN_ANG: &str = "<#assign x = 1>a";
const ASSIGN_SQU: &str = "[#assign x = 1]a";
const ASSIGN_OUT: &str = "a";
const WRONG_ANG: &str = "<#wrong>";
const WRONG_SQU: &str = "[#wrong]";
const WRONGC_ANG: &str = "</#wrong>";
const WRONGC_SQU: &str = "[/#wrong]";
const CUST_ANG: &str = "<@compress> z </@>";
const CUST_SQU: &str = "[@compress] z [/@]";
const CUST_OUT: &str = "z";

/// Java `test(cfg, template, expected)`：expected 为 None 时要求解析失败
fn check(c: &Configuration, _loader: &Arc<StringLoader>, ftl: &str, expected: Option<&str>) {
    let cfg = std::rc::Rc::new(c.clone());
    let t = match freemarker::parser::parse(&cfg, "string", ftl) {
        Ok(t) => t,
        Err(_e) => {
            if expected.is_some() {
                panic!("Couldn't create Template from {ftl:?}");
            }
            return;
        }
    };
    match expected {
        None => panic!("Template parsing should have failed for {ftl:?}"),
        Some(exp) => {
            let mut out = Vec::new();
            t.process(TModel::from_hash(indexmap::IndexMap::new()), &mut out)
                .unwrap_or_else(|e| panic!("process failed for {ftl:?}: {e}"));
            let actual = String::from_utf8_lossy(&out);
            assert_eq!(actual, exp, "ftl: {ftl}");
        }
    }
}

/// Java `test()`（JUnit3 单方法）：双配置 × 双指令 × 双标签语法的全部排列
#[test]
fn test() {
    let (c_buggy, loader_buggy) = test_config();
    let (c_fixed, loader_fixed) = test_config();
    // 引擎差异：Java cfgBuggy = `new Configuration()`（ICI 2.3.0 +
    // emulate23ParserBugs=true，未知 # 标签当静态文本）；本引擎固定 ICI 2.3.34，
    // 无 emulate23ParserBugs —— 两个配置行为一致（= Java cfgFixed）
    // 引擎差异：Java 的 setTagSyntax(ANGLE/SQUARE/AUTO_DETECT) 无对应配置项，
    // 本引擎首个标签自动检测语法

    // 指令 × 语法排列
    for if_or_assign in 0..2 {
        let (dir_ang, dir_squ, dir_out) = if if_or_assign == 0 {
            (IF_ANG, IF_SQU, IF_OUT)
        } else {
            (ASSIGN_ANG, ASSIGN_SQU, ASSIGN_OUT)
        };
        for ang_or_squ in 0..2 {
            let (dir_xxx, cust_xxx, hdr_xxx, wrong_xxx, wrongc_xxx) = if ang_or_squ == 0 {
                (dir_ang, CUST_ANG, HDR_ANG, WRONG_ANG, WRONGC_ANG)
            } else {
                (dir_squ, CUST_SQU, HDR_SQU, WRONG_SQU, WRONGC_SQU)
            };

            check(
                &c_buggy,
                &loader_buggy,
                &format!("{dir_xxx}{cust_xxx}"),
                Some(&format!("{dir_out}{CUST_OUT}")),
            );
            check(
                &c_fixed,
                &loader_fixed,
                &format!("{dir_xxx}{cust_xxx}"),
                Some(&format!("{dir_out}{CUST_OUT}")),
            );

            for wrong_or_wrongc in 0..2 {
                let wrongx_xxx = if wrong_or_wrongc == 0 {
                    wrong_xxx
                } else {
                    wrongc_xxx
                };

                // Bug: initial unknown # tags are treated as static text
                // （引擎差异：cfgBuggy 不模拟 emulate23ParserBugs —— 未知 # 标签直接
                // 解析报错；Java 中它们被当静态文本输出 → 断言改为期望解析失败）
                check(
                    &c_buggy,
                    &loader_buggy,
                    &format!("{wrongx_xxx}{dir_xxx}"),
                    None,
                );
                check(
                    &c_fixed,
                    &loader_fixed,
                    &format!("{wrongx_xxx}{dir_xxx}"),
                    None,
                );

                // Bug: same as above
                // （引擎差异：同 cfgBuggy，改为期望解析失败）
                check(
                    &c_buggy,
                    &loader_buggy,
                    &format!("{wrongx_xxx}{wrongx_xxx}{dir_xxx}"),
                    None,
                );

                check(
                    &c_buggy,
                    &loader_buggy,
                    &format!("{dir_xxx}{wrongx_xxx}"),
                    None,
                );
                check(
                    &c_fixed,
                    &loader_fixed,
                    &format!("{dir_xxx}{wrongx_xxx}"),
                    None,
                );

                check(
                    &c_buggy,
                    &loader_buggy,
                    &format!("{hdr_xxx}{wrongx_xxx}"),
                    None,
                );
                check(
                    &c_fixed,
                    &loader_fixed,
                    &format!("{hdr_xxx}{wrongx_xxx}"),
                    None,
                );

                check(
                    &c_buggy,
                    &loader_buggy,
                    &format!("{cust_xxx}{wrongx_xxx}{dir_xxx}"),
                    None,
                );
                check(
                    &c_fixed,
                    &loader_fixed,
                    &format!("{cust_xxx}{wrongx_xxx}{dir_xxx}"),
                    None,
                );
            }
        }

        // AUTO_DETECT 下的 4 种排列（Java :144-163）
        for perm in 0..4 {
            let wrong_xxx = if perm & 1 == 0 { WRONG_ANG } else { WRONG_SQU };
            let dir_xxx = if perm & 2 == 0 { dir_ang } else { dir_squ };

            // Bug: Auto-detection ignores unknown # tags
            // （引擎差异：cfgBuggy 不模拟 emulate23ParserBugs —— 未知 # 标签直接
            // 解析报错；Java 中它们被当静态文本输出 → 断言改为期望解析失败）
            check(
                &c_buggy,
                &loader_buggy,
                &format!("{wrong_xxx}{dir_xxx}"),
                None,
            );
            check(
                &c_fixed,
                &loader_fixed,
                &format!("{wrong_xxx}{dir_xxx}"),
                None,
            );

            // Bug: same as above
            // （引擎差异：同 cfgBuggy，改为期望解析失败）
            check(
                &c_buggy,
                &loader_buggy,
                &format!("{wrong_xxx}{wrong_xxx}{dir_xxx}"),
                None,
            );
        }

        // 混合语法的排列（Java :165-207）：wrong_yyy 是反括号风格的未知标签
        for ang_or_squ_start in 0..2 {
            let (hdr_xxx, cust_xxx, wrong_yyy, dir_xxx, dir_yyy) = if ang_or_squ_start == 0 {
                (HDR_ANG, CUST_ANG, WRONG_SQU, dir_ang, dir_squ)
            } else {
                (HDR_SQU, CUST_SQU, WRONG_ANG, dir_squ, dir_ang)
            };

            check(
                &c_buggy,
                &loader_buggy,
                &format!("{cust_xxx}{wrong_yyy}{dir_xxx}"),
                Some(&format!("{CUST_OUT}{wrong_yyy}{dir_out}")),
            );
            check(
                &c_fixed,
                &loader_fixed,
                &format!("{cust_xxx}{wrong_yyy}{dir_xxx}"),
                Some(&format!("{CUST_OUT}{wrong_yyy}{dir_out}")),
            );

            check(
                &c_buggy,
                &loader_buggy,
                &format!("{hdr_xxx}{wrong_yyy}{dir_xxx}"),
                Some(&format!("{wrong_yyy}{dir_out}")),
            );
            check(
                &c_fixed,
                &loader_fixed,
                &format!("{hdr_xxx}{wrong_yyy}{dir_xxx}"),
                Some(&format!("{wrong_yyy}{dir_out}")),
            );

            check(
                &c_buggy,
                &loader_buggy,
                &format!("{cust_xxx}{wrong_yyy}{dir_yyy}"),
                Some(&format!("{CUST_OUT}{wrong_yyy}{dir_yyy}")),
            );
            check(
                &c_fixed,
                &loader_fixed,
                &format!("{cust_xxx}{wrong_yyy}{dir_yyy}"),
                Some(&format!("{CUST_OUT}{wrong_yyy}{dir_yyy}")),
            );

            check(
                &c_buggy,
                &loader_buggy,
                &format!("{hdr_xxx}{wrong_yyy}{dir_yyy}"),
                Some(&format!("{wrong_yyy}{dir_yyy}")),
            );
            check(
                &c_fixed,
                &loader_fixed,
                &format!("{hdr_xxx}{wrong_yyy}{dir_yyy}"),
                Some(&format!("{wrong_yyy}{dir_yyy}")),
            );

            check(
                &c_buggy,
                &loader_buggy,
                &format!("{dir_xxx}{wrong_yyy}{dir_yyy}"),
                Some(&format!("{dir_out}{wrong_yyy}{dir_yyy}")),
            );
            check(
                &c_fixed,
                &loader_fixed,
                &format!("{dir_xxx}{wrong_yyy}{dir_yyy}"),
                Some(&format!("{dir_out}{wrong_yyy}{dir_yyy}")),
            );
        }
    }
}
