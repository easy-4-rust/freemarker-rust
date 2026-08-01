//! 模板名称格式 —— 对应 Java `freemarker.cache.TemplateNameFormat`
//! （Default020400 的 toRootBasedName/normalizeRootBasedName 各步骤逐行对照
//!   TemplateNameFormat.java:234-452；Default020300 简化兼容版对应 :150-230）
//! v1 未纳入 `rootBasedNameToAbsoluteName`（Java:148, :215-223, :455-463）。

use crate::error::{Result, TemplateError};

/// 模板名称格式 trait（对应 TemplateNameFormat 抽象类；API 尚未公开给用户自定义）
pub trait TemplateNameFormat {
    /// 对应 `toRootBasedName(baseName, targetName)`（Java:122）：把引用名换算为
    /// 相对模板根的路径（base 为引用方模板名，目录名以 "/" 结尾）
    fn to_root_based_name(&self, base_name: &str, target_name: &str) -> Result<String>;
    /// 对应 `normalizeRootBasedName(name)`（Java:138）：规范化根相对名，
    /// 使等价名称字符串相等（"sub/../t.ftl"→"t.ftl"、"/t.ftl"→"t.ftl"）
    fn normalize_root_based_name(&self, name: &str) -> Result<String>;
}

/// 默认名称格式（对应 `TemplateNameFormat.DEFAULT_2_4_0`，Java:100, :232-469）
pub struct Default020400;

impl TemplateNameFormat for Default020400 {
    /// 对应 `Default020400.toRootBasedName`（Java:234-258）
    fn to_root_based_name(&self, base_name: &str, target_name: &str) -> Result<String> {
        if find_scheme_section_end(target_name) != 0 {
            // Java:235-236 —— 目标名自带 scheme → 原样返回
            return Ok(target_name.to_string());
        }
        if let Some(target_relative) = target_name.strip_prefix('/') {
            // Java:237-245 —— 目标为绝对路径：去掉前导 "/"；base 带 scheme 则保留 scheme
            let scheme_end = find_scheme_section_end(base_name);
            if scheme_end == 0 {
                Ok(target_relative.to_string())
            } else {
                Ok(format!("{}{}", &base_name[..scheme_end], target_relative))
            }
        } else {
            // Java:246-257 —— 相对路径：以 base 所在目录为基准拼接
            let mut base = base_name;
            if !base.ends_with('/') {
                // 不是目录名 → 取所在目录（Java:247-249）
                let base_end = base.rfind('/').map_or(0, |i| i + 1);
                if base_end == 0 {
                    // 形如 "classpath:t.ftl"：不能去掉 scheme 部分（Java:250-253）
                    base = &base_name[..find_scheme_section_end(base_name)];
                } else {
                    base = &base_name[..base_end];
                }
            }
            Ok(format!("{}{}", base, target_name))
        }
    }

    /// 对应 `Default020400.normalizeRootBasedName`（Java:261-301）
    fn normalize_root_based_name(&self, name: &str) -> Result<String> {
        // Java:263 —— 禁止 NUL 字符（安全）
        check_name_has_no_null_character(name)?;
        // Java:265-269 —— 禁止反斜杠
        if name.contains('\\') {
            return Err(TemplateError::misc(format!(
                "Template name \"{}\" is not valid. Backslash (\"\\\") is not allowed in template names. Use slash (\"/\") instead.",
                name
            )));
        }
        // Java:271-283 —— 拆 scheme 与 path
        let scheme_end = find_scheme_section_end(name);
        let (scheme, path) = if scheme_end == 0 {
            (None, name.to_string())
        } else {
            (
                Some(name[..scheme_end].to_string()),
                name[scheme_end..].to_string(),
            )
        };
        // Java:285-289 —— path 部分禁止 ":"（仅 scheme 分隔符可用）
        if path.contains(':') {
            return Err(TemplateError::misc(format!(
                "Template name \"{}\" is not valid. The ':' character can only be used after the scheme name (if there's any), not in the path part",
                name
            )));
        }
        // 四个步骤，顺序与 Java:291-298 一致
        let mut path = remove_redundant_slashes(&path);
        path = remove_dot_steps(&path);
        path = resolve_dot_dot_steps(&path, name)?;
        path = remove_redundant_star_steps(&path);
        Ok(match scheme {
            Some(s) => format!("{}{}", s, path),
            None => path,
        })
    }
}

