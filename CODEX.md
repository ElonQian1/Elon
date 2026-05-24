# CODEX Project Memory

## Project Identity

This project is a cloud APK development platform. Users describe app changes in natural language from the Android APK. The backend AI agent plans code changes, updates the Rust server / Android app / future frontend, builds and deploys, then returns a new APK download link to the user.

Use the repository directory that contains `server/`, `android/`, `docs/`, `scripts/`, `.github/`, and `.copilot/` as the project root.

## Server Codex CLI Memory

Codex CLI on the server does not have reliable cross-task memory by itself. Treat repository files as the durable memory layer. At the start of a development task, read this file, `AGENTS.md`, `.github/copilot-instructions.md`, `.github/instructions/*.md`, and task-relevant files in `docs/`.

If a workflow rule changes, update the relevant documentation in the same commit as the code change. Future APK-triggered tasks depend on these files being accurate.

The built-in "一龙项目" is not a special execution path. It is a normal `local_path` project record that points at a real Git workspace. Any other local project can be configured the same way with `ELON_PROJECT_<PROJECT_ID>_PATH`, where the project id is uppercased and non-alphanumeric characters become `_`.

For `local_path` and GitHub projects, the workspace must already be a real Git repository with `.git` and a usable remote. Do not silently initialize a new Git repository for these projects; failing clearly is safer than creating an unpushable local history.

## Source Documents To Respect

- `AGENTS.md`: shared entry point for VS Code Copilot, Codex, Claude Code, and other AI agents.
- `.github/copilot-instructions.md`: global project positioning and agent principles.
- `.github/instructions/git-deploy-workflow.instructions.md`: mandatory Git, push, worktree, deploy, and report workflow.
- `.github/prompts/*.prompt.md`: VS Code slash-command prompts for recurring project workflows.
- `.github/agents/*.agent.md`: VS Code custom agents for planning, implementation, and review roles.
- `.github/skills/cloud-apk-dev/SKILL.md`: VS Code official Agent Skills entry for cloud APK development and deployment.
- `.copilot/skills/cloud-apk-dev/SKILL.md`: cloud APK development and deployment workflow.
- `docs/ai-agent-workflow.md`: full AI agent workflow from request analysis through code change, verification, commit, build, deploy, and feedback.
- `docs/system-architecture.md`: architecture, data flow, module responsibilities, and security constraints.

## Local Skills Absorbed

The local share `\\127.0.0.1\skills` has been reviewed for this repository. Keep these distilled lessons in project workflow memory:

- `ai-git-deploy-workflow`: this repo already implements the core rule through preflight, worktree isolation, explicit staging, commit, push, script deploy, and live verification. Do not reintroduce direct dirty-workspace deployment or unpushed deploys.
- `rust-shared-target-cache`: never use a relative `CARGO_TARGET_DIR`. Prefer repository scripts for Rust builds; on new machines or suspicious builds, check user/machine `CARGO_TARGET_DIR` and use an absolute Cargo `target-dir`. Do not hardcode one PC's drive letter in shared scripts; use `ELON_BUILD_TARGET_DIR` in the process environment or untracked `.env.local` for machine-specific build caches.
- `p2p-app-distribution`: this repo has APK update broadcast plus same-WiFi peer relay. Preserve `version.json` as the public source of truth, keep direct download fallback, and treat WebSocket Ping/Pong, sender backpressure, and mirror priority semantics as compatibility-sensitive.

## Repository And Git

