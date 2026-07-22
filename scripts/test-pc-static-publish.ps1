$ErrorActionPreference = 'Stop'
$repo = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$router = [System.IO.File]::ReadAllText((Join-Path $repo 'server/src/router.rs'))
$powershellPublish = [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/publish-server-pc-frontend.ps1'))
$shellPublish = [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/publish-server.sh'))
$shellPublish += [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/publish-static-dist.sh'))

$checks = [ordered]@{
    hashed_assets_are_immutable = $router.Contains('public, max-age=31536000, immutable') -and
        $router.Contains('.layer(immutable_assets.clone())')
    html_remains_no_cache = $router.Contains('no-cache, no-store, must-revalidate')
    powershell_copies_assets_before_index =
        $powershellPublish.IndexOf("cp -a '`$stagingDist/assets/.'") -lt
        $powershellPublish.IndexOf("mv -f '`$RemoteDir/.publish-new-index-")
    powershell_has_no_delete_swap = -not $powershellPublish.Contains("rm -rf '`$RemoteDir' && mv")
    powershell_retains_old_hashes = $powershellPublish.Contains('.atomic-static-retention') -and
        $powershellPublish.Contains('-mtime +14')
    powershell_pins_current_html_assets = $powershellPublish.Contains("grep -oE 'assets/[A-Za-z0-9._/-]+'")
    shell_copies_assets_before_index =
        $shellPublish.IndexOf('cp -a "$staging_dir/assets"/.') -lt
        $shellPublish.IndexOf('mv -f "$remote_dir/.publish-new-index-$release_sha"')
    shell_has_no_delete_swap = -not $shellPublish.Contains('rm -rf "$remote_dir"')
    shell_retains_old_hashes = $shellPublish.Contains('.atomic-static-retention') -and
        $shellPublish.Contains('-mtime +14')
    shell_pins_current_html_assets = $shellPublish.Contains('touch "$remote_dir/assets/$asset"')
}
$failed = @($checks.Keys | Where-Object { -not $checks[$_] })
if ($failed.Count -gt 0) { throw "PC static publish checks failed: $($failed -join ', ')" }
Write-Output "PC static publish checks passed: $($checks.Count)"