/// 简化兼容版名称格式（对应 `TemplateNameFormat.DEFAULT_2_3_0`，Java:46, :150-230）
/// 语义差异：不处理 scheme（仅 "://" 前缀）、不去冗余斜杠、只处理 "/../" 与 "/./" 序列
pub struct Default020300;

impl TemplateNameFormat for Default020300 {
    /// 对应 `Default020300.toRootBasedName`（Java:152-168）
    fn to_root_based_name(&self, base_name: &str, target_name: &str) -> Result<String> {
        // Java:153-154 —— 目标名含 "://"（位置 > 0）→ 视为 scheme 名，原样返回
        if let Some(idx) = target_name.find("://") {
            if idx > 0 {
                return Ok(target_name.to_string());
            }
        }
        if target_name.starts_with('/') {
            // Java:155-161 —— 绝对路径：base 有 "://" scheme 则保留 scheme 前缀，否则去前导 "/"
            match base_name.find("://") {
                Some(scheme_sep_idx) if scheme_sep_idx > 0 => Ok(format!(
                    "{}{}",
                    &base_name[..scheme_sep_idx + 2],
                    target_name
                )),
                _ => Ok(target_name
                    .strip_prefix('/')
                    .unwrap_or(target_name)
                    .to_string()),
            }
        } else {
            // Java:162-167 —— 相对路径：以 base 所在目录为基准拼接
            let base = if base_name.ends_with('/') {
                base_name
            } else {
                &base_name[..base_name.rfind('/').map_or(0, |i| i + 1)]
            };
            Ok(format!("{}{}", base, target_name))
        }
    }

    /// 对应 `Default020300.normalizeRootBasedName`（Java:171-212）
    fn normalize_root_based_name(&self, name: &str) -> Result<String> {
        // Java:173 —— 禁止 NUL 字符（安全）
        check_name_has_no_null_character(name)?;
        let mut path = name.to_string();
        // Java:179-195 —— 循环处理 "/../"（移除前一步骤；开头 "/../" 越界）
        loop {
            let Some(parent_dir_loc) = path.find("/../") else {
                // Java:186-190 —— 无 "/../"：以 "../" 开头 → 越出模板根
                if path.starts_with("../") {
                    return Err(root_leaving_error(name));
                }
                break;
            };
            if parent_dir_loc == 0 {
                return Err(root_leaving_error(name));
            }
            // Java:192 —— lastIndexOf('/', parentDirPathLoc-1)；字节扫描避免 UTF-8 边界问题
            let previous_slash_loc = path.as_bytes()[..parent_dir_loc - 1]
                .iter()
                .rposition(|&b| b == b'/');
            // Java:193-194 —— substring(0, previousSlashLoc+1) + substring(parentDirPathLoc + 4)
            path = format!(
                "{}{}",
                &path[..previous_slash_loc.map_or(0, |i| i + 1)],
                &path[parent_dir_loc + 4..]
            );
        }
        // Java:196-206 —— 循环处理 "/./" 与开头 "./"
        loop {
            let Some(current_dir_loc) = path.find("/./") else {
                // Java:199-201
                if path.starts_with("./") {
                    path = path[2..].to_string();
                }
                break;
            };
            // Java:204-205 —— substring(0, loc) + substring(loc + 3 - 1)
            path = format!(
                "{}{}",
                &path[..current_dir_loc],
                &path[current_dir_loc + 2..]
            );
        }
        // Java:207-210 —— 编辑后可能残留前导 "/"，去掉（仅长度 > 1，故 "/" 根保留）
        if path.len() > 1 && path.starts_with('/') {
            path = path[1..].to_string();
        }
        Ok(path)
    }
}

