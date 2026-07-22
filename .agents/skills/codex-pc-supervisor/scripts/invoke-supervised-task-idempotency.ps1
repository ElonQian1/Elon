function New-OrdinaryPostRequestContext {
    param([object]$Body)
    [pscustomobject]@{
        IdempotencyKey = "desktop-$([guid]::NewGuid().ToString('N'))"
        BodyBytes = [byte[]](Convert-ToUtf8JsonBytes $Body)
    }
}

function Test-OrdinaryPostTimeout {
    param([System.Exception]$Exception)
    $current = $Exception
    while ($null -ne $current) {
        if ($current -is [System.Net.WebException] -and
            $current.Status -eq [System.Net.WebExceptionStatus]::Timeout) {
            return $true
        }
        $current = $current.InnerException
    }
    return $false
}

function Invoke-IdempotentNodePost {
    param(
        [object]$Connection,
        [string]$Path,
        [object]$Body,
        [object]$RequestContext = $null,
        [scriptblock]$RequestInvoker = $null,
        [scriptblock]$ConnectionResolver = $null
    )
    $context = if ($null -ne $RequestContext) { $RequestContext } else {
        New-OrdinaryPostRequestContext $Body
    }
    if ($null -eq $RequestInvoker) {
        $RequestInvoker = {
            param($Candidate, $EndpointPath, [byte[]]$Bytes, [string]$Key)
            Invoke-NodeApi $Candidate 'Post' $EndpointPath $null -BodyBytes $Bytes `
                -ExtraHeaders @{ 'x-elon-idempotency-key' = $Key }
        }
    }
    if ($null -eq $ConnectionResolver) {
        $ConnectionResolver = { Get-NodeConnection -RetrySeconds 4 }
    }
    $activeConnection = $Connection
    for ($attempt = 0; $attempt -lt 2; $attempt++) {
        try {
            $response = & $RequestInvoker $activeConnection $Path `
                ([byte[]]$context.BodyBytes) ([string]$context.IdempotencyKey)
            return [pscustomobject]@{
                Response = $response
                Connection = $activeConnection
                RequestContext = $context
            }
        } catch {
            if ($attempt -ne 0 -or -not (Test-OrdinaryPostTimeout $_.Exception)) { throw }
            $activeConnection = & $ConnectionResolver
        }
    }
    throw 'Idempotent POST retry exhausted unexpectedly.'
}
