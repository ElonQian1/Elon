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

---

## APK Task Concurrency (Conversation Worktrees)

APK-triggered project tasks use per-conversation Git worktrees plus shared-resource gates on the server:

- Different `project_id` values can run coding work at the same time.
- Different `conversation_id` values inside the same project can also run coding work at the same time — each gets its own Git worktree and `ai/session/...` branch.
- Codex CLI must work only inside the assigned conversation worktree and push only that branch.
- The same `project_id + conversation_id` still runs one task at a time (no self-race).
- Merge to `main`, `/api/release/claim`, APK publishing, and server deployment remain serialized shared-resource steps.
- Backend Git preflight failures are not final user failures. Pass them to Codex CLI as context; let CLI inspect `git status/diff` and try safe recovery first.

---

## Backend and Codex CLI Cooperation

The backend is the workflow orchestrator; Codex CLI is the code executor.

- **Prewarm** (`/api/.../prewarm`) is best-effort native session warmup only. It must NOT read files, run Git, edit code, build, deploy, or inject the full project workflow.
- The backend sends a structured task brief on every real turn: user request, project path, mandatory doc-read order, Git rules, validation expectations, and the rule that release/deploy steps are serialized.
- If `codex resume` reports a stale/expired session, mark it stale and retry once with a fresh session. The retry must include a continuity handoff: the old `codex://threads/<thread_id>` URI and recent backend conversation messages so the new session can reconnect instead of starting cold.
- Non-Codex models may only act as sidecar helpers (classification, summarization, image analysis). Their output must be injected back into the bound Codex CLI session; they must not become the primary conversation owner.
- After Codex CLI finishes, the backend is responsible for observable status, task records, download links, release metadata, and shared locking around merge/version/release/deploy.
- Keep the user/client `trace_id` attached through routing, intent confirmation, prewarm, and the final Codex CLI call. `GET /api/debug/traces/:trace_id` should show `codex_cli_start/done/error/retry` events, prompt size, session hit/miss, elapsed time, and `codex_cli_elapsed_ms`.

---

## APK Release Decision Flow (Remote Codex CLI)

When running an APK release from a remote Codex CLI session:

1. Sync to latest `main` before building.
2. Build only from a known pushed SHA — no `release(android)` commit is created or allowed.
3. If another machine publishes a newer APK that already contains this build's base SHA → stop local release, call `finish(success=false)`, verify that live APK instead.
4. If `origin/main` advances but the live APK does not prove it contains this build's base SHA → abort upload, restart release from latest `main`.
5. Never reuse a `versionCode` from a `/api/release/claim` that has already been finished — restart the claim.