/// 对应 `findSchemeSectionEnd`（Java:303-316）：
/// 首个 ":" 前无 "/" 视为 scheme 分隔；后随 "//" 则一并归入 scheme 段
fn find_scheme_section_end(name: &str) -> usize {
    let Some(colon_idx) = name.find(':') else {
        return 0;
    };
    if name[..colon_idx].contains('/') {
        return 0;
    }
    let bytes = name.as_bytes();
    if colon_idx + 2 < bytes.len() && bytes[colon_idx + 1] == b'/' && bytes[colon_idx + 2] == b'/' {
        colon_idx + 3
    } else {
        colon_idx + 1
    }
}

/// 对应 `removeRedundantSlashes`（Java:318-325）：
/// 反复把 "//" → "/"，再去掉前导 "/"（path 不再以 "/" 开头）
fn remove_redundant_slashes(path: &str) -> String {
    let mut cur = path.to_string();
    loop {
        let prev = cur.clone();
        cur = prev.replace("//", "/");
        if cur == prev {
            break;
        }
    }
    if let Some(rest) = cur.strip_prefix('/') {
        rest.to_string()
    } else {
        cur
    }
}

/// 对应 `removeDotSteps`（Java:327-357）：从右向左移除 "." 步骤。
/// "foo/./bar"→"foo/bar"、"./bar"→"bar"、末尾 "foo/."→"foo/"（Java:353-354
/// 只剥末位 "." 保留 "/"）、"."→""；
/// 注意末尾 ".." 的两个 "." 会因左侧假警报被跳过，原样留给 resolveDotDotSteps 处理。
/// 游标必须用有符号整数：Java:334 `nextFromIdx = dotIdx - 1`，首字符假警报时
/// 变为 -1，`lastIndexOf('.', -1)`（Java:330）返回 -1 结束扫描；用 usize 的
/// saturating_sub 会把 -1 钳成 0，导致前导假警报（如 "**/c"、"x."）死循环
fn remove_dot_steps(path: &str) -> String {
    let mut path = path.to_string();
    let mut next_from_idx: i64 = path.len() as i64 - 1;
    loop {
        // Java:330-332 —— nextFromIdx < 0（含 lastIndexOf 语义）→ 无更多 "." → 结束
        if next_from_idx < 0 {
            return path;
        }
        // 字节扫描避免 UTF-8 边界问题（"." 为 ASCII）
        let Some(dot_idx) = path.as_bytes()[..=next_from_idx as usize]
            .iter()
            .rposition(|&b| b == b'.')
        else {
            return path;
        };
        next_from_idx = dot_idx as i64 - 1;
        // Java:336-339 —— 假警报：左侧不是步骤边界
        if dot_idx != 0 && path.as_bytes()[dot_idx - 1] != b'/' {
            continue;
        }
        // Java:341-349 —— 右侧必须是 "/" 或字符串末尾，否则假警报
        let slash_right = if dot_idx + 1 == path.len() {
            false
        } else if path.as_bytes()[dot_idx + 1] == b'/' {
            true
        } else {
            continue;
        };
        if slash_right {
            // Java:351-352 —— "foo/./bar" 或 "./bar"
            path = format!("{}{}", &path[..dot_idx], &path[dot_idx + 2..]);
        } else {
            // Java:353-354 —— "foo/." 或 "."
            path = path[..path.len() - 1].to_string();
        }
    }
}

