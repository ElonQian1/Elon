---
applyTo: "scripts/**,.github/**,AGENTS.md,CODEX.md,AI_TASK_TEMPLATE.md"
---

# Git、验证与发布按需手册

共享硬规则见 `.github/copilot-instructions.md` 的 `WF-*`；本文件只补实现细节。

## 标准路径

1. 执行 `WF-START`，切到 `EDIT_ROOT`。
2. 修改和验证后检查 `git status --short --branch`。
3. 按 `WF-FILES` 处理每个新增文件，只 stage 本任务交付物。
4. commit 后执行 `git push origin HEAD:main`。
5. 需要发布时运行对应 `publish-*` 脚本。
6. 最后执行预检输出的 `FINISH_COMMAND_*`，把 `<Kind>` 换成任务完成类型。

不要手拼远端检查、main 同步或 worktree 清理；统一用 `finish-ai-task.*` 收尾。

## 本地 main 与未跟踪文件

- `main` checkout 只存基线，不做业务编辑。
- 已跟踪修改阻止 `main` 快进；`ai/session/*` 只报 `blocked_tracked_changes`，不触碰 `main`。普通未跟踪文件不阻止快进。
- 远端新增路径与本地未跟踪文件同名时 Git 会拒绝快进；脚本输出 `FINALIZABLE=false`。
- 主工作区来源不明的未跟踪文件只告警，不自动提交或删除。
- 任务 worktree 必须干净；未跟踪源码/测试显示 `candidate_track`，生成物显示 `candidate_temporary_or_precise_ignore`。
- 自动删除仅限 `.ai/workspace-policy.txt` 声明的临时根，目前是 `.ai-tmp/`。
- 保护 60 分钟内及 locked worktree；预检锁定 `codex/*`，收尾解锁。平台会话/`ai/session/*` 仅在 clean、已合入且超龄时回收。

## push 冲突

只有 `git push origin HEAD:main` 返回 non-fast-forward 时执行：

```powershell
git fetch origin
git rebase origin/main
# 解决冲突，git add 对应文件，然后 git rebase --continue
git push origin HEAD:main
```

兼容时保留双方逻辑；不追逐远端。rebase 后复用原验证，仅为冲突或受影响处补最小验证。

## push 输出管理

Rust 收据门禁默认关闭：两个 push 入口输出 `RUST_PUSH_RECEIPT_GATE=disabled`，不运行 `prepare-push`/`cargo check`；仅 `ELON_ENABLE_RUST_PUSH_RECEIPT=1` 启用。日志写入 `.ai-tmp/push.log`：

```powershell
git push origin HEAD:main *> .ai-tmp/push.log
if ($LASTEXITCODE -ne 0) { Get-Content .ai-tmp/push.log -Tail 40 }
```

Bash: `git push origin HEAD:main > .ai-tmp/push.log 2>&1 || tail -n 40 .ai-tmp/push.log`

## 验证入口

### Rust/Cargo

Windows：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\cargo-dev.ps1 -- check --manifest-path server\Cargo.toml --locked
```

Linux/macOS：

```bash
bash scripts/cargo-dev.sh check --manifest-path server/Cargo.toml --locked
```

验证走 `scripts/validate-rust.ps1`；细节见 `docs/rust-cache-platform.md`。

### Rust 格式化

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\format-rust.ps1 -Apply -Files <本次改动.rs...>
```

Linux/macOS 使用 `bash scripts/format-rust.sh --apply --files ...`。脚本按 `.rustfmt-version` 拒绝版本漂移；无参数只做全仓检查。全量写入只能在干净隔离任务中显式使用 `-Apply -All` / `--apply --all`，确认全部为纯 rustfmt 后先提交并推送独立 `style(rust)` commit，再提交业务改动。误触后若仍是纯 rustfmt，不为缩小 diff 反复撤销；工具链错误、混入语义变化或来源不明现场则停止并报告。

## 发布入口

| 类型 | 发布 | 收尾 |
|---|---|---|
| 只同步代码 | 无 | `finish-ai-task.*` + `CodePushed` |
| 后端 | `scripts/publish-server.ps1` / `.sh` | `Server` |
| PC 前端 | 先构建，再运行 `publish-server.*` | `PcFrontend` |
| Win 节点 | `scripts/publish-node-agent.ps1` | `NodeAgent` |
| Android APK | `publish-apk.*`，提供 changelog | `AndroidFeature` |

发布脚本负责版本 claim/finish、构建、上传、并发保护和线上验证。不要直接改版本字段、手搓上传或部署未提交内容。

## PowerShell 与网络

- bootstrap 和未声明 PS7 的兼容脚本可用 `powershell.exe`。
- 有 `#requires -Version 7.0` 时必须用 `pwsh`；策略详见 `docs/powershell-version-policy.md`。
- 访问项目服务器时遵守仓库 `direct-network` 脚本输出，不把 token 打进日志。

## 工作流变更门禁

修改预检、收尾、worktree 清理、共享契约或本文件后，必须运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\test-ai-task-preflight-workflow.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\audit-ai-prompt-assets.ps1
```

## 最终汇报

报告提交 SHA、push/发布验证结果，并原样概括统一收尾的：

- `BUSINESS_STATUS`
- `LOCAL_MAIN_STATUS`
- `MAIN_UNTRACKED_STATUS`
- `TASK_WORKTREE_STATUS`
- `FINALIZABLE`

只有 `FINALIZABLE=true` 才能正常宣告任务完整结束；否则要区分“业务已完成”和“本机收尾待处理”。完整产品执行流程仅在卡住或处理复杂发布时读取 `docs/ai-agent-workflow.md`。
