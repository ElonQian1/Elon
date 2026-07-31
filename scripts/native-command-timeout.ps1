function ConvertTo-ElonWindowsArgument {
    param([AllowEmptyString()] [string]$Value)

    if ($Value -notmatch '[\s"]' -and $Value.Length -gt 0) { return $Value }
    if ($Value.Length -eq 0) { return '""' }

    $builder = New-Object System.Text.StringBuilder
    [void]$builder.Append('"')
    $backslashes = 0
    foreach ($character in $Value.ToCharArray()) {
        if ($character -eq '\') {
            $backslashes++
            continue
        }
        if ($character -eq '"') {
            [void]$builder.Append(('\' * (($backslashes * 2) + 1)))
            [void]$builder.Append('"')
            $backslashes = 0
            continue
        }
        if ($backslashes -gt 0) {
            [void]$builder.Append(('\' * $backslashes))
            $backslashes = 0
        }
        [void]$builder.Append($character)
    }
    if ($backslashes -gt 0) {
        [void]$builder.Append(('\' * ($backslashes * 2)))
    }
    [void]$builder.Append('"')
    return $builder.ToString()
}

function Stop-ElonProcessTree {
    param([Parameter(Mandatory)] [int]$ProcessId)

    $process = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if (-not $process) { return }
    & taskkill.exe /PID $ProcessId /T /F 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-ElonNativeCommand {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)] [string]$FilePath,
        [string[]]$ArgumentList = @(),
        [string]$WorkingDirectory = (Get-Location).Path,
        [ValidateRange(1, 3600)] [int]$TimeoutSeconds = 60,
        [string]$Label = $FilePath
    )

    $resolvedWorkingDirectory = [System.IO.Path]::GetFullPath($WorkingDirectory)
    if (-not (Test-Path -LiteralPath $resolvedWorkingDirectory -PathType Container)) {
        throw "Native command working directory does not exist: $resolvedWorkingDirectory"
    }

    $stdoutPath = [System.IO.Path]::GetTempFileName()
    $stderrPath = [System.IO.Path]::GetTempFileName()
    $argumentText = (($ArgumentList | ForEach-Object {
        ConvertTo-ElonWindowsArgument -Value ([string]$_)
    }) -join ' ')
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $process = $null
    try {
        $process = Start-Process -FilePath $FilePath `
            -ArgumentList $argumentText `
            -WorkingDirectory $resolvedWorkingDirectory `
            -PassThru -WindowStyle Hidden `
            -RedirectStandardOutput $stdoutPath `
            -RedirectStandardError $stderrPath
        # Windows PowerShell 5 may report ExitCode=0 for a fast process unless
        # its native handle is materialized before the process exits.
        [void]$process.Handle

        while (-not $process.HasExited -and $watch.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
            Start-Sleep -Milliseconds 200
            $process.Refresh()
        }

        $timedOut = -not $process.HasExited
        if ($timedOut) {
            Stop-ElonProcessTree -ProcessId $process.Id
            $process.WaitForExit(5000) | Out-Null
        } else {
            $process.WaitForExit()
        }
        $process.Refresh()

        $stdout = if (Test-Path -LiteralPath $stdoutPath) {
            Get-Content -LiteralPath $stdoutPath -Raw -ErrorAction SilentlyContinue
        } else { '' }
        $stderr = if (Test-Path -LiteralPath $stderrPath) {
            Get-Content -LiteralPath $stderrPath -Raw -ErrorAction SilentlyContinue
        } else { '' }
        $exitCode = if ($timedOut) { 124 } else {
            try { [int]$process.ExitCode } catch { 1 }
        }

        [PSCustomObject]@{
            Label = $Label
            ExitCode = $exitCode
            TimedOut = $timedOut
            DurationSeconds = [Math]::Round($watch.Elapsed.TotalSeconds, 1)
            Stdout = [string]$stdout
            Stderr = [string]$stderr
        }
    } finally {
        $watch.Stop()
        Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force -ErrorAction SilentlyContinue
    }
}

function Assert-ElonNativeCommand {
    param(
        [Parameter(Mandatory)] $Result,
        [string]$FailureMessage = 'Native command failed.'
    )

    if ($Result.ExitCode -eq 0) { return }
    $detail = @($Result.Stderr, $Result.Stdout) |
        Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } |
        Select-Object -First 1
    $suffix = if ($detail) { ": $(([string]$detail).Trim())" } else { '' }
    if ($Result.TimedOut) {
        throw "$FailureMessage timed out after $($Result.DurationSeconds)s$suffix"
    }
    throw "$FailureMessage exit=$($Result.ExitCode)$suffix"
}
