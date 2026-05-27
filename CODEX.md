# CODEX Project Entry

Last updated: 2026-05-27

This is the Codex-specific lightweight entry for the Elon repository. Keep it short: load the always-on rules here, then read task-specific documents only when the request needs them.

## Load Order

1. Read `AGENTS.md` and this file.
2. Inspect `git status --short --branch` and run repository preflight before code changes.
3. Read only the task-relevant detailed document listed below.
4. Prefer scripts, hooks, tests, and release tools over remembering long process text.

Do not automatically read `.github/copilot-instructions.md` for normal Codex work. It is a Copilot/VS Code customization surface; read it only when the task is about Copilot, VS Code customizations, or comparing agent instruction behavior.

## Required Start

```powershell
powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree
```

If it returns `WORKTREE_CREATED=true`, continue only in `WORKTREE_PATH`. On a new clone, install hooks once with `pwsh scripts/install-hooks.ps1`.

Linux/macOS/server Codex CLI:

```bash
bash scripts/ai-task-preflight.sh --create-worktree
```

On a new Linux/macOS clone, run `bash scripts/install-hooks.sh` once.

## Always-On Rules

- Protect existing work. Never discard unrelated user or AI changes.
- Before editing code, output a 5-15 line file plan with target files and estimated line counts.
- Keep source files modular: new source files target <=500 lines; 501-800 lines are tolerated only for one focused responsibility; >800 lines must be split.
- Do not add feature logic to existing >1500-line files except tiny fixes; extract the touched responsibility first.
- Fetch `origin/main` at task start and again before commit/rebase/push.
- Stage only this task's files, including new files. Check for untracked files before committing.
- Do not force push. If push is rejected, rebase onto the latest remote state and resolve conflicts.
- Backend/APK releases must build from committed and pushed code.
- Release versions are server-allocated; do not manually bump or commit release-only version fields.
- Do not commit secrets, `.env`, APK signing material, or machine-specific absolute paths.

## Read On Demand

| Current task | Read next |
|---|---|
| Git workflow, worktree isolation, push rejection, deploy/release | `.github/instructions/git-deploy-workflow.instructions.md` |
| Modularization, giant-file cleanup, file-size policy | `.github/instructions/modular-architecture.instructions.md` |
| High-level implementation workflow | `docs/ai-agent-workflow.md` |
| Architecture, module boundaries, data flow | `docs/system-architecture.md` |
| Server runtime prompt, APK-triggered Codex CLI behavior | `server/src/ai_cli_prompts.rs`, `server/src/ai_cli_tests.rs` |
| Source-size preflight behavior | `server/src/source_hygiene.rs` |
| Android Gradle download/cache/proxy issue | `docs/android-setup.md` |
| Copilot/VS Code instructions | `.github/copilot-instructions.md`, `.github/prompts/`, `.github/agents/`, `.github/skills/` |

## Script-First Rules

- Use `scripts\ai-task-preflight.ps1` / `scripts/ai-task-preflight.sh` for workspace safety instead of hand-writing sync logic.
- Use `scripts\publish-server.ps1` / `scripts/publish-server.sh` for backend deployment after commit + push.
- Use `scripts\publish-apk.ps1` plus `scripts\check-task-complete.ps1 -Kind AndroidFeature` for installable APK changes after commit + push.
- Use hook/test output as the source of truth when it conflicts with prose.
- If a script fails, read the matching detailed document or script section before retrying.

## Delivery Checklist

- `git status --short --branch` is understood.
- New files are explicitly staged when committing.
- The smallest meaningful tests/builds were run, or skipped with a concrete reason.
- Server or APK tasks report the published SHA and live verification result.
- Final response states what changed, what was verified, and what remains.

## Persistence Boundary

Codex does not have reliable cross-task memory by itself. Durable workflow changes must be committed to repository entrypoints, task-specific docs, scripts/hooks, or the backend runtime prompt that launches remote Codex CLI.
