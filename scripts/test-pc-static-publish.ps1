$ErrorActionPreference = 'Stop'
$repo = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$router = [System.IO.File]::ReadAllText((Join-Path $repo 'server/src/router.rs'))
$powershellPublish = [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/publish-server-pc-frontend.ps1'))
$shellPublish = [System.IO.File]::ReadAllText((Join-Path $repo 'scripts/publish-server.sh'))
$shellServerPublish = $shellPublish
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
    powershell_frontend_only_cas = $powershellPublish.Contains('.pc-static-publish.lock') -and
        $powershellPublish.Contains('static release changed: expected=')
    shell_copies_assets_before_index =
        $shellPublish.IndexOf('cp -a "$staging_dir/assets"/.') -lt
        $shellPublish.IndexOf('mv -f "$remote_dir/.publish-new-index-$release_sha"')
    shell_has_no_delete_swap = -not $shellPublish.Contains('rm -rf "$remote_dir"')
    shell_retains_old_hashes = $shellPublish.Contains('.atomic-static-retention') -and
        $shellPublish.Contains('-mtime +14')
    shell_pins_current_html_assets = $shellPublish.Contains('touch "$remote_dir/assets/$asset"')
    shell_frontend_only_cas = $shellPublish.Contains('.pc-static-publish.lock') -and
        $shellPublish.Contains('static release changed: expected=') -and
        $shellPublish.Contains('current_pc_release_sha')
    shell_public_release_marker = $shellPublish.Contains('write_static_release_marker') -and
        $shellPublish.Contains('read_static_release_baseline')
    shell_writes_frontend_release_marker = $shellPublish.Contains('elon.pc_frontend_release.v1') -and
        $shellPublish.Contains('compatibleServerGitSha') -and
        $shellPublish.Contains('release-sha.txt')
    shell_switches_frontend_after_server_verification =
        $shellServerPublish.IndexOf('DEPLOYED_VERSION_NAME') -lt
        $shellServerPublish.IndexOf('upload_static_dist "$PC_DIST_DIR"')
}
$failed = @($checks.Keys | Where-Object { -not $checks[$_] })
if ($failed.Count -gt 0) { throw "PC static publish checks failed: $($failed -join ', ')" }
Write-Output "PC static publish checks passed: $($checks.Count)"
