use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

const GRADLE_WRAPPER_JAR: &[u8] =
    include_bytes!("../../../android/gradle/wrapper/gradle-wrapper.jar");
const GRADLEW_BAT: &str = include_str!("../../../android/gradlew.bat");
const GRADLEW_SH: &str = r#"#!/bin/sh
APP_HOME=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec java -classpath "$APP_HOME/gradle/wrapper/gradle-wrapper.jar" org.gradle.wrapper.GradleWrapperMain "$@"
"#;

pub(crate) fn pc_apk_sync_script(
    fresh_after_unix_secs: Option<u64>,
    build_if_missing: bool,
) -> String {
    let freshness_filter = fresh_after_unix_secs
        .map(|secs| {
            format!(
                "$minModifiedUtc = [DateTimeOffset]::FromUnixTimeSeconds({secs}).UtcDateTime\n$files = @($files | Where-Object {{ $_.LastWriteTimeUtc -ge $minModifiedUtc }})"
            )
        })
        .unwrap_or_default();
    let build_flag = if build_if_missing { "$true" } else { "$false" };

    SCRIPT_TEMPLATE
        .replace("__ELON_BUILD_IF_MISSING__", build_flag)
        .replace("__ELON_FRESHNESS_FILTER__", &freshness_filter)
        .replace("__ELON_WRAPPER_JAR_B64__", &B64.encode(GRADLE_WRAPPER_JAR))
        .replace(
            "__ELON_GRADLEW_BAT_B64__",
            &B64.encode(GRADLEW_BAT.as_bytes()),
        )
        .replace(
            "__ELON_GRADLEW_SH_B64__",
            &B64.encode(GRADLEW_SH.as_bytes()),
        )
}

pub(crate) fn pc_apk_sync_loader_command(
    public_url: &str,
    fresh_after_unix_secs: Option<u64>,
    build_if_missing: bool,
) -> String {
    let mut url = format!(
        "{}/api/agent/scripts/pc-apk-sync.ps1?build_if_missing={}",
        public_url.trim_end_matches('/'),
        if build_if_missing { "true" } else { "false" }
    );
    if let Some(secs) = fresh_after_unix_secs {
        url.push_str("&fresh_after_unix_secs=");
        url.push_str(&secs.to_string());
    }
    let url = powershell_single_quoted(&url);
    format!(
        "$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $u='{url}'; $s=(Invoke-WebRequest -UseBasicParsing -Uri $u).Content; Invoke-Expression $s"
    )
}

fn powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

const SCRIPT_TEMPLATE: &str = r#"
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$BuildIfMissing = __ELON_BUILD_IF_MISSING__
$WrapperJarB64 = '__ELON_WRAPPER_JAR_B64__'
$GradlewBatB64 = '__ELON_GRADLEW_BAT_B64__'
$GradlewShB64 = '__ELON_GRADLEW_SH_B64__'
$script:BootstrapTouched = $false

function Find-LatestApk {
  $roots = @(
    (Join-Path (Get-Location) 'app\build\outputs\apk'),
    (Join-Path (Get-Location) 'android\app\build\outputs\apk'),
    (Join-Path (Get-Location) 'build'),
    (Join-Path (Get-Location) 'artifacts')
  )
  $files = @()
  foreach ($root in $roots) {
    if (Test-Path -LiteralPath $root) {
      $files += Get-ChildItem -LiteralPath $root -Recurse -Filter *.apk -File -ErrorAction SilentlyContinue
    }
  }
__ELON_FRESHNESS_FILTER__
  return ($files | Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1)
}

function Resolve-AndroidBuildRoot {
  $cwd = (Get-Location).Path
  foreach ($candidate in @($cwd, (Join-Path $cwd 'android'))) {
    if (-not (Test-Path -LiteralPath $candidate -PathType Container)) { continue }
    $markers = @(
      'settings.gradle',
      'settings.gradle.kts',
      'app\build.gradle',
      'app\build.gradle.kts',
      'app\src\main\AndroidManifest.xml',
      'app\src\main\java',
      'app\src\main\kotlin',
      'app\src\main\res'
    )
    foreach ($marker in $markers) {
      if (Test-Path -LiteralPath (Join-Path $candidate $marker)) {
        return (Resolve-Path -LiteralPath $candidate).Path
      }
    }
  }
  return $null
}

