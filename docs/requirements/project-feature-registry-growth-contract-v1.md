---
title: 项目功能注册表增长门禁修正
status: current
reviewed_at: 2026-08-14
owners: ai-platform, developer-experience
implementation_status: implementation_locally_verified
---

# 项目功能注册表增长门禁修正

## 问题

`.elon/project-features.json` 已从初始种子演进为 Git 共享功能注册表。服务端允许最多 512 项功能，MCP 与 PC 通过有界分页读取；旧 adoption 测试仍要求整个文件不超过 64 KiB，并只允许 1 至 12 项功能。主线注册表在本次修改前已经超过 64 KiB，因此该门禁不再验证当前合同，只会阻断正常生命周期记录。

## 合同

- 注册表继续是有界 Git 文件，CI 文件预算固定为 8 MiB，不允许无上限增长；
- 功能数量与服务端 `MAX_FEATURES=512` 保持一致；
- adoption 状态检查与当前状态机保持一致：`draft`、`proposed`、`accepted`、`ready`、`claimed`、`in_progress`、`blocked`、`implemented`、`verified`、`released`、`retired`；
- MCP/PC 的 list、plan、history 仍须分页或限制返回数量，本修正不允许一次响应返回完整注册表或源码正文；
- 历史审计仍由服务端限制为最近 200 条，不通过删除真实生命周期记录压缩本批提交。

## 验收

1. 当前超过 64 KiB 的合法注册表可以通过 adoption test；超过 8 MiB 仍失败关闭。
2. adoption test 接受 1 至 512 项功能，并拒绝重复 ID。
3. adoption feature 的需求哈希、任务路径、验收标准和当前状态仍被核对。
4. `test-ai-task-preflight-workflow.ps1` 与 `audit-ai-prompt-assets.ps1` 继续通过。
