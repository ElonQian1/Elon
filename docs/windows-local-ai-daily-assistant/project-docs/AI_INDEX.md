# AI Index

最后更新：2026-07-09

本文件是给 AI 的入口索引。当前项目还未创建真实代码目录，下面先定义未来推荐结构。

## 根目录文档

| 文件 | 用途 |
|---|---|
| `AGENTS.md` | 所有 AI 代理入口 |
| `AI_PROJECT.md` | 项目目标、用户、MVP 范围 |
| `AI_ARCHITECTURE.md` | 架构分层、模块边界、数据流 |
| `AI_RULES.md` | 硬规则和禁止事项 |
| `AI_TASK_TEMPLATE.md` | 给 AI 派任务的模板 |
| `README.md` | 面向人类用户和开发者的项目说明 |

## 专项约束文档

| 文件 | 任务 |
|---|---|
| `docs/product-scope.md` | 产品范围、MVP/非 MVP |
| `docs/privacy-and-data-policy.md` | 隐私、授权、数据保留、删除 |
| `docs/ai-analysis-policy.md` | AI Vision、prompt、JSON、成本、失败处理 |
| `docs/windows-capture-policy.md` | Windows 截图、窗口标题、排除应用 |
| `docs/local-storage-policy.md` | SQLite、迁移、数据清理 |
| `docs/ui-product-rules.md` | UI、设置、时间线、日报预览 |
| `docs/build-and-release-policy.md` | 构建、测试、打包、发布 |

## 未来源码入口建议

| 领域 | 推荐路径 |
|---|---|
| Tauri Rust 后端 | `src-tauri/src/` |
| 截图采集 | `src-tauri/src/recorder/` |
| 隐私保护 | `src-tauri/src/privacy/` |
| AI Provider | `src-tauri/src/ai/` |
| SQLite 存储 | `src-tauri/src/storage/` |
| 时间线合并 | `src-tauri/src/timeline/` |
| 日报生成 | `src-tauri/src/report/` |
| React UI | `src/` |
| 设置页 | `src/features/settings/` |
| 时间线页 | `src/features/timeline/` |
| 日报页 | `src/features/report/` |
| 共享类型 | `src/shared/` |

## 未来验证命令建议

真实项目创建后再确认，以项目实际脚本为准。

建议预留：

```powershell
npm run typecheck
npm run lint
npm run test
npm run tauri build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

## 搜索建议

- 找隐私规则：搜索 `Privacy Guard`、`screenshot_policy`、`excluded_app`。
- 找 AI 分析：搜索 `AI Analyzer`、`VisionProvider`、`analysis_records`。
- 找日报生成：搜索 `Report Generator`、`reports`、`Markdown`。
- 找 Windows 采集：搜索 `Recorder`、`foreground window`、`window_title`。
