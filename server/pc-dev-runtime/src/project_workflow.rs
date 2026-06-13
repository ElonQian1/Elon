use crate::project_scaffold::ProjectScaffoldRequest;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub(crate) fn ensure_project_workflow_files(
    repo: &Path,
    _req: &ProjectScaffoldRequest<'_>,
) -> io::Result<()> {
    ensure_file(
        repo.join("scripts").join("elon-new-task.ps1"),
        new_task_script,
    )?;
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

fn new_task_script() -> io::Result<String> {
    Ok(r#"param(
    [Parameter(Mandatory = $true)]
    [string]$Name,

    [string]$Base = ''
)

$ErrorActionPreference = 'Stop'

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$GitArgs
    )
    $output = & git @GitArgs 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "git $($GitArgs -join ' ') failed: $output"
    }
    return $output
}

function Test-GitSuccess {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$GitArgs
    )
    & git @GitArgs *> $null
    return $LASTEXITCODE -eq 0
}

function New-Slug {
    param([Parameter(Mandatory = $true)][string]$Value)
    $slug = $Value.Trim().ToLowerInvariant() -replace '[^a-z0-9._-]+', '-'
    $slug = $slug.Trim('-')
    if (-not $slug) {
        throw 'Task name must contain at least one letter or number.'
    }
    return $slug
}

$ProjectRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$ProjectParent = Split-Path -Parent $ProjectRoot
$WorktreeRoot = Join-Path $ProjectParent 'task-worktrees'
$Slug = New-Slug $Name
$Branch = "ai/task/$Slug"
$WorktreePath = Join-Path $WorktreeRoot $Slug

Set-Location $ProjectRoot
Invoke-Git @('rev-parse', '--is-inside-work-tree') | Out-Null

if (Test-Path -LiteralPath $WorktreePath) {
    if (Test-GitSuccess @('-C', $WorktreePath, 'rev-parse', '--is-inside-work-tree')) {
        Write-Host "Task worktree already exists: $WorktreePath"
        Write-Host "Branch: $Branch"
        exit 0
    }
    throw "Task path exists but is not a Git worktree: $WorktreePath"
}

New-Item -ItemType Directory -Force -Path $WorktreeRoot | Out-Null

if (Test-GitSuccess @('remote', 'get-url', 'origin')) {
    Invoke-Git @('fetch', 'origin') | Out-Null
}

if ($Base.Trim()) {
    $StartRef = $Base.Trim()
} else {
    $CurrentBranch = (Invoke-Git @('rev-parse', '--abbrev-ref', 'HEAD')).Trim()
    $OriginRef = "origin/$CurrentBranch"
    if ($CurrentBranch -and $CurrentBranch -ne 'HEAD' -and (Test-GitSuccess @('rev-parse', '--verify', $OriginRef))) {
        $StartRef = $OriginRef
    } else {
        $StartRef = 'HEAD'
    }
}

if (Test-GitSuccess @('rev-parse', '--verify', "refs/heads/$Branch")) {
    Invoke-Git @('worktree', 'add', $WorktreePath, $Branch) | Out-Null
} else {
    Invoke-Git @('worktree', 'add', '-b', $Branch, $WorktreePath, $StartRef) | Out-Null
}

Write-Host "Task worktree created: $WorktreePath"
Write-Host "Branch: $Branch"
Write-Host "Base: $StartRef"
"#
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::{ensure_project_workflow_files, new_task_script};
    use crate::project_scaffold::ProjectScaffoldRequest;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn workflow_files_are_created_without_overwrite() {
        let root = temp_dir("workflow_files_are_created_without_overwrite");
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("scripts").join("elon-new-task.ps1"), "custom").unwrap();

        ensure_project_workflow_files(&root, &request()).unwrap();

        assert_eq!(
            fs::read_to_string(root.join("scripts").join("elon-new-task.ps1")).unwrap(),
            "custom"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn new_task_script_creates_sibling_worktrees() {
        let script = new_task_script().unwrap();
        assert!(script.contains("'task-worktrees'"));
        assert!(script.contains("'worktree', 'add'"));
        assert!(script.contains("ai/task/$Slug"));
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
