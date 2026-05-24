---
name: cloud-apk-dev
description: >
  一龙云端 APK 开发平台的代码修改、验证、Git、部署和 APK 发布工作流。
  当任务涉及 Android、Rust 服务端、Web、部署、发布新 APK、版本号或服务器验证时使用。
---

# Cloud APK Development Skill

Use this skill for one-stop project work in the Elon repository:

- Understand a user request.
- Locate the affected Android, Rust server, Web, docs, or script files.
- Make the smallest safe change.
- Run the matching verification command.
- Commit only task-related files.
- Push to `origin/main`.
- Deploy server or publish APK only from a clean pushed SHA.

## Required Context

Read these files before acting:

1. `AGENTS.md`
2. `.github/copilot-instructions.md`
3. `.github/instructions/git-deploy-workflow.instructions.md`
4. `docs/ai-agent-workflow.md`
5. `docs/system-architecture.md`

## Commands And Entry Points

- Common task prompt: `/elon-dev-task`
- APK release prompt: `/elon-apk-release`
- Planning agent: `elon-planner`
- Implementation agent: `elon-implementer`
- Review agent: `elon-reviewer`

## Non-Negotiables

- Start and end with `git status --short --branch`.
- If the main workspace has unrelated uncommitted changes, use a temporary worktree.
- Never deploy uncommitted code.
- Never stage unrelated files.
- Never commit secrets, `.env`, APK signing keys, or generated private credentials.
- If push is rejected, fetch/rebase or merge, resolve conflicts while preserving both sides when compatible, then push again.
- If uncommitted changes are unrelated or unclear, create a new worktree from `origin/main` instead of pulling in the dirty workspace.
- For backend runtime changes, increment `server/Cargo.toml` `package.version`, deploy with `scripts/publish-server.*`, and verify `/api/server/version`.
- For Android installable features, PR/debug build is not complete. Run `scripts\publish-apk.ps1`, then `scripts\check-task-complete.ps1 -Kind AndroidFeature`, unless the user explicitly says not to publish the APK.
