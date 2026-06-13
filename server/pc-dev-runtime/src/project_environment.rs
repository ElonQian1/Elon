use crate::project_scaffold::ProjectScaffoldRequest;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) fn ensure_project_environment_files(
    repo: &Path,
    req: &ProjectScaffoldRequest<'_>,
) -> io::Result<()> {
    ensure_file(repo.join("docs").join("dev-environment.md"), || {
        dev_environment_doc(req)
    })?;
    ensure_file(repo.join("scripts").join("elon-dev-check.ps1"), || {
        powershell_dev_check(req)
    })?;
    Ok(())
}

fn ensure_file(path: PathBuf, content: impl FnOnce() -> io::Result<String>) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content()?)
}

fn dev_environment_doc(req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    let mut doc = format!(
        "# Development Environment\n\nThis project was initialized by Elon PC Dev Runtime.\n\n- project_id: `{}`\n- template: `{}`\n- owner_user_id: `{}`\n\n## Baseline\n\nGit is required for project creation, recovery, migration, and AI task history. The PC node configures repository-local Git identity during provisioning.\n\nRun the local check script after opening the project on a PC node:\n\n```powershell\npowershell -ExecutionPolicy Bypass -File scripts\\elon-dev-check.ps1\n```\n",
        req.project_id, req.template, req.user_id
    );
    if req.template.eq_ignore_ascii_case("android") {
        doc.push_str("\n## Android Projects\n\nRecommended local tools:\n\n- JDK 17+\n- Android SDK with `platforms` and `build-tools`\n- Gradle or the project Gradle wrapper when available\n\nCopy `local.properties.example` to `local.properties` and set `sdk.dir` when Android Studio has not done it for you.\n");
    } else {
        doc.push_str("\n## Optional Toolchains\n\nInstall the toolchain that matches the project you are building. The check script reports Rust, Node, npm, Java, Gradle, and Android SDK availability without requiring any AI CLI.\n");
    }
    Ok(doc)
}

fn powershell_dev_check(req: &ProjectScaffoldRequest<'_>) -> io::Result<String> {
    Ok(format!(
        r#"$ErrorActionPreference = 'Stop'

$ProjectId = '{project_id}'
$Template = '{template}'
$Root = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$HasError = $false

function Test-Tool {{
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [string[]]$Args = @('--version'),
        [switch]$Required
    )
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $cmd) {{
        $level = if ($Required) {{ 'ERROR' }} else {{ 'WARN' }}
        Write-Host "[$level] $Name not found"
        if ($Required) {{ $script:HasError = $true }}
        return
    }}
    $version = ''
    try {{
        $output = & $Name @Args 2>&1 | Select-Object -First 1
        if ($output) {{ $version = " - $output" }}
    }} catch {{
        $version = " - version check failed: $($_.Exception.Message)"
    }}
    Write-Host "[OK] $Name -> $($cmd.Source)$version"
}}

Write-Host "Elon PC Dev Runtime check"
Write-Host "Project: $ProjectId"
Write-Host "Template: $Template"
Write-Host "Root: $Root"
Write-Host ''

if (-not (Test-Path -LiteralPath (Join-Path $Root '.git'))) {{
    Write-Host '[ERROR] .git directory missing; project recovery and task history need Git.'
    $HasError = $true
}}

Test-Tool -Name git -Required
Test-Tool -Name java -Args @('-version')
Test-Tool -Name gradle
Test-Tool -Name node
Test-Tool -Name npm
Test-Tool -Name rustc
Test-Tool -Name cargo

$sdk = $env:ANDROID_HOME
if (-not $sdk) {{ $sdk = $env:ANDROID_SDK_ROOT }}
if (-not $sdk -and $env:LOCALAPPDATA) {{
    $sdk = Join-Path $env:LOCALAPPDATA 'Android\Sdk'
}}
if ($sdk -and (Test-Path -LiteralPath (Join-Path $sdk 'platforms')) -and (Test-Path -LiteralPath (Join-Path $sdk 'build-tools'))) {{
    Write-Host "[OK] android_sdk -> $sdk"
}} elseif ($Template -eq 'android') {{
    Write-Host '[WARN] Android SDK not detected. Set ANDROID_HOME or local.properties sdk.dir before building Android projects.'
}} else {{
    Write-Host '[INFO] Android SDK not detected.'
}}

if ($HasError) {{ exit 1 }}
exit 0
"#,
        project_id = ps_single_quote(req.project_id),
        template = ps_single_quote(req.template),
    ))
}

fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::{ensure_project_environment_files, powershell_dev_check};
    use crate::project_scaffold::ProjectScaffoldRequest;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn environment_files_are_created_without_overwrite() {
        let root = temp_dir("environment_files_are_created_without_overwrite");
        fs::create_dir_all(root.join("docs")).unwrap();
        fs::write(root.join("docs").join("dev-environment.md"), "custom").unwrap();

        ensure_project_environment_files(&root, &request()).unwrap();

        assert_eq!(
            fs::read_to_string(root.join("docs").join("dev-environment.md")).unwrap(),
            "custom"
        );
        assert!(root.join("scripts").join("elon-dev-check.ps1").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn powershell_script_escapes_project_values() {
        let script = powershell_dev_check(&ProjectScaffoldRequest {
            project_id: "project'one",
            user_id: "user",
            name: "Demo",
            template: "android",
            repo_url: None,
            branch: None,
        })
        .unwrap();
        assert!(script.contains("$ProjectId = 'project''one'"));
        assert!(script.contains("function Test-Tool {"));
        assert!(script.contains("Test-Tool -Name git -Required"));
        assert!(!script.contains("{{"));
        assert!(!script.contains("}}"));
    }

    fn request() -> ProjectScaffoldRequest<'static> {
        ProjectScaffoldRequest {
            project_id: "project-1",
            user_id: "user-1",
            name: "Demo App",
            template: "android",
            repo_url: None,
            branch: None,
        }
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elon-pc-dev-runtime-{label}-{nanos}"))
    }
}
