# CODEX Project Entry

Last updated: 2026-05-28

Codex-specific overlay. All shared rules live in `copilot-instructions.md` — do not duplicate them here.

## Load Order

1. Read `.github/copilot-instructions.md` — authoritative rules source (rules, task-start, deploy cheatsheet, APK build decision table).
2. Read `AGENTS.md` — routing table to task-specific docs.
3. Read this file — Codex runtime overrides only.
4. Run preflight (`bash scripts/ai-task-preflight.sh --create-worktree`), inspect `git status --short --branch`.
5. Read only the task-specific doc routed by `AGENTS.md`.

## Codex-Specific Routing

| Current task | Read next |
|---|---|
| Server runtime prompt or APK-triggered Codex CLI behavior | `server/src/ai_cli_prompts.rs`, `server/src/ai_cli_tests.rs` |
| Source-size preflight behavior | `server/src/source_hygiene.rs` |

## Script Output Priority

When a script prints `NEXT=`, `ERROR_CODE=`, `DOC=`, or a stop/retry message, follow that output first. Script/hook output wins over prose when they disagree.

## Codex Runtime Rules

- **APK 并发**：不同 `project_id` 或 `conversation_id` 可并行编码，各用独立 worktree；merge/claim/deploy 仍串行。同一 `project_id + conversation_id` 同一时刻只跑一个任务。
- **Prewarm** 只做预热，不读文件、不改代码、不构建、不部署。
- **Stale session**：`codex resume` 失败时标记 stale，带旧 `codex://threads/<thread_id>` URI 和最近后端消息重试一次（不冷启动）。
- **APK 发布用脚本**（`publish-apk.sh`），脚本已内置并发保护和 claim/finish，不要手搓发布流程。
- **后端发布必须用脚本**（`bash scripts/publish-server.sh`）：脚本负责 `git pull --rebase` → `POST /api/release/claim`（分配版本号）→ `cargo zigbuild`（注入 `ELON_BUILD_VERSION`）→ 上传 → `POST /api/release/finish`。**绝对禁止直接调用 `cargo build` / `cargo zigbuild` 替代脚本**，否则 binary 版本号不递增（卡在 Cargo.toml 兜底值），且服务器 release 槽位泄漏。
