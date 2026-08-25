function ConvertTo-NodeAgentExtendedPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    if ([System.Environment]::OSVersion.Platform -ne [System.PlatformID]::Win32NT -or
        $fullPath.StartsWith('\\?\', [System.StringComparison]::Ordinal)) {
        return $fullPath
    }
    if ($fullPath.StartsWith('\\', [System.StringComparison]::Ordinal)) {
        return '\\?\UNC\' + $fullPath.TrimStart('\')
    }
    return '\\?\' + $fullPath
}

function Get-NodeAgentTrackedRustSources {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string[]]$Pathspecs
    )

    $root = [System.IO.Path]::GetFullPath($RepoRoot).TrimEnd('\', '/')
    $rootPrefix = $root + [System.IO.Path]::DirectorySeparatorChar
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $tracked = @()
    try {
        $tracked = @(& git -c core.longpaths=true -c core.quotepath=false `
            -C $root ls-files -- @Pathspecs 2>&1)
        $gitExit = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($gitExit -ne 0) {
        $detail = (@($tracked | Select-Object -Last 8) -join ' | ').Trim()
        throw "无法读取节点发布源码清单（git exit=$gitExit）：$detail"
    }

    foreach ($relativeValue in $tracked) {
        $relative = ([string]$relativeValue).Trim()
        if ([string]::IsNullOrWhiteSpace($relative)) { continue }
        $fullPath = [System.IO.Path]::GetFullPath((Join-Path $root $relative))
        if (-not $fullPath.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "节点发布源码路径越出仓库：$relative"
        }
        [pscustomobject]@{
            RelativePath = $relative.Replace('/', '\')
            FullPath = $fullPath
            ReadPath = ConvertTo-NodeAgentExtendedPath -Path $fullPath
        }
    }
}