- Remote: `git@github.com:ElonQian1/Elon.git`
- Main branch: `main`
- Always run the task preflight before editing: `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree` on Windows, or `bash scripts/ai-task-preflight.sh --create-worktree` on Linux/macOS/server CLI. If it creates a worktree, switch to `WORKTREE_PATH` for the task.
- Still check `git status --short --branch` before and after a task.
- Start by fetching remote state. If the workspace is clean, run `git pull --rebase origin main`. If uncommitted changes belong to the current task, stash with `git stash push -u`, rebase, then pop and resolve conflicts. If uncommitted changes are unrelated or unclear, do not stash or rebase that workspace; create a new worktree from `origin/main`.
- Only stage files related to the current task. Never stage unrelated user or agent changes.
- After a task commit, push with `git push origin main` unless the user explicitly says not to.
- Before pushing, fetch/rebase against `origin/main` again so the task commit is based on the latest remote history.
- Commit messages should use conventional prefixes such as `feat`, `fix`, `style`, `refactor`, or `docs`.
- For user-facing code tasks, prefer Chinese commit descriptions that identify the user request when possible.
- Do not use destructive Git commands such as `git reset --hard` or `git checkout --` in the main workspace.

### ⚠️ New files must be explicitly staged (critical lesson)

`git add server/src/main.rs` does NOT automatically include newly created `.rs` files in the same directory.
New files are `untracked` and must be staged separately. Omitting them causes build failures for other developers.

Required check before every commit:
```powershell
# Check for untracked new files — any output means there are files NOT staged
git status --short | Select-String "^\?\?"
# If any output: review and git add each new file that belongs to this task
```

Example of the mistake that breaks builds:
```powershell
# ❌ Added mod homecli_agent; in main.rs but forgot to stage the new file
git add server/src/main.rs
git commit   # homecli_agent.rs not in repo → other devs get: error[E0583]: file not found

# ✅ Correct:
git add server/src/homecli_agent.rs  # new file must be explicit
git add server/src/main.rs
git commit
```

## Concurrent Work Rule

If the main workspace has unrelated uncommitted changes or is behind `origin/main`, avoid editing code in the main workspace. Use the preflight script to create a temporary Git worktree for isolated code changes, then push the finished commit.

Typical isolation flow:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree
Set-Location "<WORKTREE_PATH>"
# edit, verify, commit in the temporary worktree
git push origin main
```

For documentation-only changes that are safely isolated by staging a single file, it is still important to avoid touching unrelated modified files.

## APK Project Task Concurrency

APK-triggered project tasks use a project-scoped execution gate on the server:

- Different `project_id` values can run at the same time.
- The same `project_id` runs one workspace-mutating task at a time and later requests wait in a queue.
- This protects the current shared workspace from concurrent `git pull`, file edits, commits, and pushes.
- Task worktrees are still the target model for same-project parallel coding, but merge to `main`, Android version bumps, APK release publishing, and server deployment remain serialized shared-resource steps.
- The built-in "一龙项目" follows this exact same rule as any other GitHub or `local_path` project.
- Backend Git preflight failures are not final user failures. Pass them to the Codex CLI task context first; let CLI inspect `git status/diff` and try safe recovery. Only if CLI determines the flow cannot be recovered should the app show a friendly blocker.

## Backend And Codex CLI Cooperation

The backend is the workflow orchestrator; Codex CLI is the code executor.

- Before calling Codex CLI, the backend performs hard checks: project identity, workspace path, Git/origin readiness, permissions, queue/lock state, and user-selected model.
- The backend sends Codex CLI a structured task brief every time. The brief must include the user's request, project path, mandatory document-read order, Git rules, validation expectations, and the rule that shared release/deploy steps are serialized.
- APK clients may call `/api/.../prewarm` when a user opens or resumes a project conversation. This is only a best-effort native Codex CLI session warmup: it may create or reuse the session id, but it must not inspect files, run Git, edit code, build, deploy, publish, or inject the full project workflow.
- The backend tracks native Codex session bootstrap state. The first real chat/development turn injects the full rules; later turns in the same native session use a shorter resume prompt. If `codex resume` reports a stale/expired session, mark that native session stale and retry once with a fresh session.
- Fresh-session retry must include a continuity handoff: the old `codex://threads/<thread_id>` URI and recent backend conversation messages, so the new Codex session can reconnect to the prior context instead of starting cold.
- Future non-Codex models may only act as sidecar helpers for lightweight classification, summarization, image/special analysis, or other narrow tasks. They must not become the primary conversation owner. Their output must be summarized back into the same APK conversation's native Codex CLI session so the Codex context remains continuous.
- Codex CLI must inspect the repository and read project documents instead of relying on memory. Unknown projects are handled by reading `AGENTS.md`, `CODEX.md`, `README.md`, `.github/instructions`, and relevant `docs/`; if those files do not exist, use the platform default workflow and recommend adding them.
- After Codex CLI finishes, the backend remains responsible for observable status, task records, download links, release metadata, and any shared locking around merge/version/release/deploy.
- Do not rely on Codex CLI alone for concurrency safety, version ordering, or release publishing. These must be enforced by backend code and scripts.
- Latency debugging is a backend responsibility too. Keep the user/client `trace_id` attached through routing, intent confirmation, prewarm, and the final Codex CLI call. `GET /api/debug/traces/:trace_id` should show `codex_cli_start/done/error/retry` events, prompt size, session hit/miss, operation, attempt, elapsed time, and summary `codex_cli_elapsed_ms`.