/// 对应 `resolveDotDotSteps`（Java:362-422）：解析 ".." 步骤（移除前一步骤）。
/// 越出根目录（无前一步骤可移除）→ 报错；`name` 为原始名（错误消息用）
fn resolve_dot_dot_steps(path: &str, name: &str) -> Result<String> {
    let mut path = path.to_string();
    let mut next_from_idx = 0usize;
    loop {
        // Java:365 —— path.indexOf("..", nextFromIdx)
        let Some(rel) = path.as_bytes()[next_from_idx..]
            .windows(2)
            .position(|w| w == b"..")
        else {
            return Ok(path);
        };
        let dot_dot_idx = next_from_idx + rel;
        // Java:370-371 —— 开头 ".." → 越出根
        if dot_dot_idx == 0 {
            return Err(root_leaving_error(name));
        }
        // Java:372-376 —— 假警报：左侧不是 "/"
        if path.as_bytes()[dot_dot_idx - 1] != b'/' {
            next_from_idx = dot_dot_idx + 3;
            continue;
        }
        // Java:379-388 —— 右侧必须是 "/" 或字符串末尾，否则假警报
        let bytes = path.as_bytes();
        let slash_right = if dot_dot_idx + 2 == bytes.len() {
            false
        } else if bytes[dot_dot_idx + 2] == b'/' {
            true
        } else {
            next_from_idx = dot_dot_idx + 3;
            continue;
        };
        // Java:390-413 —— 向后找 ".." 之前最近的步骤分隔符（跳过 "*" 步骤）
        let mut search_backwards_from = dot_dot_idx.saturating_sub(2);
        let mut skipped_star_step = false;
        let previous_slash_idx: Option<usize> = loop {
            if search_backwards_from == usize::MAX {
                // Java:395-397 —— 已回溯到字符串开头之前 → 越出根
                return Err(root_leaving_error(name));
            }
            let Some(ps) = path.as_bytes()[..=search_backwards_from]
                .iter()
                .rposition(|&b| b == b'/')
            else {
                // Java:399-404 —— 未找到 "/"：若是 "*/.." → 越出根，否则结束回溯（previousSlashIdx=-1）
                if search_backwards_from == 0 && path.as_bytes()[0] == b'*' {
                    return Err(root_leaving_error(name));
                }
                break None;
            };
            // Java:406-411 —— 紧邻的 "*" 步骤被 ".." 越过：记录并继续回溯
            if ps + 2 < path.len()
                && path.as_bytes()[ps + 1] == b'*'
                && path.as_bytes()[ps + 2] == b'/'
            {
                skipped_star_step = true;
                search_backwards_from = ps.saturating_sub(1);
            } else {
                break Some(ps);
            }
        };
        // Java:415-420 —— 移除 "a/{b/*/../}c" 部分
        let prev_end = previous_slash_idx.map_or(0, |i| i + 1);
        path = format!(
            "{}{}{}",
            &path[..prev_end],
            if skipped_star_step { "*/" } else { "" },
            &path[dot_dot_idx + if slash_right { 3 } else { 2 }..]
        );
        next_from_idx = prev_end;
    }
}

/// 对应 `removeRedundantStarSteps`（Java:424-452）：连续 "*" 步骤合并为一个，
/// 前导 "*" 步骤冗余（"*/*/c"→"c"）；非步骤形式的 "*"（如 "**/c"）不动
fn remove_redundant_star_steps(path: &str) -> String {
    let mut path = path.to_string();
    // Java:426-439 —— 合并 "*/*"
    while let Some(idx) = path.find("*/*") {
        let prev = path.clone();
        let delimited = (idx == 0 || path.as_bytes()[idx - 1] == b'/')
            && (idx + 3 == path.len() || path.as_bytes()[idx + 3] == b'/');
        if delimited {
            path = format!("{}{}", &path[..idx], &path[idx + 2..]);
        }
        if path == prev {
            break;
        }
    }
    // Java:441-449 —— 前导 "*" 步骤冗余
    if path.starts_with('*') {
        if path.len() == 1 {
            path.clear();
        } else if path.as_bytes()[1] == b'/' {
            path = path[2..].to_string();
        }
    }
    path
}

/// 对应 `checkNameHasNoNullCharacter`（Java:471-476）
fn check_name_has_no_null_character(name: &str) -> Result<()> {
    if name.contains('\0') {
        return Err(TemplateError::misc(format!(
            "Template name \"{}\" is not valid. Null character (\\u0000) in the name; possible attack attempt",
            name
        )));
    }
    Ok(())
}

