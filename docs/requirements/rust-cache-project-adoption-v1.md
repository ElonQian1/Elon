---
title: "Rust 缓存子项目一键接入 V1"
owner: platform
version_status: current
reviewed_at: 2026-08-16
---

# Rust 缓存子项目一键接入 V1

## 目标

为不包含缓存平台源码的 Rust 子项目提供统一接入命令。命令默认只预演，显式应用后生成可提交的项目清单和薄启动器，使不同 Windows PC、Codex、Copilot 及其他 AI 代理都能使用同一项目入口调用已安装缓存平台。

## 非目标

- 不复制缓存平台模块到子项目。
- 不提交缓存盘符、用户目录、节点数据根或安装路径。
- 不自动启用未经审查的命名共享分区。
- 不覆盖子项目已有的非平台脚本。
- 不在接入过程中运行构建、删除缓存或修改 Cargo 锁文件。

## 验收标准

1. `adopt-project` 默认返回预演，且不创建任何文件。
2. `adopt-project -Apply` 创建 `rust-cache.project.json` 和 `scripts/rust-cache.ps1` 两个可提交文件。
3. 薄启动器从当前 PowerShell 会话调用 `%LOCALAPPDATA%\Elon\bin\rust-cache.ps1`，不启动可见子 PowerShell，不包含生成电脑的绝对路径。
4. 重复使用相同参数保持幂等；现有清单或脚本内容不一致时失败关闭且不覆盖。
5. 生成的薄启动器拒绝调用方另传 `-ProjectRoot`，始终绑定自身仓库根。
6. 便携性回归覆盖预演、应用、幂等、冲突保护、无机器路径和实际参数转发。
7. `manage-shared-build-cache` Skill 使用该命令作为子项目首选接入流程，并保留 `init-project` 作为只生成清单的低层入口。

## 实现范围

- `scripts/rust-cache/RustCache.ProjectAdoption.psm1`
- `scripts/rust-cache.ps1`
- `scripts/rust-cache/RustCache.Help.psm1`
- `scripts/test-rust-cache-portability.ps1`
- `.agents/skills/manage-shared-build-cache/SKILL.md`
- `docs/rust-cache-on-demand-adoption.md`