function Write-TextIfMissing {
  param([string]$Path, [string]$Content)
  if (Test-Path -LiteralPath $Path) { return }
  $parent = Split-Path -Parent $Path
  if ($parent -and -not (Test-Path -LiteralPath $parent)) {
    New-Item -ItemType Directory -Path $parent | Out-Null
  }
  [System.IO.File]::WriteAllText($Path, $Content, [System.Text.UTF8Encoding]::new($false))
  $script:BootstrapTouched = $true
}

function Write-BytesIfMissing {
  param([string]$Path, [string]$B64)
  if (Test-Path -LiteralPath $Path) { return }
  $parent = Split-Path -Parent $Path
  if ($parent -and -not (Test-Path -LiteralPath $parent)) {
    New-Item -ItemType Directory -Path $parent | Out-Null
  }
  [System.IO.File]::WriteAllBytes($Path, [Convert]::FromBase64String($B64))
  $script:BootstrapTouched = $true
}

function Resolve-AppPackage {
  $manifest = Join-Path (Get-Location) 'app\src\main\AndroidManifest.xml'
  if (Test-Path -LiteralPath $manifest) {
    $text = Get-Content -LiteralPath $manifest -Raw -ErrorAction SilentlyContinue
    if ($text -match 'package\s*=\s*"([^"]+)"') { return $Matches[1] }
  }
  $srcRoot = Join-Path (Get-Location) 'app\src\main'
  if (Test-Path -LiteralPath $srcRoot) {
    $file = Get-ChildItem -LiteralPath $srcRoot -Recurse -File -Include MainActivity.java,MainActivity.kt -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($file) {
      $line = Get-Content -LiteralPath $file.FullName -TotalCount 20 -ErrorAction SilentlyContinue |
        Where-Object { $_ -match '^\s*package\s+([A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)*)\s*;?' } |
        Select-Object -First 1
      if ($line -and $Matches[1]) { return $Matches[1] }
    }
  }
  return 'com.elon.generated'
}

function Ensure-AndroidBuildBootstrap {
  $packageName = Resolve-AppPackage
  if ($packageName -notmatch '^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*)+$') {
    $packageName = 'com.elon.generated'
  }
  Write-TextIfMissing 'settings.gradle' @"
pluginManagement {
    repositories {
        maven { url 'https://maven.aliyun.com/repository/google' }
        maven { url 'https://maven.aliyun.com/repository/central' }
        maven { url 'https://maven.aliyun.com/repository/gradle-plugin' }
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        maven { url 'https://maven.aliyun.com/repository/google' }
        maven { url 'https://maven.aliyun.com/repository/central' }
        google()
        mavenCentral()
    }
}
rootProject.name = 'ElonUserApp'
include ':app'
"@
  Write-TextIfMissing 'build.gradle' @"
plugins {
    id 'com.android.application' version '8.4.0' apply false
}
"@
  Write-TextIfMissing 'gradle.properties' @"
android.useAndroidX=false
org.gradle.jvmargs=-Xmx1536m -Dfile.encoding=UTF-8
"@
  Write-TextIfMissing 'app\build.gradle' @"
plugins {
    id 'com.android.application'
}

android {
    namespace '$packageName'
    compileSdk 34

    defaultConfig {
        applicationId "$packageName"
        minSdk 26
        targetSdk 34
        versionCode 1
        versionName "1.0"
    }
}
"@
  Write-TextIfMissing 'app\src\main\AndroidManifest.xml' @"
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <application
        android:allowBackup="true"
        android:label="@string/app_name"
        android:theme="@style/AppTheme">
        <activity
            android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"@
  $packagePath = $packageName.Replace('.', '\')
  Write-TextIfMissing (Join-Path 'app\src\main\java' (Join-Path $packagePath 'MainActivity.java')) @"
package $packageName;

import android.app.Activity;
import android.os.Bundle;

public class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
    }
}
"@
  Write-TextIfMissing 'app\src\main\res\layout\activity_main.xml' @"
