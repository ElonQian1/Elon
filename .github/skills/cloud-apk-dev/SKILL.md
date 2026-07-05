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

Read `AGENTS.md` first, then follow its task routing table. Do not load every
instruction or docs file by default.

Use this lightweight routing:

- Git, worktree, commit, push, deploy, release, or Cargo verification:
  `.github/instructions/git-deploy-workflow.instructions.md`.
- Large files, refactors, or module boundaries:
  `.github/instructions/modular-architecture.instructions.md`.
- Backend architecture or API/data-flow design:
  `docs/system-architecture.md` plus task-related source.
- Android build environment or Gradle download failures:
  `docs/android-setup.md`.
- Full workflow uncertainty or stuck tasks:
  `docs/ai-agent-workflow.md`.

## Commands And Entry Points

- Common task prompt: `/elon-dev-task`
- APK release prompt: `/elon-apk-release`
- Planning agent: `elon-planner`
- Implementation agent: `elon-implementer`
- Review agent: `elon-reviewer`

## Non-Negotiables

- Start by running the task preflight script: Windows `powershell -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree`; Linux/macOS/server CLI `bash scripts/ai-task-preflight.sh --create-worktree`.
- If preflight prints `WORKTREE_CREATED=true`, change to `WORKTREE_PATH` before reading target files or editing. The `main` checkout is only the shared baseline.
- Start and end with `git status --short --branch`.
- Do not keep adding logic to giant files. For files over 1500 lines, extract the touched responsibility into a focused module unless the change is a tiny fix.
- Never deploy uncommitted code.
- Never stage unrelated files.
- Never commit secrets, `.env`, APK signing keys, or generated private credentials.
- If `git push origin HEAD:main` is rejected with non-fast-forward, run `git fetch origin` and `git rebase origin/main`, resolve conflicts while preserving both sides when compatible, then push again. Do not rebase just because `origin/main` moved during coding or publishing.
- If uncommitted changes are unrelated or unclear, rely on the preflight-created worktree from `origin/main` instead of pulling in the dirty workspace.
- For backend runtime changes, push the business commit to `origin/main` first and run `scripts\check-task-complete.ps1 -Kind CodePushed`; then run `scripts/publish-server.*` when this task owns deployment. The script calls `POST /api/release/claim` so the server atomically allocates a new version number, injects it into the binary at compile time via `ELON_BUILD_VERSION`, deploys, then calls `POST /api/release/finish`. **Do NOT manually edit and commit `server/Cargo.toml` `package.version`** — the field is only a cold-start fallback. If deploy is superseded by newer main/server state, report "code has landed; release is handed to latest main" instead of rebasing and rerunning.
- For Android installable features, PR/debug build is not release complete. Push the business commit first and run `scripts\check-task-complete.ps1 -Kind CodePushed`. Run `scripts\publish-apk.ps1` when this task owns APK publishing (it claims `versionCode/versionName` from the server, temporarily writes `build.gradle`, builds, uploads, then restores `build.gradle`; nothing is committed to git), then run `scripts\check-task-complete.ps1 -Kind AndroidFeature`. If the APK publish is superseded by newer main/server state, report "code has landed; release is handed to latest main" instead of rebasing and rerunning.
- For Rust builds, do not rely on a relative `CARGO_TARGET_DIR`; use project scripts or an absolute target directory.
- For APK update/P2P work, keep `version.json` as the public source of truth, preserve direct `downloadUrl` fallback, and verify live `/app/version.json` after publishing.
- For LAN distribution: after publishing APK, `scripts\publish-apk.ps1` automatically calls `scripts\lan-dist-client.ps1 -ProjectId "elon" -ArtifactId "user-apk" ...` to register the artifact with the shared daemon on port 7788. The daemon serves `GET /dist/<project>/<artifact>`, re-registers with the server every 55 min, and auto-exits when all TTLs expire. Other projects (bb64a, etc.) can join by calling the same script with their own `-ProjectId`/`-ArtifactId`/`-ServerRegisterUrl`. See user skill `p2p-app-distribution` for the full pattern.
- For Android builds on a new machine, read `docs/android-setup.md` and run its speed-test before trying `./gradlew`; network misconfiguration will stall downloads indefinitely.

## Android Build Environment Setup (New Machine Only)

Do not inline the setup commands here. Read `docs/android-setup.md`, run its
download speed test, then apply only the machine-specific fix it recommends.
User-level Gradle files under `~/.gradle/` must not be committed.
