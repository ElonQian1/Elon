param()

$ErrorActionPreference = "Stop"

function Invoke-Captured {
    param(
        [string]$Command,
        [string[]]$Arguments
    )

    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $Command @Arguments 2>&1
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $oldPreference
    }
    return [pscustomobject]@{
        ExitCode = $exitCode
        Text = (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    }
}

function Assert-Contains {
    param([string]$Text, [string]$Expected, [string]$Message)
    if (-not $Text.Contains($Expected)) {
        throw "$Message Missing: $Expected`nActual:`n$Text"
    }
}

function Resolve-BashCommand {
    # Prefer the Bash shipped with the active Git installation. WindowsApps
    # exposes WSL's bash.exe on many machines; that binary cannot consume the
    # Git-Bash-style C:/... script path used by this cross-platform guard.
    $gitCommand = Get-Command git -ErrorAction Stop
    $gitRoot = Split-Path (Split-Path $gitCommand.Source -Parent) -Parent
    foreach ($candidate in @(
        (Join-Path $gitRoot "bin\bash.exe"),
        (Join-Path $gitRoot "usr\bin\bash.exe")
    )) {
        if (Test-Path -LiteralPath $candidate) {
            return $candidate
        }
    }

    $available = Get-Command bash -ErrorAction SilentlyContinue
    if ($available) {
        return $available.Source
    }
    throw "bash is required to verify scripts/format-rust.sh."
}

$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
$psScript = Join-Path $repoRoot "scripts\format-rust.ps1"
$shScript = Join-Path $repoRoot "scripts\format-rust.sh"
$bashCommand = Resolve-BashCommand

$psRefusal = Invoke-Captured -Command "powershell" -Arguments @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $psScript, "-Apply"
)
if ($psRefusal.ExitCode -ne 2) {
    throw "PowerShell formatter must reject implicit full writes with exit 2.`n$($psRefusal.Text)"
}
Assert-Contains $psRefusal.Text "-Apply -All" "PowerShell refusal must show the explicit full-write command."

$shRefusal = Invoke-Captured -Command $bashCommand -Arguments @(($shScript -replace '\\', '/'), "--apply")
if ($shRefusal.ExitCode -ne 2) {
    throw "Shell formatter must reject implicit full writes with exit 2.`n$($shRefusal.Text)"
}
Assert-Contains $shRefusal.Text "--apply --all" "Shell refusal must show the explicit full-write command."

$psContent = Get-Content -Raw -LiteralPath $psScript -Encoding UTF8
$shContent = Get-Content -Raw -LiteralPath $shScript -Encoding UTF8
Assert-Contains $psContent "Test-FullFormatClean" "PowerShell full apply must verify convergence."
Assert-Contains $psContent 'for ($pass = 1; $pass -le 3; $pass++)' "PowerShell full apply must retry until idempotent."
Assert-Contains $psContent "status --porcelain=v1 --untracked-files=all" "PowerShell full apply must require a clean worktree."
Assert-Contains $shContent "full_format_clean" "Shell full apply must verify convergence."
Assert-Contains $shContent "for pass in 1 2 3" "Shell full apply must retry until idempotent."
Assert-Contains $shContent "git status --porcelain=v1 --untracked-files=all" "Shell full apply must require a clean worktree."