<?xml version="1.0" encoding="utf-8"?>
<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
    android:layout_width="match_parent"
    android:layout_height="match_parent"
    android:gravity="center"
    android:orientation="vertical">
    <TextView
        android:layout_width="wrap_content"
        android:layout_height="wrap_content"
        android:text="@string/app_name"
        android:textSize="24sp" />
</LinearLayout>
"@
  Write-TextIfMissing 'app\src\main\res\values\strings.xml' @"
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">Elon App</string>
    <style name="AppTheme" parent="android:style/Theme.Material.Light.NoActionBar" />
</resources>
"@
  Write-TextIfMissing 'gradle\wrapper\gradle-wrapper.properties' @"
distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip
networkTimeout=10000
validateDistributionUrl=false
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
"@
  Write-TextIfMissing 'gradlew.bat' ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($GradlewBatB64)))
  Write-TextIfMissing 'gradlew' ([Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($GradlewShB64)))
  Write-BytesIfMissing 'gradle\wrapper\gradle-wrapper.jar' $WrapperJarB64
}

function Invoke-GitQuiet {
  param([string[]]$GitArgs)
  $oldPreference = $ErrorActionPreference
  $hadNativePreference = Test-Path Variable:\PSNativeCommandUseErrorActionPreference
  if ($hadNativePreference) {
    $oldNativePreference = $PSNativeCommandUseErrorActionPreference
    $PSNativeCommandUseErrorActionPreference = $false
  }
  $ErrorActionPreference = 'Continue'
  try {
    & git @GitArgs *> $null
    return $LASTEXITCODE
  } catch {
    return 1
  } finally {
    $ErrorActionPreference = $oldPreference
    if ($hadNativePreference) {
      $PSNativeCommandUseErrorActionPreference = $oldNativePreference
    }
  }
}

function Commit-BootstrapIfGit {
  if (-not $script:BootstrapTouched) { return }
  try {
    if ((Invoke-GitQuiet -GitArgs @('rev-parse', '--is-inside-work-tree')) -ne 0) { return }
    $paths = @(
      'settings.gradle', 'build.gradle', 'gradle.properties', 'app\build.gradle',
      'app\src\main\AndroidManifest.xml', 'app\src\main\java', 'app\src\main\res',
      'gradlew.bat', 'gradlew', 'gradle\wrapper\gradle-wrapper.properties',
      'gradle\wrapper\gradle-wrapper.jar'
    ) | Where-Object { Test-Path -LiteralPath $_ }
    if (-not $paths.Count) { return }
    [void](Invoke-GitQuiet -GitArgs @('config', 'user.name', 'Elon PC Node'))
    [void](Invoke-GitQuiet -GitArgs @('config', 'user.email', 'node@elon.local'))
    [void](Invoke-GitQuiet -GitArgs (@('add', '-f', '--') + $paths))
    [void](Invoke-GitQuiet -GitArgs @('commit', '-m', 'chore(build): ensure Android debug build pipeline'))
  } catch {
    Write-Output ('ELON_APK_BOOTSTRAP_COMMIT_SKIPPED:' + $_.Exception.Message)
  }
}

