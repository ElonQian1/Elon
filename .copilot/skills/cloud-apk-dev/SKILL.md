---
description: >
  云端APK开发平台的 AI 代码修改与部署工作流。
  当用户要求修改 APK 功能、修改服务端 Rust 代码、修改前端代码、
  触发编译构建、部署服务器、分发 APK 下载链接时，使用此技能。
  适用场景：理解需求 → 定位代码 → 修改 → git提交 → 编译 → 部署 → 反馈用户。
---

# 云端 APK 开发平台 — AI 代码修改部署技能

## 何时使用此技能

- 用户描述了一个功能需求，需要修改 APK 或服务端代码
- 需要触发自动化编译和部署流程
- 需要生成 APK 下载链接并推送给用户
- 需要理解本项目的代码结构才能正确修改

## 执行步骤摘要

1. **分析需求** → 判断涉及哪些模块（Android / Rust / 前端）
2. **读取目标文件** → 理解现有代码结构
3. **生成修改方案** → 精确定位要改的内容
4. **执行修改** → 使用精确替换，保持代码风格
5. **语法检查** → `cargo check` / `./gradlew lint` / `npm run lint`
6. **git commit + push** → 只提交本任务文件，推送到 `origin/main`
7. **触发编译/发布** → 后端用 `scripts/publish-server.ps1|sh`，APK 用 `scripts/publish-apk.ps1`
8. **签名APK** → 使用环境变量或用户 Gradle 配置中的密钥，不硬编码
9. **部署上线** → 必须基于已提交、已推送的 SHA
10. **推送结果** → 汇报 SHA、验证结果、APK 版本和下载链接

## 关键规则

- 修改前必须先读文件，不允许盲改
- 有其他任务或来源不明的未提交改动时，必须从 `origin/main` 新建 worktree，不在脏工作区硬拉远端
- 服务端运行代码变更必须递增 `server/Cargo.toml` 的 `version`，部署后校验 `/api/server/version`
- Rust 构建不得依赖相对路径 `CARGO_TARGET_DIR`；优先使用项目脚本或绝对 target 目录
- APK 签名密钥只能来自 `ELON_RELEASE_*` 环境变量或用户级 Gradle 配置，不能硬编码或提交到仓库
- 每次任务必须有 git commit 记录
- APK 更新/P2P 分发以公网 `/app/version.json` 为事实来源；保留 `downloadUrl` 直链兜底，修改 mirrors/peer relay 后必须发布 APK 并校验线上版本
- 新机器首次 Android 编译前必须先测速再配置 Gradle 镜像，否则构建会因下载卡死

## 🌐 Android 编译环境首次配置（每台新机器必做）

每台远程开发机网络环境不同，**必须先测速再决定下载方式**，否则 Gradle 构建会因下载卡死。

### 第一步：测速

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

判断：`code=206` 且 speed 最大 → 使用该路径；官方源 `code=307 speed=0` → 跳到 GitHub，国内不可用，必须用镜像。

### 第二步：修复 Wrapper 缓存（如卡下载）

```powershell
$d = "$HOME\.gradle\wrapper\dists\gradle-8.6-bin\afr5mpiioh2wthjmwnkmdsd5w"
if (!(Test-Path $d)) { New-Item -ItemType Directory -Path $d | Out-Null }
Remove-Item "$d\*.part","$d\*.lck" -ErrorAction SilentlyContinue
curl.exe -L --noproxy '*' -o "$d\gradle-8.6-bin.zip" "https://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip"
```

### 第三步：全局 Gradle 镜像（永久生效）

```powershell
# init.gradle — 注意：AGP 7+ 用 FAIL_ON_PROJECT_REPOS，必须用 settingsEvaluated，不能用 allprojects { repositories {} }
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
# gradle.properties — 禁用 JVM SOCKS 代理（否则 Maven 下载超时）
$props = "$HOME\.gradle\gradle.properties"
if (!(Test-Path $props)) { New-Item -ItemType File -Path $props | Out-Null }
$content = Get-Content $props | Where-Object { $_ -notmatch '^systemProp\.' }
$content += "systemProp.java.net.useSystemProxies=false"
Set-Content $props $content -Encoding UTF8
```

### 验证

```powershell
cd e:\lodex\Elon\android
.\gradlew.bat --version --no-daemon
.\gradlew.bat :app:assembleRelease --no-daemon   # 首次通过阿里云镜像约 2-5 分钟
```

> `init.gradle` 写在 `~/.gradle/`（用户级），不进入 git，不影响 CI 和其他成员。

## 详细流程

完整的分步骤操作流程见：`docs/ai-agent-workflow.md`
系统架构和代码结构见：`docs/system-architecture.md`