/// 越出模板根目录错误（对应 Java `newRootLeavingException`，Java:478-480；
/// 消息按项目约定风格：`Template name "..." is not valid. It doesn't stay within the template root directory.`）
fn root_leaving_error(name: &str) -> TemplateError {
    TemplateError::misc(format!(
        "Template name \"{}\" is not valid. It doesn't stay within the template root directory.",
        name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default020400_normalization_matrix() {
        let f = Default020400;
        let cases: &[(&str, &str)] = &[
            // 冗余斜杠（Java:82-84 文档示例："foo//bar///baaz.ftl"→"foo/bar/baaz.ftl"）
            ("a//b", "a/b"),
            ("a//b///c", "a/b/c"),
            // "." 步骤（Java:86-94）
            ("a/./b", "a/b"),
            ("./a", "a"),
            // 末尾 "."：Java:353-354 只剥掉末位 "."，保留前导 "/"（"a/."→"a/"）
            ("a/.", "a/"),
            (".", ""),
            // ".." 步骤（Java:86-94）：旧版 bug 修复示例
            ("a/../b", "b"),
            ("a/b/../c", "a/c"),
            ("a/./../c", "c"),
            // "*" 步骤参与 ".." 解析（Java:88-89 文档示例）
            ("a/b/*/../c", "a/*/c"),
            // 尾部 ".." 保留目录语义（Java:94 文档示例）
            ("a/b/..", "a/"),
            ("foo/bar/..", "foo/"),
            // 连续 "*" 步骤合并（Java:96-97）
            ("*/*/c", "c"),
            ("a/*/*/b", "a/*/b"),
            ("**/c", "**/c"),
            // 前导/内部假警报 "."（回归：Java:334 nextFromIdx=-1 结束扫描；
            // saturating_sub 曾使 "**/c" 死循环）—— 均原样保留
            ("x.", "x."),
            ("a..b/c", "a..b/c"),
            // 根与空名（Java:76-84）："/" 经 removeRedundantSlashes（Java:324 无条件
            // 去前导 "/"）→ ""；与 Default020300 的 length>1 守卫（:208）不同
            ("/x", "x"),
            ("/", ""),
            ("", ""),
            // scheme（Java:57-68 文档示例）
            ("classpath:foo.ftl", "classpath:foo.ftl"),
            ("classpath://x", "classpath://x"),
            ("myscheme:/x", "myscheme:x"),
            ("myscheme:///x", "myscheme://x"),
            // 前缀含 ":" 且 ":" 前无 "/" → 整个前缀是 scheme，path 无 ":" → 合法
            // （Java 测试 TemplateNameFormatTest:214-218 "s:a/b"、"s:/a/b" 同理）
            ("a:b.ftl", "a:b.ftl"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                f.normalize_root_based_name(input).unwrap(),
                *expected,
                "input: {input}"
            );
        }
    }

    #[test]
    fn default020400_rejects_escape_and_malformed() {
        let f = Default020400;
        // 越出根目录（Java:370-371, :395-397, :400-403）
        let e = f.normalize_root_based_name("../x").unwrap_err();
        assert_eq!(
            e.to_user_message(),
            "Template name \"../x\" is not valid. It doesn't stay within the template root directory."
        );
        let e = f.normalize_root_based_name("/../x").unwrap_err();
        assert!(e
            .to_user_message()
            .contains("doesn't stay within the template root directory"));
        let e = f.normalize_root_based_name("a/b/../../..").unwrap_err();
        assert!(e
            .to_user_message()
            .contains("doesn't stay within the template root directory"));
        let e = f.normalize_root_based_name("*/..").unwrap_err();
        assert!(e
            .to_user_message()
            .contains("doesn't stay within the template root directory"));
        // 反斜杠（Java:265-269）
        let e = f.normalize_root_based_name("a\\b.ftl").unwrap_err();
        assert_eq!(
            e.to_user_message(),
            "Template name \"a\\b.ftl\" is not valid. Backslash (\"\\\") is not allowed in template names. Use slash (\"/\") instead."
        );
        // path 中 ":"（Java:285-289）—— 需 ":" 出现在 scheme 段之外：
        // 前缀 "a/b" 含 "/"，findSchemeSectionEnd 判无 scheme，整个名字是 path → 含 ":" 报错
        let e = f.normalize_root_based_name("a/b:c.ftl").unwrap_err();
        assert!(e
            .to_user_message()
            .contains("The ':' character can only be used after the scheme name"));
        // "a:b.ftl" 的 ":" 前无 "/" → 是 scheme 分隔符而非非法字符（与 Java 测试
        // TemplateNameFormatTest:214-218 "s:a/b" 一致），应正常规范化
        assert_eq!(f.normalize_root_based_name("a:b.ftl").unwrap(), "a:b.ftl");
        // NUL 字符（Java:263, :471-476）
        let e = f.normalize_root_based_name("x\0y").unwrap_err();
        assert!(e.to_user_message().contains("Null character"));
    }

    #[test]
    fn default020300_normalization_compat() {
        let f = Default020300;
        let cases: &[(&str, &str)] = &[
            ("a/../b", "b"),
            ("a/b/../c", "a/c"),
            ("/a/../b", "b"),
            ("a/./b", "a/b"),
            ("./a", "a"),
            ("a/b/./c", "a/b/c"),
            ("/x", "x"),
            // 020300 不去冗余斜杠（兼容旧版）
            ("a//b", "a//b"),
            // 根 "/" 保留（长度 1 不剥离）
            ("/", "/"),
            ("", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(
                f.normalize_root_based_name(input).unwrap(),
                *expected,
                "input: {input}"
            );
        }
        // 越出根（Java:181-184, :187-190）
        let e = f.normalize_root_based_name("../x").unwrap_err();
        assert!(e
            .to_user_message()
            .contains("doesn't stay within the template root directory"));
        let e = f.normalize_root_based_name("/../x").unwrap_err();
        assert!(e
            .to_user_message()
            .contains("doesn't stay within the template root directory"));
    }

    #[test]
    fn default020400_to_root_based_name() {
        let f = Default020400;
        // 相对引用：以 base 所在目录为基准（Java:246-257）
        assert_eq!(
            f.to_root_based_name("sub/context.ftl", "t.ftl").unwrap(),
            "sub/t.ftl"
        );
        assert_eq!(
            f.to_root_based_name("sub/context.ftl", "a/b.ftl").unwrap(),
            "sub/a/b.ftl"
        );
        assert_eq!(f.to_root_based_name("sub/", "t.ftl").unwrap(), "sub/t.ftl");
        assert_eq!(f.to_root_based_name("t.ftl", "x.ftl").unwrap(), "x.ftl");
        // 绝对引用：去前导 "/"；base 带 scheme 则保留（Java:237-245）
        assert_eq!(
            f.to_root_based_name("sub/context.ftl", "/t.ftl").unwrap(),
            "t.ftl"
        );
        // scheme 处理（Java:249-253）："classpath:t.ftl" 的目录基准保留 scheme
        assert_eq!(
            f.to_root_based_name("classpath:sub/context.ftl", "t.ftl")
                .unwrap(),
            "classpath:sub/t.ftl"
        );
        assert_eq!(
            f.to_root_based_name("classpath:context.ftl", "t.ftl")
                .unwrap(),
            "classpath:t.ftl"
        );
        // 目标自带 scheme → 原样（Java:235-236）
        assert_eq!(
            f.to_root_based_name("sub/context.ftl", "other:foo.ftl")
                .unwrap(),
            "other:foo.ftl"
        );
    }

    #[test]
    fn default020300_to_root_based_name() {
        let f = Default020300;
        // 相对引用（Java:162-167）
        assert_eq!(
            f.to_root_based_name("sub/context.ftl", "t.ftl").unwrap(),
            "sub/t.ftl"
        );
        // 绝对引用：无 scheme 时去前导 "/"（Java:159-161）
        assert_eq!(
            f.to_root_based_name("sub/context.ftl", "/t.ftl").unwrap(),
            "t.ftl"
        );
        // 目标含 "://"（位置 > 0）→ scheme 名原样（Java:153-154）
        assert_eq!(
            f.to_root_based_name("sub/context.ftl", "http://x/y.ftl")
                .unwrap(),
            "http://x/y.ftl"
        );
        // base 带 "://" scheme 时绝对引用保留 scheme 前缀（Java:155-159：
        // substring(0, schemeSepIdx+2) 含 ":/" 两个字符，再拼绝对名）
        assert_eq!(
            f.to_root_based_name("classpath://dir/t.ftl", "/x").unwrap(),
            "classpath://x"
        );
    }
}
