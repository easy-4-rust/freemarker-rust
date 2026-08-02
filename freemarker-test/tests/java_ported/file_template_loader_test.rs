//! Java `freemarker.cache.FileTemplateLoaderTest` 的 Rust 1:1 实现
//! （FileTemplateLoaderTest.java：临时目录 + FileTemplateLoader 的加载/未找到/
//!   大小写敏感文件系统模拟测试）
//!
//! 引擎映射：`freemarker::cache::FileLoader` 对应 FileTemplateLoader；
//! `setDirectoryForTemplateLoading` → `c.template_loader = Arc::new(FileLoader::new(dir)?)`。
//! 引擎差异：`setEmulateCaseSensitiveFileSystem` 未实现（v1 FileLoader 恒为
//! OS 文件系统语义——macOS 上大小写不敏感，与 Java 模拟关闭时一致）。

#[allow(unused_imports)] // 任务约定：每个测试文件以 use crate::util::* 开头
use crate::util::*;
use freemarker::cache::FileLoader;
use freemarker::template::Configuration;
use std::path::PathBuf;
use std::sync::Arc;

/// 唯一临时目录（std::env::temp_dir() 下；对应 Java Files.createTempDir）
fn unique_temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "freemarker-rs-fileloader-{tag}-{}-{}",
        std::process::id(),
        nanos
    ))
}

/// 配置 + FileLoader（对应 Java @Before setup：建 sub1/sub2/t.ftl("foo") +
/// setDirectoryForTemplateLoading）
fn file_cfg(tag: &str) -> (Configuration, PathBuf) {
    let dir = unique_temp_dir(tag);
    std::fs::create_dir_all(dir.join("sub1/sub2")).unwrap();
    std::fs::write(dir.join("sub1/sub2/t.ftl"), "foo").unwrap();
    let mut c = Configuration::new();
    c.settings.locale = "en_US".to_string();
    let loader = Arc::new(FileLoader::new(&dir).unwrap());
    c.template_loader = loader;
    (c, dir)
}

/// Java testSuccessful：两次 getTemplate("sub1/sub2/t.ftl") 均输出 "foo"
/// （第二次走模板缓存）
#[test]
fn test_successful() {
    let (c, _dir) = file_cfg("successful");
    for _ in 0..2 {
        let t = c.get_template("sub1/sub2/t.ftl").expect("加载成功");
        let mut out = Vec::new();
        t.process(
            freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out), "foo");
    }
}

/// Java testSuccessful2：`setEmulateCaseSensitiveFileSystem(true)` + 清缓存后两次加载。
/// 引擎差异：v1 FileLoader 无大小写模拟开关（恒为 OS 语义，此处等价模拟关闭），
/// 断言保留 Java 的输出结果 "foo"。
#[test]
fn test_successful2() {
    let (c, _dir) = file_cfg("successful2");
    // 引擎差异：FileTemplateLoader.setEmulateCaseSensitiveFileSystem(true) 未实现
    for _ in 0..2 {
        c.cache.lock().unwrap().clear(); // 对应 cfg.clearTemplateCache()
        let t = c.get_template("sub1/sub2/t.ftl").expect("加载成功");
        let mut out = Vec::new();
        t.process(
            freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
            &mut out,
        )
        .unwrap();
        assert_eq!(String::from_utf8_lossy(&out), "foo");
    }
}

/// 取回 get_template 的错误（Template 无 Debug，不能 expect_err）
fn get_err(c: &Configuration, name: &str) -> freemarker::error::TemplateError {
    match c.get_template(name) {
        Err(e) => e,
        Ok(_) => panic!("get_template({name}) 应报错"),
    }
}

/// Java testNotFound：不存在的文件 → TemplateNotFoundException，
/// 消息含 "sub1X"（Java 另断言 getCause()==null，Rust 无 cause 概念）
#[test]
fn test_not_found() {
    let (c, _dir) = file_cfg("notfound");
    for _ in 0..2 {
        let e = get_err(&c, "sub1X/sub2/t.ftl");
        assert!(
            e.to_user_message().contains("sub1X"),
            "{}",
            e.to_user_message()
        );
    }
}

/// Java testCaseSensitivity：模拟大小写敏感与否对大小写错误的名称的影响。
/// macOS/Windows 文件系统大小写不敏感且模拟关闭 → 坏大小写也能命中。
/// 引擎差异：setEmulateCaseSensitiveFileSystem 未实现——恒为 OS 语义
/// （Java 的 emuCaseSensFS=true 分支无法复现，注释保留其断言）。
#[test]
fn test_case_sensitivity() {
    let (c, _dir) = file_cfg("casesens");
    let case_insensitive_fs = cfg!(target_os = "macos") || cfg!(target_os = "windows");
    for name_with_bad_case in ["SUB1/sub2/t.ftl", "sub1/SUB2/t.ftl", "sub1/sub2/T.FTL"] {
        c.cache.lock().unwrap().clear();
        if case_insensitive_fs {
            // Java（macOS/Windows && !emuCaseSensFS）分支：坏大小写直接命中
            let t = c
                .get_template(name_with_bad_case)
                .expect("大小写不敏感 FS 应命中");
            let mut out = Vec::new();
            t.process(
                freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
                &mut out,
            )
            .unwrap();
            assert_eq!(String::from_utf8_lossy(&out), "foo");
            // 引擎差异：emuCaseSensFS=true 分支（Java 期望 TemplateNotFoundException）无法复现
        } else {
            // 大小写敏感 FS（Linux）：小写名命中，原大小写未找到
            let lower = name_with_bad_case.to_lowercase();
            let t = c.get_template(&lower).expect("小写名应命中");
            let mut out = Vec::new();
            t.process(
                freemarker::template::TModel::from_hash(indexmap::IndexMap::new()),
                &mut out,
            )
            .unwrap();
            assert_eq!(String::from_utf8_lossy(&out), "foo");
            let e = get_err(&c, name_with_bad_case);
            assert!(
                e.to_user_message().contains(name_with_bad_case),
                "{}",
                e.to_user_message()
            );
        }
    }
}

/// Java testDefault：默认 emulation 关闭。
/// 引擎差异：v1 无 getEmulateCaseSensitiveFileSystem——直接验证构造成功
/// （对应 Java `new FileTemplateLoader(templateRootDir)` 不抛异常）。
#[test]
fn test_default() {
    let dir = unique_temp_dir("default");
    std::fs::create_dir_all(&dir).unwrap();
    let loader = FileLoader::new(&dir).expect("目录存在应构造成功");
    // 引擎差异：Java 断言 getEmulateCaseSensitiveFileSystem()==false；v1 无此开关
    // （macOS 的 temp_dir 经 /var→/private/var 符号链接，基目录为 canonical 形式）
    assert_eq!(
        loader.base_directory().to_string_lossy(),
        std::fs::canonicalize(&dir).unwrap().to_string_lossy()
    );
    let _ = dir;
}
