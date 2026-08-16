function Get-RustCacheCommandHelp {
    $commands = @(
        [pscustomobject]@{ name = "help"; purpose = "Show the command contract and common examples." }
        [pscustomobject]@{ name = "doctor"; purpose = "Read-only health check for one PC and project." }
        [pscustomobject]@{ name = "fleet-report"; purpose = "Write a sanitized machine report for fleet aggregation." }
        [pscustomobject]@{ name = "fleet-stage"; purpose = "Queue an immutable hashed report envelope for node upload." }
        [pscustomobject]@{ name = "status"; purpose = "Inspect local partitions, disk state, and legacy registrations." }
        [pscustomobject]@{ name = "run"; purpose = "Run Cargo through the managed cache and partition locks." }
        [pscustomobject]@{ name = "gc"; purpose = "Preview or apply managed partition reclamation." }
        [pscustomobject]@{ name = "gc-plan"; purpose = "Create an immutable local GC plan and sanitized approval summary." }
        [pscustomobject]@{ name = "gc-apply-approved"; purpose = "Re-scan and apply only an exact digest-bound approved GC plan." }
        [pscustomobject]@{ name = "install"; purpose = "Install or upgrade the per-PC platform and launcher." }
        [pscustomobject]@{ name = "adopt-project"; purpose = "Preview or write a portable child-project manifest and thin launcher." }
        [pscustomobject]@{ name = "init-project"; purpose = "Preview or write the portable project manifest." }
        [pscustomobject]@{ name = "register-legacy"; purpose = "Register an external cache without deleting it." }
        [pscustomobject]@{ name = "purge-legacy"; purpose = "Preview or apply deletion of one retired registered cache." }
    )
    [pscustomobject]@{
        schema = "elon.rust_cache.command_help.v1"
        usage = "rust-cache.ps1 <command> [options] [-- cargo arguments]"
        commands = $commands
        examples = @(
            "rust-cache.ps1 doctor -ProjectRoot D:\work\project"
            "rust-cache.ps1 fleet-report -ProjectRoot D:\work\project -NodeId <platform-node-id> -IncludeSizes"
            "rust-cache.ps1 fleet-stage -ProjectRoot D:\work\project -NodeId <platform-node-id> -IncludeSizes"
            "rust-cache.ps1 adopt-project -ProjectRoot D:\work\project -ProjectId stable-project-id"
            "rust-cache.ps1 init-project -ProjectRoot D:\work\project -ProjectId stable-project-id"
            "rust-cache.ps1 gc -ProjectRoot D:\work\project -IncludeSizes"
            "rust-cache.ps1 gc-plan -RequestId <32-hex-id> -NodeId <platform-node-id>"
            "rust-cache.ps1 gc-apply-approved -RequestId <id> -PlanId <id> -PlanDigest <sha256> -NodeId <node-id>"
            "rust-cache.ps1 run -ProjectRoot D:\work\project -Domain dev-windows-msvc -- check --locked"
        )
        safety = @(
            "gc and purge-legacy are dry-run unless -Apply is present."
            "gc-apply-approved executes only an unexpired immutable plan after an exact local rescan."
            "Never recursively delete a cache root outside the managed commands."
            "Invoke the launcher in the current PowerShell session; do not open a nested visible shell."
        )
    }
}

function Show-RustCacheCommandHelp {
    $help = Get-RustCacheCommandHelp
    Write-Host $help.usage -ForegroundColor Cyan
    $help.commands | Format-Table name, purpose -AutoSize
    Write-Host "Examples:" -ForegroundColor Cyan
    $help.examples | ForEach-Object { Write-Host "  $_" }
    return $help
}

Export-ModuleMember -Function Get-RustCacheCommandHelp, Show-RustCacheCommandHelp
