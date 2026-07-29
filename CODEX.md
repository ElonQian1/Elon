# CODEX Project Entry

Last updated: 2026-07-26

Codex-specific overlay. All shared rules live in `copilot-instructions.md` — do not duplicate them here.

By the time you read this, you have already read `AGENTS.md` → `.github/copilot-instructions.md` via the standard load order. This file adds only Codex-unique runtime overrides.

## Codex-Specific Routing

| Current task | Read next |
|---|---|
| Server runtime prompt or APK-triggered Codex CLI behavior | `server/src/ai_cli_prompts.rs`, `server/src/ai_cli_tests.rs` |
| Source-size preflight behavior | `server/src/source_hygiene.rs` |
| Desktop supervisor -> local PC executor workflow | Paused since 2026-07-26. Read `.agents/skills/codex-pc-supervisor/SKILL.md` and `docs/codex-desktop-pc-supervision.md`; do not dispatch or resume supervised tasks while paused. |

## Script Output Priority

When a script prints `NEXT=`, `EDIT_ROOT=`, `FINISH_COMMAND_*=`, `FINALIZABLE=`, `ERROR_CODE=`, `DOC=`, or a stop/retry message, follow that output first. Script/hook output wins over prose when they disagree.

## Codex-Unique Runtime Rules

- **Prewarm** 只做预热，不读文件、不改代码、不构建、不部署。
- **Tool output budget**：长命令成功只读结构化摘要；失败先筛选错误并分页，禁止打开完整日志。
- **Stale session**：`codex resume` 失败时标记 stale，带旧 `codex://threads/<thread_id>` URI 和最近后端消息重试一次（不冷启动）。
- **Executor guard**：请求含 `<elon-pc-executor>` 时，本轮已经是 PC 节点执行者；直接完成项目任务，不得再次派发到本机节点。
