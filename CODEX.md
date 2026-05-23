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

## Repository And Git

- Remote: `git@github.com:ElonQian1/Elon.git`
- Main branch: `main`
- Always check `git status --short --branch` before and after a task.
- Only stage files related to the current task. Never stage unrelated user or agent changes.
- After a task commit, push with `git push origin main` unless the user explicitly says not to.
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

If the main workspace has unrelated uncommitted changes, avoid editing code in the main workspace. Use a temporary Git worktree for isolated code changes, then integrate the finished commit back into `main` with `cherry-pick` and push.

Typical isolation flow:

```powershell
$id = Get-Random -Maximum 9999
git worktree add ..\Elon-session-$id -b ai/session-$id main
# edit and commit in the temporary worktree
git cherry-pick <session_commit_sha>
git push origin main
git worktree remove ..\Elon-session-$id --force
```

For documentation-only changes that are safely isolated by staging a single file, it is still important to avoid touching unrelated modified files.

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

APK:

- Latest APK path on server: `/root/Elon/app/ElonSpeed-latest.apk`
- Download URL: `http://43.139.149.158:8080/app/ElonSpeed-latest.apk`
- Release signing keystore is local-only at `D:\一龙\elon-release(1).jks`; do not commit it.
- Keystore type: `PKCS12`; key alias: `elon`.
- For future release APP publishing, set `APK_KEYSTORE` to the JKS path and `APK_KEYSTORE_PASS` to the provided password, then build with `android\gradlew.bat assembleRelease`, run `zipalign`, sign with `apksigner --ks-key-alias elon`, verify with `apksigner verify`, and upload the signed APK to the latest APK server path above.

Backend deploys should be based on a committed SHA. When the main workspace has unrelated uncommitted changes, deploy from a detached temporary worktree based on `HEAD`, not from the dirty main workspace.

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
- Testing-stage routing: image-related chat/project requests go directly to Codex CLI and must not auto-call `image_generation` or API fallback; app/web/server development prefers Codex CLI; ordinary chat/model configuration stays in chat.
- See `docs/intent-routing.md` before adding new capabilities, providers, or low-cost classifier logic.

## Local Notes

- Chinese docs may display as mojibake in some PowerShell output due to terminal encoding. Inspect carefully before rewriting text-heavy files.
- `.gitignore` changes require extra care. Verify they do not accidentally ignore Rust/Kotlin/source XML files.
