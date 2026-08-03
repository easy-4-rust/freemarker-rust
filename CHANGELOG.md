# Changelog

本项目遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/) 与
[语义化版本](https://semver.org/lang/zh-CN/)（晋级规则见
[docs/release/versioning.md](docs/release/versioning.md)）。

## [Unreleased]

### Added

- **依赖与许可治理门禁**：`deny.toml`（licenses allow 列表 + sources/advisories 策略） +
  `.cargo/audit.toml`（RUSTSEC 豁免登记） + CI 接入 `EmbarkStudios/cargo-deny-action@v2`
  与 `cargo audit` 步骤。
- **公共 API 基线**：`docs/release/api-baseline.txt`（freemarker 核心 3,705 项公开 API 快照）+
  CI `cargo public-api` diff 门禁。
- **workspace 元数据补全**：`[workspace.package]` 增补 description/authors/repository/
  homepage/categories/keywords；各成员 crate 补 categories/keywords/repository
  （freemarker-pyo3 publish=true 但本轮不实际发布；freemarker-test publish=false）。
- **版本治理文档**：`docs/release/versioning.md`（alpha→1.0 晋级规则与可执行门禁）。

### Fixed

- freemarker-pyo3 `Cargo.toml` 元数据字段位置修正（categories/keywords 从 `[lib]` 段后移至 `[package]` 段内）。

### Governance

- `cargo deny check` 全绿（licenses/bans/sources/advisories）。
- `cargo audit` 0 error。
- `cargo public-api -p freemarker` diff 基线 0 diff。