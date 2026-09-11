#requires -Version 7.0
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-native-draft.ps1')

$count = 0
function Check($State, [bool]$Expected) {
    $actual = Test-ChatGptWebNativeDraftEmpty $State
    if ($actual -isnot [bool] -or $actual -ne $Expected) { throw 'native_draft_admission_mismatch' }
    $script:count++
}

Check @{ input = @{ has_text = $false; text_length = 0 } } $true
Check ('{"input":{"has_text":false,"text_length":0,"send_enabled":false}}' | ConvertFrom-Json) $true
Check @{ input = @{ has_text = $false; text_length = [long]0 } } $true
foreach ($state in @(
    $null, @{}, @{ input = $null }, @{ input = @{} },
    @{ input = @{ text = '' } },
    @{ input = @{ has_text = $false } },
    @{ input = @{ text_length = 0 } },
    @{ input = @{ has_text = $true; text_length = 8 } },
    @{ input = @{ has_text = $true; text_length = 0 } },
    @{ input = @{ has_text = $false; text_length = 8 } },
    @{ input = @{ has_text = $false; text_length = -1 } },
    @{ input = @{ has_text = 'false'; text_length = 0 } },
    @{ input = @{ has_text = 0; text_length = 0 } },
    @{ input = @{ has_text = $null; text_length = 0 } },
    @{ input = @{ has_text = $false; text_length = '0' } },
    @{ input = @{ has_text = $false; text_length = 0.0 } },
    @{ input = @{ has_text = $false; text_length = $null } },
    @{ input = @{ has_text = $false; text_length = $false } }
)) { Check $state $false }

# Exercise the actual preflight and final-restoration expressions, without a phone.
foreach ($file in @('smoke-chatgpt-web-file-reference-inventory.ps1', 'smoke-chatgpt-web-extended-tools.ps1')) {
    $path = Join-Path $PSScriptRoot $file
    $tokens = $null; $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
    if ($errors.Count) { throw 'native_draft_consumer_parse_failed' }
    $source = $ast.Extent.Text
    if (-not $source.Contains("'chatgpt-web-smoke-native-draft.ps1'") -or $source -match '\.input\.text\b') {
        throw 'legacy_native_draft_consumer'
    }
    $checks = @($ast.FindAll({ param($node)
        $node -is [System.Management.Automation.Language.CommandAst] -and
            $node.GetCommandName() -eq 'Test-ChatGptWebNativeDraftEmpty'
    }, $true))
    if ($checks.Count -lt 3) { throw 'missing_native_draft_safety_checks' }
    foreach ($probe in @(
        @{ state = @{ input = @{ has_text = $false; text_length = 0 } }; empty = $true },
        @{ state = @{ input = @{ has_text = $true; text_length = 8 } }; empty = $false },
        @{ state = @{ input = @{ text = '' } }; empty = $false }
    )) {
        $empty = $probe.empty
        $origin = $probe.state
        $after = $origin
        $current = $origin
        foreach ($check in $checks) {
            if ((. ([scriptblock]::Create($check.Extent.Text))) -ne $empty) { throw 'native_draft_consumer_mismatch' }
            $count++
        }
    }
}
Write-Output "CHATGPT_NATIVE_DRAFT_TESTS=passed cases=$count"