# Exercise the real entry point in isolated minimal crates. The desktop child
# remains deliberately unformatted to prove both default-scope exclusion and
# skip_children protection without touching any developer's Rust files.
$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("elon-rustfmt-directed-" + [guid]::NewGuid().ToString("N"))
$fixtureFullPath = [System.IO.Path]::GetFullPath($fixtureRoot)
$tempPrefix = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
$utf8 = New-Object System.Text.UTF8Encoding($false)
function Write-FixtureFile {
    param([string]$Relative, [string]$Text)
    $target = Join-Path $fixtureFullPath $Relative
    [System.IO.Directory]::CreateDirectory((Split-Path $target -Parent)) | Out-Null
    [System.IO.File]::WriteAllText($target, $Text, $utf8)
}
function Invoke-FixtureFormat {
    param([string[]]$FormatArguments = @())
    return Invoke-Captured -Command "powershell" -Arguments (@(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Join-Path $fixtureFullPath "scripts/format-rust.ps1")
    ) + $FormatArguments)
}
try {
    Write-FixtureFile "scripts/format-rust.ps1" $psContent
    Write-FixtureFile ".rustfmt-version" (Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".rustfmt-version"))
    Write-FixtureFile "rust-toolchain.toml" (Get-Content -Raw -LiteralPath (Join-Path $repoRoot "rust-toolchain.toml"))
    $defaultRoots = @("server", "server/pc-dev-runtime", "server/homecli-proto", "server/tests/esk-platform-harness", "server/tests/account-https-harness", "tools/esk-paper-contract-tests")
    $allFixtureRoots = @($defaultRoots) + @("desktop-shell/src-tauri")
    for ($index = 0; $index -lt $allFixtureRoots.Count; $index++) {
        $root = $allFixtureRoots[$index]
        Write-FixtureFile "$root/Cargo.toml" "[package]`nname = `"format_fixture_$index`"`nversion = `"0.1.0`"`nedition = `"2021`"`n`n[workspace]`n"
        Write-FixtureFile "$root/src/lib.rs" "pub fn baseline() {}`n"
    }
    $desktopFile = "desktop-shell/src-tauri/src/lib.rs"
    $childFile = Join-Path $fixtureFullPath "desktop-shell/src-tauri/src/nested.rs"
    Write-FixtureFile $desktopFile "mod nested;pub async fn ready()->u8{1}`n"
    Write-FixtureFile "desktop-shell/src-tauri/src/nested.rs" "pub fn untouched()->u8{2}`n"
    $childBefore = [System.IO.File]::ReadAllText($childFile)

    $directedCheck = Invoke-FixtureFormat -FormatArguments @("-Files", $desktopFile)
    if ($directedCheck.ExitCode -ne 1) { throw "Desktop check must report formatting differences with exit 1.`n$($directedCheck.Text)" }
    Assert-Contains $directedCheck.Text "edition 2021" "Desktop edition must come from its manifest."
    $directedApply = Invoke-FixtureFormat -FormatArguments @("-Apply", "-Files", $desktopFile)
    if ($directedApply.ExitCode -ne 0) { throw "Explicit desktop formatting failed.`n$($directedApply.Text)" }
    if ([System.IO.File]::ReadAllText($childFile) -ne $childBefore) { throw "Directed desktop formatting changed an unrequested child module." }
    $directedClean = Invoke-FixtureFormat -FormatArguments @("-Files", (Join-Path $fixtureFullPath $desktopFile))
    if ($directedClean.ExitCode -ne 0) { throw "Directed desktop format check must accept an absolute in-repository path.`n$($directedClean.Text)" }

    $defaultCheck = Invoke-FixtureFormat
    if ($defaultCheck.ExitCode -ne 0) { throw "Default check must retain its existing crate scope.`n$($defaultCheck.Text)" }
    foreach ($root in $defaultRoots) { Assert-Contains $defaultCheck.Text "Checking $root/Cargo.toml" "Default format scope lost a crate." }
    if ($defaultCheck.Text.Contains("desktop-shell")) { throw "Default format scope unexpectedly included desktop-shell." }

    $outside = Invoke-FixtureFormat -FormatArguments @("-Files", "../outside.rs")
    if ($outside.ExitCode -eq 0) { throw "Directed formatter accepted a path outside the repository." }
    Assert-Contains $outside.Text "outside repository" "Directed formatter must preserve the repository path boundary."
    Write-FixtureFile "desktop-shell/other/src/lib.rs" "pub fn outside_crate() {}`n"
    $unknown = Invoke-FixtureFormat -FormatArguments @("-Files", "desktop-shell/other/src/lib.rs")
    if ($unknown.ExitCode -eq 0) { throw "Directed formatter accepted an unregistered desktop crate." }
    Assert-Contains $unknown.Text "not under a known crate" "Desktop support must remain limited to src-tauri."

    Write-FixtureFile "desktop-shell/src-tauri/Cargo.toml" "[package]`nname = `"format_fixture_desktop`"`nversion = `"0.1.0`"`n"
    $missingEdition = Invoke-FixtureFormat -FormatArguments @("-Files", $desktopFile)
    if ($missingEdition.ExitCode -eq 0) { throw "Directed desktop format accepted a missing explicit edition." }
    Assert-Contains $missingEdition.Text "missing an explicit edition" "Desktop format must preserve the manifest edition gate."
    Write-FixtureFile ".rustfmt-version" "intentionally-incompatible-test-version"
    $wrongVersion = Invoke-FixtureFormat -FormatArguments @("-Files", $desktopFile)
    if ($wrongVersion.ExitCode -eq 0) { throw "Directed desktop format accepted an unlocked formatter version." }
    Assert-Contains $wrongVersion.Text "rustfmt version mismatch" "Desktop format must preserve the formatter version gate."
} finally {
    Set-Location $repoRoot
    if (-not $fixtureFullPath.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path $fixtureFullPath -Leaf) -notmatch '^elon-rustfmt-directed-[a-f0-9]{32}$') {
        throw "Refusing to remove an unexpected format-test fixture path."
    }
    if (Test-Path -LiteralPath $fixtureFullPath) { Remove-Item -LiteralPath $fixtureFullPath -Recurse -Force }
}
Write-Host "PASS directed desktop format checks (edition, version, paths, skip_children, default scope)"

$fullCheck = Invoke-Captured -Command "powershell" -Arguments @(
    "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $psScript
)
if ($fullCheck.ExitCode -ne 0) {
    throw "Repository-wide Rust format baseline is not clean.`n$($fullCheck.Text)"
}

$workflowDoc = Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".github\instructions\git-deploy-workflow.instructions.md") -Encoding UTF8
Assert-Contains $workflowDoc "-Apply -All" "Workflow documentation must describe explicit full formatting."
Assert-Contains $workflowDoc "style(rust)" "Workflow documentation must require a separate mechanical-format commit."

$hook = Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".githooks\pre-push") -Encoding UTF8
Assert-Contains $hook "bash scripts/format-rust.sh" "Pre-push must enforce the full Rust format baseline."
Assert-Contains $hook 'unset $git_local_env_vars' "Pre-push must clear shared Git variables before checking a linked worktree."

$toolchain = Get-Content -Raw -LiteralPath (Join-Path $repoRoot "rust-toolchain.toml") -Encoding UTF8
Assert-Contains $toolchain 'channel = "1.97.0"' "Rust toolchain must match the repository and CI baseline."
Assert-Contains $toolchain 'components = ["rustfmt"]' "Pinned toolchain must install rustfmt."

$versionLock = Get-Content -Raw -LiteralPath (Join-Path $repoRoot ".rustfmt-version") -Encoding UTF8
Assert-Contains $versionLock 'rustfmt 1.9.0-stable (2d8144b788 2026-07-07)' "Rustfmt build must stay locked to the committed baseline."

Write-Host "PASS Rust format workflow guard"
