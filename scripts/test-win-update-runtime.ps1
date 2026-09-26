$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = Split-Path -Parent $PSScriptRoot
$guard = Join-Path $repo 'desktop-shell/src-tauri/src/codex_semantic_bridge/update_restart_guard.ps1'
$tokens = $null; $errors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($guard, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw ($errors | Out-String) }
# Load only the pure lookups; never run the guard or an installer in a unit test.
foreach ($name in @('Get-TargetLocalReleaseState', 'Test-InstalledTarget')) {
    $definition = $ast.Find({ param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $name
    }, $true)
    . ([scriptblock]::Create($definition.Extent.Text))
}
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('win-update-test-' + [guid]::NewGuid().ToString('N'))
$localReleaseRoot = Join-Path $temporary 'managed/release-state/local-node-releases-v1'
$internalDir = Join-Path $temporary 'install/_internal'
$sha = 'a' * 40
$ExpectedReleaseIdentity = "0.3.69+$sha"
try {
    $releaseDir = Join-Path $localReleaseRoot "releases/$sha"
    New-Item -ItemType Directory -Path $releaseDir, $internalDir -Force | Out-Null
    if ((Get-TargetLocalReleaseState) -ne '') { throw 'Missing package must remain unknown.' }
    $stateFile = Join-Path $releaseDir 'state.json'
    @{ release_identity = $ExpectedReleaseIdentity; activation_state = 'waiting_for_terminal' } |
        ConvertTo-Json | Set-Content -LiteralPath $stateFile -Encoding UTF8
    if ((Get-TargetLocalReleaseState) -ne 'waiting_for_terminal') { throw 'Managed staged package was not found.' }
    @{ release_identity = "0.3.69+$('b' * 40)"; activation_state = 'activated' } |
        ConvertTo-Json | Set-Content -LiteralPath $stateFile -Encoding UTF8
    if ((Get-TargetLocalReleaseState) -ne '') { throw 'Wrong package must not match.' }
    if (Test-InstalledTarget) { throw 'Missing installed identity is not success.' }
    @{ version = '0.3.69'; gitSha = $sha } | ConvertTo-Json |
        Set-Content -LiteralPath (Join-Path $internalDir 'node-agent-version.json') -Encoding UTF8
    if (-not (Test-InstalledTarget)) { throw 'Installed target lookup failed.' }
    $ExpectedReleaseIdentity = "0.3.69+$('c' * 40)"
    if (Test-InstalledTarget) { throw 'Old installed metadata is not success.' }
    Write-Output 'WIN_UPDATE_RUNTIME_TESTS=passed'
} finally {
    $resolved = [IO.Path]::GetFullPath($temporary)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe test cleanup.' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
