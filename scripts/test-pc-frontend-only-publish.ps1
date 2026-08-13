$ErrorActionPreference = 'Stop'
$repo = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$entry = [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/publish-pc-frontend.ps1'))
$helper = [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/publish-server-pc-frontend.ps1'))
$serverPublisher = [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/publish-server.ps1'))
$completion = [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/check-task-complete.ps1'))

function Assert-Contains([string]$Text, [string]$Pattern, [string]$Label) {
    if (-not $Text.Contains($Pattern)) { throw "Missing $Label" }
}

Assert-Contains $entry "git diff --name-only `"`$serverSha..`$Sha`" -- server contracts sdk" 'backend change gate'
Assert-Contains $entry "Assert-GitAncestor `$Sha 'origin/main'" 'pushed commit gate'
Assert-Contains $entry '-ExpectedCurrentReleaseSha $currentReleaseSha' 'remote release CAS'
Assert-Contains $entry 'Publish-PcFrontendRelease' 'shared frontend publisher'
Assert-Contains $entry "Invoke-ElonPublishJsonGet -Uri `"`$ServerUrl/pc/assets/release.json`"" 'published marker verification'
if ($entry -match '(?m)^\s*&\s+.*publish-server\.ps1' -or
    $entry -match '(?m)^\s*(?:cargo|cargo\.exe)\s+' -or
    $entry.Contains('/api/release/claim')) {
    throw 'PC-only publisher must not rebuild or claim a backend release'
}

Assert-Contains $helper '.pc-static-publish.lock' 'remote static publish lock'
Assert-Contains $helper "static release changed: expected=" 'remote static release CAS check'
Assert-Contains $helper 'elon.pc_frontend_release.v1' 'versioned frontend release marker'
Assert-Contains $serverPublisher 'Publish-PcFrontendRelease' 'server bundle shared frontend publisher'
Assert-Contains $helper 'Get-PcFrontendReleaseBaseline' 'server bundle frontend rollback guard'
Assert-Contains $helper "cat '`$RemoteDir/release-sha.txt'" 'authoritative SSH release baseline'
if ($serverPublisher.IndexOf('Invoke-ElonServerPostDeploySmoke') -gt
    $serverPublisher.IndexOf('Publish-PcFrontendRelease')) {
    throw 'server publisher must switch the PC frontend only after backend smoke succeeds'
}
Assert-Contains $completion 'compatibleServerGitSha' 'frontend/server compatibility completion gate'
Assert-Contains $completion 'server contracts sdk' 'completion backend change gate'
Assert-Contains $completion "SERVER_RELEASE_STATUS=`$serverReleaseStatus" 'separate frontend-only completion status'

. (Join-Path $repo 'scripts/publish-server-pc-frontend.ps1')
$temp = Join-Path ([System.IO.Path]::GetTempPath()) "elon-pc-release-$([Guid]::NewGuid().ToString('N'))"
try {
    New-Item -ItemType Directory -Path $temp | Out-Null
    Set-Content -LiteralPath (Join-Path $temp 'index.html') -Value '<div id="root"></div>' -NoNewline
    $frontendSha = '1111111111111111111111111111111111111111'
    $serverSha = '2222222222222222222222222222222222222222'
    Write-PcFrontendReleaseMarker -DistDir $temp -GitSha $frontendSha `
        -CompatibleServerGitSha $serverSha -ReleaseMode frontend_only
    $marker = Get-Content (Join-Path $temp 'release.json') -Raw | ConvertFrom-Json
    if ($marker.schema -ne 'elon.pc_frontend_release.v1' -or
        $marker.gitSha -ne $frontendSha -or
        $marker.compatibleServerGitSha -ne $serverSha -or
        $marker.releaseMode -ne 'frontend_only') {
        throw 'frontend release marker content is invalid'
    }
    if ((Get-Content (Join-Path $temp 'release-sha.txt') -Raw).Trim() -ne $frontendSha) {
        throw 'frontend release SHA marker is invalid'
    }
    if ((Get-Content (Join-Path $temp 'assets/release-sha.txt') -Raw).Trim() -ne $frontendSha) {
        throw 'public frontend release SHA marker is invalid'
    }
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host 'PC_FRONTEND_ONLY_PUBLISH_TEST=passed'