## Code Change Workflow

1. Classify the request: Android UI, Android logic, Rust server logic, full-stack, config/text, deployment, or documentation.
2. Locate the exact target files before editing.
3. Read the existing files and surrounding context first.
4. Plan narrowly scoped changes.
5. Edit only the needed files and preserve existing style.
6. Verify locally with the smallest useful command.
7. Commit only the task files.
8. Push to `origin/main`.
9. If deployment is required, deploy from a clean committed state or detached temporary worktree.
10. Report the commit SHA, push status, verification result, and deployment result.

Avoid large mixed commits. If a change touches more than about five files or spans multiple concerns, split it into smaller tasks when practical.

## Verification Commands

Rust backend:

```powershell
cd server
cargo check
```

Android:

```powershell
cd android
.\gradlew.bat lint
.\gradlew.bat assembleDebug
.\gradlew.bat assembleRelease
```

Use the command that matches the risk and scope of the change. Do not deploy code that fails verification.

## Deployment Memory

Server:

- SSH: `root@43.139.149.158`
- Project path: `/root/Elon`
- Binary: `/root/Elon/server/target/release/elon-server`
- Log: `/root/elon-server.log`
- Port: `8080`
- Health check: `curl http://43.139.149.158:8080/health`
- Deploy script (Windows):     `cd scripts && .\publish-server.ps1`
- Deploy script (Linux/macOS): `bash scripts/publish-server.sh`
- Server deploys must compile locally on the developer machine. The production server is low-spec and should only receive the already-built binary plus a restart/health check; do not run release compilation on the server for normal desktop Codex work.
- Per-machine build cache paths must be local configuration, not committed script defaults. `scripts/publish-server.ps1` and `scripts/publish-server.sh` read `ELON_BUILD_TARGET_DIR` from the process environment or root `.env.local`; if unset, the Windows script uses a portable user-local cache directory and the Linux/macOS script keeps the cache under its temporary build worktree.
- Both scripts are functionally identical: git pull → worktree → cross-compile with git SHA → SHA check → upload → restart → verify `/health` and `/api/server/version`

APK:

- Latest APK path on server: `/root/Elon/app/ElonSpeed-latest.apk`
- Download URL: `http://43.139.149.158:8080/app/ElonSpeed-latest.apk`
- APK release builds must run locally and upload `ElonSpeed-latest.apk` plus `version.json`; the production server should not be used as the Android build machine.
- `scripts/publish-apk.ps1` writes APK provenance to server-side `.apk-deployed-sha` and `version.json.gitSha`. If `origin/main` advances during a slow APK build, the script must not rebase and upload the old artifact. If the server already has a deployed SHA containing this build's base commit, stop and test that newer live APK; otherwise rerun the release from latest `main`.
- Remote Codex CLI must treat this as the default APK release decision flow:
  1. Sync to latest `main` before building.
  2. Build only from a known pushed SHA.
  3. If another machine publishes a newer APK that already contains the build's base SHA, stop local release work and verify that live APK instead.
  4. If `origin/main` advances but the live APK does not prove it contains this build's base SHA, do not upload the stale artifact; restart the release from latest `main`.
  5. Never rebase after APK compilation and then upload the old APK; the embedded `BuildConfig.VERSION_CODE` may no longer match `version.json`.
