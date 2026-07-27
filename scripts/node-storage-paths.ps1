$ErrorActionPreference = 'Stop'

function Test-ElonStorageAbsolutePath {
    param([AllowNull()][string]$PathValue)

    if ([string]::IsNullOrWhiteSpace($PathValue)) { return $false }
    if (-not [System.IO.Path]::IsPathRooted($PathValue)) { return $false }
    if ($PathValue -match '^[A-Za-z]:($|[^\\/])') { return $false }
    if ($PathValue -match '^[\\/][^\\/]') { return $false }
    return $true
}

function Read-ElonStorageJson {
    param([Parameter(Mandatory = $true)][string]$Path)

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $offset = if (
        $bytes.Length -ge 3 -and
        $bytes[0] -eq 0xEF -and
        $bytes[1] -eq 0xBB -and
        $bytes[2] -eq 0xBF
    ) { 3 } else { 0 }
    $strictUtf8 = New-Object System.Text.UTF8Encoding($false, $true)
    $json = $strictUtf8.GetString($bytes, $offset, $bytes.Length - $offset)
    return ($json | ConvertFrom-Json -ErrorAction Stop)
}

function Test-ElonOwnedNodeDataRoot {
    param(
        [Parameter(Mandatory = $true)][string]$RootPath,
        [AllowNull()][string]$ExpectedInstallId
    )

    if (-not (Test-ElonStorageAbsolutePath $RootPath)) { return $false }
    try {
        $fullRoot = [System.IO.Path]::GetFullPath($RootPath.Trim())
        $volumeRoot = [System.IO.Path]::GetPathRoot($fullRoot).TrimEnd('\', '/')
        if ($fullRoot.TrimEnd('\', '/') -eq $volumeRoot) { return $false }
        if (-not (Test-Path -LiteralPath $fullRoot -PathType Container)) { return $false }
        $rootItem = Get-Item -LiteralPath $fullRoot -Force
        if (($rootItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
            return $false
        }

        $markerPath = Join-Path $fullRoot '.elon-node-data-root.json'
        if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) { return $false }
        $marker = Read-ElonStorageJson -Path $markerPath
        $markerInstallId = [string]$marker.install_id
        if (-not [string]::IsNullOrWhiteSpace($ExpectedInstallId) -and
            -not [string]::Equals(
                $markerInstallId,
                $ExpectedInstallId,
                [System.StringComparison]::Ordinal
            )) {
            return $false
        }
        return $true
    } catch {
        return $false
    }
}

function Get-ElonPersistedNodeDataRoot {
    if ([string]::IsNullOrWhiteSpace($env:APPDATA)) { return $null }

    $configPath = Join-Path $env:APPDATA 'elon-node-agent\node.json'
    if (-not (Test-Path -LiteralPath $configPath -PathType Leaf)) { return $null }
    try {
        $config = Read-ElonStorageJson -Path $configPath
        $root = [string]$config.node_data_root
        $installId = [string]$config.install_id
        if ([string]::IsNullOrWhiteSpace($installId)) { return $null }
        if (-not (Test-ElonOwnedNodeDataRoot -RootPath $root -ExpectedInstallId $installId)) {
            return $null
        }
        return [System.IO.Path]::GetFullPath($root.Trim())
    } catch {
        return $null
    }
}

function Get-ElonNodeDataRoot {
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_NODE_DATA_ROOT) -and
        (Test-ElonOwnedNodeDataRoot -RootPath $env:ELON_NODE_DATA_ROOT -ExpectedInstallId $null)) {
        return [System.IO.Path]::GetFullPath($env:ELON_NODE_DATA_ROOT.Trim())
    }
    return Get-ElonPersistedNodeDataRoot
}

function Get-ElonManagedReleaseTargetRoot {
    $nodeDataRoot = Get-ElonNodeDataRoot
    if ([string]::IsNullOrWhiteSpace($nodeDataRoot)) { return $null }
    return (Join-Path $nodeDataRoot 'cache\release-targets')
}

function Resolve-ElonNodeAgentTargetDir {
    $candidates = @()
    if (-not [string]::IsNullOrWhiteSpace($env:ELON_NODE_AGENT_TARGET_DIR)) {
        $candidates += $env:ELON_NODE_AGENT_TARGET_DIR
    }
    $managedReleaseRoot = Get-ElonManagedReleaseTargetRoot
    if ($managedReleaseRoot) {
        $candidates += (Join-Path $managedReleaseRoot 'elon-node-agent')
    }
    if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        $candidates += (Join-Path $env:LOCALAPPDATA 'Elon\build-target\elon-node-agent')
    }
    if (-not [string]::IsNullOrWhiteSpace($env:PUBLIC)) {
        $candidates += (Join-Path $env:PUBLIC 'Elon\build-target\elon-node-agent')
    }

    foreach ($candidate in $candidates) {
        if ([string]::IsNullOrWhiteSpace($candidate)) { continue }
        $fullPath = [System.IO.Path]::GetFullPath($candidate)
        if ($fullPath -match '\s') { continue }
        New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
        return $fullPath
    }
    throw '无法解析无空格的 PC 节点 target 目录；请设置 ELON_NODE_AGENT_TARGET_DIR 为无空格路径。'
}

