# Android 编译环境首次配置

> 每台新机器只需配置一次。配置完成后，`./gradlew` 会通过阿里云镜像自动下载依赖，无需额外操作。

---

## 第一步：测速（选择最快的下载路径）

```powershell
# 分别测试官方直连、官方不走代理、腾讯镜像，取 speed 最大的
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

判断标准：
- `speed` 最大（> 3MB/s）且 `code=206` → 使用那条路径
- 官方源 `code=307` 且 speed=0 → 说明最终跳转到 GitHub，国内基本不可用，必须用镜像
- `code=000` → 路由不通

---

## 第二步：修复 Gradle Wrapper 缓存（如果卡下载）

如果 `~/.gradle/wrapper/dists/gradle-8.6-bin/` 下只有 `.part/.lck` 文件（无完整 zip），说明历史下载中断，必须手动用镜像重新灌入：

```powershell
$d = "$HOME\.gradle\wrapper\dists\gradle-8.6-bin\afr5mpiioh2wthjmwnkmdsd5w"
if (!(Test-Path $d)) { New-Item -ItemType Directory -Path $d | Out-Null }
Remove-Item "$d\*.part","$d\*.lck" -ErrorAction SilentlyContinue
# 按测速结果选择最快的 URL（中国大陆一般是腾讯镜像）
curl.exe -L --noproxy '*' -o "$d\gradle-8.6-bin.zip" "https://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip"
```

---

## 第三步：配置全局 Gradle 镜像（一次性，永久生效）

```powershell
# 1. 创建 ~/.gradle/init.gradle — 重定向所有 Maven 仓库到阿里云
# 注意：现代 AGP 用 FAIL_ON_PROJECT_REPOS，需用 settingsEvaluated 注入依赖仓库
$initFile = "$HOME\.gradle\init.gradle"
Set-Content $initFile -Encoding UTF8 @'
// buildscript classpath（插件解析）走 allprojects.buildscript
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
// 依赖仓库通过 settingsEvaluated 注入，避免与 FAIL_ON_PROJECT_REPOS 冲突
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

# 2. 向 ~/.gradle/gradle.properties 添加禁用 JVM 系统代理
# （JVM 会读取系统 SOCKS 代理导致访问超时，必须关闭）
$props = "$HOME\.gradle\gradle.properties"
if (!(Test-Path $props)) { New-Item -ItemType File -Path $props | Out-Null }
$content = Get-Content $props | Where-Object { $_ -notmatch '^systemProp\.' }
$content += "systemProp.java.net.useSystemProxies=false"
Set-Content $props $content -Encoding UTF8
```

---

## 验证

```powershell
cd e:\lodex\Elon\android
.\gradlew.bat --version --no-daemon   # 应在几秒内输出 Gradle 版本，无下载提示
.\gradlew.bat :app:assembleRelease --no-daemon   # 首次编译会下载插件/依赖，通过阿里云镜像约 2-5 分钟
```

> **为什么不把镜像配置提交到仓库？**
> `init.gradle` 写入用户级 `~/.gradle/`，不进入 git，不影响其他团队成员或 CI 环境。
> 每台机器自行按网络测速决定镜像策略，符合"本地环境自治"原则。

---

## Windows 中文路径下的单元测试

项目已内置兼容层。Windows 工作目录含中文字符时，Gradle `Test` 任务会自动把项目内的测试运行时 classpath 映射到 Gradle 用户目录中的纯英文缓存路径；APK 编译、Release 构建和英文路径下的测试不受影响。

因此不要再为单元测试手工创建 `subst` 盘符，也不需要添加临时 Gradle init script。每次实际执行 `Test` 任务时会刷新约 17 MB 的本地映射，通常只增加不到 1 秒；Gradle 的编译增量仍正常工作。

如果 Windows 用户目录本身也含中文，先指定一个纯英文绝对路径：

```powershell
$env:ELON_GRADLE_TEST_ASCII_ROOT = "C:\GradleTestRuntime"
.\gradlew.bat :app:testDebugUnitTest
```

兼容层入口为 `android/gradle/windows-unicode-test-classpath.gradle`。看到“Windows 中文路径兼容”日志，表示映射已自动启用。