- Release signing keystore is local-only at `D:\一龙\elon-release(1).jks`; do not commit it.
- Keystore type: `PKCS12`; key alias: `elon`.
- For future release APP publishing, set `APK_KEYSTORE` to the JKS path and `APK_KEYSTORE_PASS` to the provided password, then build with `android\gradlew.bat assembleRelease`, run `zipalign`, sign with `apksigner --ks-key-alias elon`, verify with `apksigner verify`, and upload the signed APK to the latest APK server path above.

Backend deploys should be based on a committed SHA. When the main workspace has unrelated uncommitted changes, deploy from a detached temporary worktree based on `HEAD`, not from the dirty main workspace.

Backend server runtime changes must increment `server/Cargo.toml` `package.version` before commit. Use PATCH for fixes, MINOR for backward-compatible features, and MAJOR for incompatible API changes. The server exposes the deployed version at `GET /api/server/version`; deployment scripts inject the git SHA at build time, and the Android APK displays this server version dynamically on the profile page.

### ⚠️ Concurrent deployment protection (SHA ordering)

`scripts/publish-server.ps1` automatically protects against a slow build overwriting a newer deployment:
- After compilation, before binary swap, the script reads `/root/Elon/.deployed-sha` from the server
- If the server already runs a commit that is **newer** than this build, the deploy is aborted
- After a successful deploy, the script writes the current SHA to `/root/Elon/.deployed-sha`

This means: if PC-B deploys v2 while PC-A is still compiling v1, PC-A's deploy will be rejected automatically.
Use `-Force` flag only when you intentionally want to overwrite a newer deployment.

Android APK builds are allowed in the main workspace when necessary, but still commit and push the intended Android changes before building/releasing.

## Security And Safety

- Never commit APK signing keys, `.env`, database passwords, API keys, or other secrets.
- APK signing keys must come from environment variables or deployment configuration.
- Do not hard-code secrets in Rust, Kotlin, Gradle, shell scripts, or docs.
- Do not delete existing features unless the user explicitly asks.
- If verification fails, fix the issue before commit/deploy. If it cannot be fixed safely, stop and report the blocker.
- Preserve traceability: user tasks should have Git history that explains what changed and why.

## Module Map

- `server/`: Rust backend service, API, AI agent, tools, admin/user endpoints.
- `android/`: Android Kotlin client and resources.
- `docs/`: architecture and AI workflow documentation.
- `scripts/`: setup, Android SDK, deploy, and template helper scripts.
- `.github/`: project-wide AI and workflow instructions.
- `.copilot/`: project-specific AI skill instructions.

## Intent Routing / 能力路由

- `server/src/intent_router.rs` is the shared capability router for Web, APK, and future Win clients.
- Always classify user messages before choosing a backend model. Do not duplicate intent checks inside Web/APK handlers.
- Current testing-stage routing defaults to `AI_CODEX_CLI_ONLY=true`: ordinary chat, model configuration discussion, intent-routed execution, image requests, and project/code collaboration all go through Codex CLI. The server ignores non-Codex agent selections and disables API fallback in this mode.
- Codex-only does not mean every message uses the full development workflow. `ChatAgent` requests stay in the same native Codex CLI session but use a lightweight chat prompt that avoids Git checks, document reading, file edits, and release rules. The full project workflow is injected only for `CodeAgent` / development routes.
- When multi-model routing returns later, Codex CLI remains the conversation spine: helper model outputs are treated as sidecar evidence and are injected back into the bound Codex CLI session instead of starting a separate long-running assistant context.
- See `docs/intent-routing.md` before adding new capabilities, providers, or low-cost classifier logic.

## Local Notes

- Chinese docs may display as mojibake in some PowerShell output due to terminal encoding. Inspect carefully before rewriting text-heavy files.
- `.gitignore` changes require extra care. Verify they do not accidentally ignore Rust/Kotlin/source XML files.
