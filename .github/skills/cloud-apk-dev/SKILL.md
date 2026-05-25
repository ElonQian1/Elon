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
4. `.github/instructions/modular-architecture.instructions.md`
5. `docs/ai-agent-workflow.md`
6. `docs/system-architecture.md`

## Commands And Entry Points

- Common task prompt: `/elon-dev-task`
- APK release prompt: `/elon-apk-release`
- Planning agent: `elon-planner`
- Implementation agent: `elon-implementer`
- Review agent: `elon-reviewer`

## Non-Negotiables

- Start and end with `git status --short --branch`.
- If the main workspace has unrelated uncommitted changes, use a temporary worktree.
- Do not keep adding logic to giant files. For files over 1500 lines, extract the touched responsibility into a focused module unless the change is a tiny fix.
- Never deploy uncommitted code.
- Never stage unrelated files.
- Never commit secrets, `.env`, APK signing keys, or generated private credentials.
- If push is rejected, fetch/rebase or merge, resolve conflicts while preserving both sides when compatible, then push again.
- If uncommitted changes are unrelated or unclear, create a new worktree from `origin/main` instead of pulling in the dirty workspace.
- For backend runtime changes, increment `server/Cargo.toml` `package.version`, deploy with `scripts/publish-server.*`, and verify `/api/server/version`.
- For Android installable features, PR/debug build is not complete. Run `scripts\publish-apk.ps1`, then `scripts\check-task-complete.ps1 -Kind AndroidFeature`, unless the user explicitly says not to publish the APK.
- For Rust builds, do not rely on a relative `CARGO_TARGET_DIR`; use project scripts or an absolute target directory.
- For APK update/P2P work, keep `version.json` as the public source of truth, preserve direct `downloadUrl` fallback, and verify live `/app/version.json` after publishing.
- For Android builds on a new machine, run the speed-test below first before trying `./gradlew`; network misconfiguration will stall downloads indefinitely.

## Android Build Environment Setup (New Machine Only)

Every developer machine has a different network. **Run the speed test first**, then configure accordingly. Skipping this will cause Gradle to silently hang on downloads.

### Step 1: Speed-test download paths

```powershell
$cases = @(
  @{Name='official-noproxy';   Url='https://services.gradle.org/distributions/gradle-8.6-bin.zip'; NoProxy=$true},
  @{Name='tencent-mirror';     Url='https://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip';  NoProxy=$true},
  @{Name='official-with-proxy';Url='https://services.gradle.org/distributions/gradle-8.6-bin.zip'; NoProxy=$false}
)
foreach ($c in $cases) {
  Write-Host "=== $($c.Name) ==="
  $a = @('-L','-r','0-10485759','-o','NUL','-s','-w','speed=%{speed_download}B/s total=%{time_total}s code=%{http_code}\n')
  if ($c.NoProxy) { $a += @('--noproxy','*') }
  $a += $c.Url
  & curl.exe @a
}
```

Decision: use the URL with the highest `speed` and `code=206`. If the official source returns `code=307` with `speed=0`, it is redirecting to GitHub — use the Tencent mirror instead.

### Step 2: Fix a broken Gradle Wrapper cache (if needed)

If `~/.gradle/wrapper/dists/gradle-8.6-bin/` contains only `.part`/`.lck` files (no complete zip), the previous download was interrupted. Re-download using the fastest URL from Step 1:

```powershell
$d = "$HOME\.gradle\wrapper\dists\gradle-8.6-bin\afr5mpiioh2wthjmwnkmdsd5w"
if (!(Test-Path $d)) { New-Item -ItemType Directory -Path $d | Out-Null }
Remove-Item "$d\*.part","$d\*.lck" -ErrorAction SilentlyContinue
curl.exe -L --noproxy '*' -o "$d\gradle-8.6-bin.zip" "https://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip"
```

### Step 3: Configure global Gradle mirrors (once, permanent)

```powershell
# init.gradle — redirect Maven repos to Alibaba Cloud mirrors
# IMPORTANT: Modern AGP sets FAIL_ON_PROJECT_REPOS.
# Use settingsEvaluated to inject dependency repos; do NOT use allprojects { repositories {} }
$initFile = "$HOME\.gradle\init.gradle"
Set-Content $initFile -Encoding UTF8 @'
allprojects {
    buildscript {
        repositories {
            maven { url "https://maven.aliyun.com/repository/google" }
            maven { url "https://maven.aliyun.com/repository/central" }
            maven { url "https://maven.aliyun.com/repository/gradle-plugin" }
            maven { url "https://maven.aliyun.com/repository/public" }
        }
    }
}
settingsEvaluated { settings ->
    settings.dependencyResolutionManagement {
        repositories {
            maven { url "https://maven.aliyun.com/repository/google" }
            maven { url "https://maven.aliyun.com/repository/central" }
            maven { url "https://maven.aliyun.com/repository/gradle-plugin" }
            maven { url "https://maven.aliyun.com/repository/public" }
        }
    }
}
'@

# gradle.properties — disable JVM system proxy (SOCKS proxy causes Maven timeouts)
$props = "$HOME\.gradle\gradle.properties"
if (!(Test-Path $props)) { New-Item -ItemType File -Path $props | Out-Null }
$content = Get-Content $props | Where-Object { $_ -notmatch '^systemProp\.' }
$content += "systemProp.java.net.useSystemProxies=false"
Set-Content $props $content -Encoding UTF8
```

### Verify

```powershell
cd e:\lodex\Elon\android
.\gradlew.bat --version --no-daemon       # should print Gradle version in seconds
.\gradlew.bat :app:assembleRelease --no-daemon   # first run ~2-5 min via Alibaba mirrors
```

> These files go in `~/.gradle/` (user-level). Do **not** commit them — every machine picks its own strategy.
