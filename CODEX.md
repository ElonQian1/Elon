# CODEX Project Entry

Last updated: 2026-05-27

This is the Codex-specific overlay for the Elon repository. `AGENTS.md` is the shared source of always-on rules; do not duplicate those rules here.

## Load Order

1. Read `.github/copilot-instructions.md` — this is the authoritative rules source for all AI agents (rules, task-start commands, deploy cheatsheet).
2. Read `AGENTS.md` — routing table to task-specific docs.
3. Read this file (`CODEX.md`) — Codex-specific overrides only.
4. Run and obey repository preflight before code changes.
5. Inspect `git status --short --branch`.
6. Read only the task-specific document routed by `AGENTS.md`.
7. Prefer scripts, hooks, tests, and release tools over remembered process text.

## Codex-Specific Routing

| Current task | Read next |
|---|---|
| Server runtime prompt or APK-triggered Codex CLI behavior | `server/src/ai_cli_prompts.rs`, `server/src/ai_cli_tests.rs` |
| Source-size preflight behavior | `server/src/source_hygiene.rs` |
| Copilot/VS Code instructions | `.github/copilot-instructions.md`, `.github/prompts/`, `.github/agents/`, `.github/skills/` |

## Script Output

When a script prints `NEXT=`, `ERROR_CODE=`, `DOC=`, or a clear stop/retry message, follow that script output first. Read broader workflow docs only if the script output is insufficient.

Hook/test/script output wins over prose when they disagree.

## Persistence Boundary

Codex does not have reliable cross-task memory by itself. Durable workflow changes must be committed to repository entrypoints, task-specific docs, scripts/hooks, or the backend runtime prompt that launches remote Codex CLI.