function Invoke-AndroidDebugBuild {
  $buildRoot = Resolve-AndroidBuildRoot
  if (-not $buildRoot) {
    Write-Output 'ELON_APK_BUILD_SKIPPED:no android project markers'
    return
  }
  Push-Location $buildRoot
  try {
    Ensure-AndroidBuildBootstrap
    Commit-BootstrapIfGit
    $file = $null
    $args = @(':app:assembleDebug', '--no-daemon', '--stacktrace')
    if (Test-Path -LiteralPath 'gradlew.bat') {
      $file = '.\gradlew.bat'
    } elseif (Test-Path -LiteralPath 'gradlew') {
      if (Get-Command bash -ErrorAction SilentlyContinue) {
        $file = 'bash'
        $args = @('./gradlew') + $args
      } else {
        $file = '.\gradlew'
      }
    } elseif (Get-Command gradle -ErrorAction SilentlyContinue) {
      $file = 'gradle'
    } else {
      Write-Output 'ELON_APK_BUILD_SKIPPED:no gradle entry'
      return
    }
    $log = Join-Path ([System.IO.Path]::GetTempPath()) ('elon-gradle-build-' + [Guid]::NewGuid().ToString('N') + '.log')
    & $file @args *> $log
    $code = $LASTEXITCODE
    if ($code -ne 0) {
      $tail = if (Test-Path -LiteralPath $log) { (Get-Content -LiteralPath $log -Tail 80 -ErrorAction SilentlyContinue) -join "`n" } else { '' }
      throw "Android debug build failed with exit code $code`n$tail"
    }
  } finally {
    Pop-Location
  }
}

$apk = if ($BuildIfMissing) { $null } else { Find-LatestApk }
if ($BuildIfMissing) {
  Write-Output 'ELON_APK_BUILD_ATTEMPT_BEGIN'
  Invoke-AndroidDebugBuild
  Write-Output 'ELON_APK_BUILD_ATTEMPT_END'
  $apk = Find-LatestApk
}
if (-not $apk) { exit 2 }
if ($apk.Length -gt 104857600) { Write-Error 'APK too large to relay'; exit 3 }
Write-Output ('ELON_APK_NAME:' + $apk.Name)
Write-Output 'ELON_APK_BASE64_BEGIN'
$payload = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes($apk.FullName))
$chunkSize = 65536
for ($offset = 0; $offset -lt $payload.Length; $offset += $chunkSize) {
  $length = [Math]::Min($chunkSize, $payload.Length - $offset)
  Write-Output $payload.Substring($offset, $length)
}
Write-Output 'ELON_APK_BASE64_END'
"#;

#[cfg(test)]
mod tests {
    use super::{pc_apk_sync_loader_command, pc_apk_sync_script};

    #[test]
    fn apk_sync_script_can_enable_build_fallback() {
        let script = pc_apk_sync_script(None, true);
        assert!(script.contains("$BuildIfMissing = $true"));
        assert!(script.contains("$apk = if ($BuildIfMissing) { $null } else { Find-LatestApk }"));
        assert!(script.contains("$chunkSize = 65536"));
        assert!(script.contains("Ensure-AndroidBuildBootstrap"));
        assert!(script.contains("Invoke-GitQuiet"));
        assert!(script.contains("ELON_APK_BOOTSTRAP_COMMIT_SKIPPED"));
        assert!(script.contains("gradle-wrapper.jar"));
    }

    #[test]
    fn apk_sync_script_keeps_freshness_filter() {
        let script = pc_apk_sync_script(Some(42), false);
        assert!(script.contains("$BuildIfMissing = $false"));
        assert!(script.contains("FromUnixTimeSeconds(42)"));
    }

    #[test]
    fn apk_sync_loader_command_stays_short() {
        let command = pc_apk_sync_loader_command("http://example.test/", Some(42), true);

        assert!(command.len() < 512);
        assert!(command.contains("/api/agent/scripts/pc-apk-sync.ps1"));
        assert!(command.contains("build_if_missing=true"));
        assert!(command.contains("fresh_after_unix_secs=42"));
        assert!(!command.contains("gradle-wrapper.jar"));
    }

    #[test]
    fn apk_sync_loader_command_escapes_single_quote() {
        let command = pc_apk_sync_loader_command("http://example.test/a'b", None, false);

        assert!(command.contains("a''b"));
        assert!(command.contains("build_if_missing=false"));
    }
}