function Resolve-ElonBuildTargetRoot {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)

    if (-not [string]::IsNullOrWhiteSpace($env:ELON_BUILD_TARGET_DIR)) {
        $root = $env:ELON_BUILD_TARGET_DIR.Trim()
        $source = 'ELON_BUILD_TARGET_DIR'
    } elseif ($managedReleaseRoot = Get-ElonManagedReleaseTargetRoot) {
        $root = $managedReleaseRoot
        $source = '节点数据根 release-targets'
    } elseif (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        $root = Join-Path $env:LOCALAPPDATA 'Elon\build-target'
        $source = 'LOCALAPPDATA'
    } else {
        $root = Join-Path (Split-Path $RepoRoot -Parent) '.elon-build-target'
        $source = '仓库同级 fallback'
    }

    if (-not (Test-ElonStorageAbsolutePath $root)) {
        throw "$source 必须解析为绝对构建缓存路径，当前值：$root"
    }
    $fullPath = [System.IO.Path]::GetFullPath($root)
    $pathRoot = [System.IO.Path]::GetPathRoot($fullPath)
    if ($pathRoot -and -not (Test-Path -LiteralPath $pathRoot)) {
        throw "构建缓存目录所在盘符不存在：$fullPath"
    }
    New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
    return $fullPath
}

function Resolve-ElonServerMuslTargetDir {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)

    $targetVarName = $null
    $targetDir = $null
    if (-not [string]::IsNullOrWhiteSpace($env:RUST_SERVER_MUSL_TARGET_DIR)) {
        $targetVarName = 'RUST_SERVER_MUSL_TARGET_DIR'
        $targetDir = $env:RUST_SERVER_MUSL_TARGET_DIR.Trim()
    } elseif (-not [string]::IsNullOrWhiteSpace($env:RUST_MUSL_TARGET_DIR)) {
        $targetVarName = 'RUST_MUSL_TARGET_DIR'
        $targetDir = $env:RUST_MUSL_TARGET_DIR.Trim()
    }

    if ($targetDir) {
        if (-not (Test-ElonStorageAbsolutePath $targetDir)) {
            throw "$targetVarName 必须是绝对路径，当前值：$targetDir"
        }
        $fullPath = [System.IO.Path]::GetFullPath($targetDir)
        $pathRoot = [System.IO.Path]::GetPathRoot($fullPath)
        if ($pathRoot -and -not (Test-Path -LiteralPath $pathRoot)) {
            throw "server musl 构建缓存目录所在盘符不存在：$fullPath"
        }
        New-Item -ItemType Directory -Force -Path $fullPath | Out-Null
        return $fullPath
    }

    return (Join-Path (Resolve-ElonBuildTargetRoot -RepoRoot $RepoRoot) 'elon-server-musl')
}
