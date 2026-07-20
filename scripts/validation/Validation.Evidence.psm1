function Write-ValidationJsonAtomic {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)]$Value)
    New-Item -ItemType Directory -Force -Path (Split-Path $Path -Parent) | Out-Null
    $temporary = "$Path.$PID.tmp"
    $Value | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $temporary -Encoding UTF8
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Get-ValidationFailureItems {
    param([string[]]$Lines)
    return @($Lines | Where-Object { $_ -match '(^|\s)(FAILED|failures:|error(?:\[[^]]+\])?:)' } | Select-Object -First 100)
}

function Stop-ValidationCapturedProcess {
    param([Parameter(Mandatory)]$Process, [string]$TaskkillFilePath = 'taskkill.exe')
    $treeStopped = $false
    try {
        & $TaskkillFilePath /PID $Process.Id /T /F 2>$null | Out-Null
        $treeStopped = ($LASTEXITCODE -eq 0)
    } catch { $treeStopped = $false }
    if (-not $treeStopped) {
        $rootId = [int]$Process.Id
        $all = @(Get-CimInstance Win32_Process -ErrorAction Stop)
        $byParent = @{}
        foreach ($item in $all) { $byParent[[int]$item.ParentProcessId] = @($byParent[[int]$item.ParentProcessId]) + @($item) }
        $ordered = New-Object System.Collections.Generic.List[object]
        $visited = New-Object 'System.Collections.Generic.HashSet[int]'
        [void]$visited.Add($rootId)
        function Add-ValidationDescendants([int]$ParentId) {
            foreach ($child in @($byParent[$ParentId])) {
                $childId = [int]$child.ProcessId
                if (-not $visited.Add($childId)) { continue }
                Add-ValidationDescendants -ParentId $childId
                $ordered.Add($child)
            }
        }
        Add-ValidationDescendants -ParentId $rootId
        foreach ($child in $ordered) {
            Stop-Process -Id ([int]$child.ProcessId) -Force -ErrorAction SilentlyContinue
        }
        if (-not $Process.HasExited) { $Process.Kill() }
    }
    return $treeStopped
}

function Invoke-ValidationCapturedProcess {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(Mandatory)][string[]]$ArgumentList,
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$EvidenceDirectory,
        [int]$TimeoutSeconds = 3600
    )
    if ($env:ELON_TEST_VALIDATION_CAPTURE_EXCEPTION -eq '1') { throw 'Injected validation capture exception.' }
    New-Item -ItemType Directory -Force -Path $EvidenceDirectory | Out-Null
    $stdoutPath = Join-Path $EvidenceDirectory "stdout.log"
    $stderrPath = Join-Path $EvidenceDirectory "stderr.log"
    $quoted = foreach ($arg in $ArgumentList) {
        if ($arg -match '[\s"]') { '"' + $arg.Replace('"','\"') + '"' } else { $arg }
    }
    $started = [DateTime]::UtcNow
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $FilePath
    $info.Arguments = $quoted -join ' '
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.WorkingDirectory = $WorkingDirectory
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $info
    if (-not $process.Start()) { throw "Unable to start validation process: $FilePath" }
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $timedOut = -not $process.WaitForExit([Math]::Max(1,$TimeoutSeconds)*1000)
    if ($timedOut) {
        Stop-ValidationCapturedProcess -Process $process | Out-Null
        if (-not $process.WaitForExit(5000)) { throw "Validation process did not terminate after timeout: $FilePath" }
    }
    $stdoutText = $stdoutTask.GetAwaiter().GetResult(); $stderrText = $stderrTask.GetAwaiter().GetResult()
    [IO.File]::WriteAllText($stdoutPath, $stdoutText, (New-Object Text.UTF8Encoding($false)))
    [IO.File]::WriteAllText($stderrPath, $stderrText, (New-Object Text.UTF8Encoding($false)))
    $finished = [DateTime]::UtcNow
    $stdout = @($stdoutText -split "`r?`n")
    $stderr = @($stderrText -split "`r?`n")
    return [pscustomobject]@{
        exit_code = if ($timedOut) { 124 } else { [int]$process.ExitCode }
        timed_out = $timedOut
        started_utc = $started.ToString("o")
        finished_utc = $finished.ToString("o")
        duration_ms = [int]($finished-$started).TotalMilliseconds
        stdout_path = $stdoutPath
        stderr_path = $stderrPath
        stdout_lines = $stdout.Count
        stderr_lines = $stderr.Count
        failures = @(Get-ValidationFailureItems -Lines (@($stdout)+@($stderr)))
        tail = @(@($stdout)+@($stderr) | Select-Object -Last 40)
    }
}

function Remove-ExpiredValidationEvidence {
    param([Parameter(Mandatory)][string]$EvidenceRoot, [int]$RetentionDays=14, [int]$MaximumResults=100)
    if (-not (Test-Path -LiteralPath $EvidenceRoot)) { return }
    $cutoff = [DateTime]::UtcNow.AddDays(-$RetentionDays)
    $items = @(Get-ChildItem -LiteralPath $EvidenceRoot -Directory -ErrorAction SilentlyContinue | Sort-Object LastWriteTimeUtc -Descending)
    for ($i=0; $i -lt $items.Count; $i++) {
        $lock = Join-Path $items[$i].FullName ".run.lock"
        if (Test-Path -LiteralPath $lock) { continue }
        if ($i -ge $MaximumResults -or $items[$i].LastWriteTimeUtc -lt $cutoff) {
            Remove-Item -LiteralPath $items[$i].FullName -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Export-ModuleMember -Function Write-ValidationJsonAtomic, Get-ValidationFailureItems, Stop-ValidationCapturedProcess, Invoke-ValidationCapturedProcess, Remove-ExpiredValidationEvidence
