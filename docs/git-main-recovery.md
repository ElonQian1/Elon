---
version_status: current
reviewed_at: 2026-09-17
---

# main 同步失败的取证与恢复

仅在 `LOCAL_MAIN_RECOVERY_REQUIRED=true`、Git 快进失败或主工作区存在疑似同步残留时读取。普通任务遵守 `WF-*` 和 Git 工作流手册。

## 为什么会出现“已提交文件变成未提交”

Git checkout/快进遇到路径过长、磁盘或权限错误时，可能已经写入部分文件而未推进 HEAD。此时 `M`、`??` 只是相对旧 HEAD 的状态，不能单凭它认定某个会话漏提交。

预检和收尾的 Windows Git 写入、worktree 创建使用命令级 `-c core.longpaths=true`，不改变用户全局或仓库配置。该选项不能解决磁盘、权限、单路径分量过长或其他工具的路径限制。

## 失败信号

- 本轮同步失败：预检非零退出，输出 `ERROR_CODE=MAIN_BASELINE_SYNC_FAILED`、`EDIT_ROOT=BLOCKED_MAIN_BASELINE_SYNC_FAILED`、`MAIN_BASELINE_SYNC_FROM` 和 `MAIN_BASELINE_SYNC_TARGET`，不继续创建任务 worktree。
- 先前已存在的主目录改动：保留并输出 `LOCAL_MAIN_RECOVERY_REQUIRED=true`。其他独立任务可以完成已推送的业务，但报告必须保留本机异常和恢复入口；不能把 `FINALIZABLE=true` 解释为本地 main 干净或最新。
- 失败后立即保存原始日志和文件证据，避免把一次部分 checkout 误报为业务代码漏提交。

## 恢复步骤

1. 记录 `git status --porcelain=v1 -uall`、HEAD、失败目标和文件哈希；确认没有其他代理正在写这些文件。
2. 将每个文件与失败目标、当前 `origin/main` 及其历史中的同路径 Git blob 比较。只有完全匹配远端可达提交的内容，才能认定为已提交内容的残留。时间戳和文件名只用于辅助定位。
3. 把所有将处理的文件连同清单备份到仓库外，逐个校验 SHA-256；再次检查现场和 HEAD 没有变化。若没有清理授权，先交付取证结果。
4. 已授权且证据完整时，只按核验清单恢复已跟踪文件、转存已确认残留的未跟踪文件。独立改动、删除、重命名、符号链接或未知文件保留并单独处理。
5. 通过预检/统一收尾重新同步 main，验证工作区干净且 `HEAD=origin/main`，并报告备份位置。恢复不产生新的业务提交。

禁止用全仓 `reset --hard`、`clean -fd`、批量 add/commit 或新增忽略规则隐藏现场。失败后按实际错误修复，不无限重试，也不自动恢复来源未知的文件。

## 验证入口

`scripts/test-ai-main-baseline-sync.ps1` 使用真实临时 Git 仓库验证同步失败停止、未知文件保留、普通快进和长路径写入，由 `scripts/test-ai-task-preflight-workflow.ps1` 统一调用。恢复代码或工作流变更同时执行提示词资产审计。
