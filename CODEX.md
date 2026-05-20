# CODEX Project Memory

## Project Identity

This project is a cloud APK development platform. Users describe app changes in natural language from the Android APK. The backend AI agent plans code changes, updates the Rust server / Android app / future frontend, builds and deploys, then returns a new APK download link to the user.

Use the repository directory that contains `server/`, `android/`, `docs/`, `scripts/`, `.github/`, and `.copilot/` as the project root.

## Source Documents To Respect

- `.github/copilot-instructions.md`: global project positioning and agent principles.
- `.github/instructions/git-deploy-workflow.instructions.md`: mandatory Git, push, worktree, deploy, and report workflow.
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

- SSH: `ubuntu@182.254.168.75`
- Project path: `/home/ubuntu/Elon`
- Binary: `/home/ubuntu/Elon/server/target/release/elon-server`
- Log: `/home/ubuntu/elon-server.log`
- Port: `8080`
- Health check: `curl http://182.254.168.75:8080/health`

APK:

- Latest APK path on server: `/home/ubuntu/Elon/app/ElonSpeed-latest.apk`
- Download URL: `http://182.254.168.75:8080/app/ElonSpeed-latest.apk`

Backend deploys should be based on a committed SHA. When the main workspace has unrelated uncommitted changes, deploy from a detached temporary worktree based on `HEAD`, not from the dirty main workspace.

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

## Local Notes

- Chinese docs may display as mojibake in some PowerShell output due to terminal encoding. Inspect carefully before rewriting text-heavy files.
- `.gitignore` changes require extra care. Verify they do not accidentally ignore Rust/Kotlin/source XML files.
