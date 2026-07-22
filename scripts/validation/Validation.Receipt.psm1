Import-Module "$PSScriptRoot\Validation.Fingerprint.psm1" -Force -DisableNameChecking
Import-Module "$PSScriptRoot\Validation.Evidence.psm1" -Force -DisableNameChecking

function Get-ValidationReceiptProfileId {
    param([Parameter(Mandatory)]$FingerprintDetails)
    $payload=$FingerprintDetails.payload
    $stable=[ordered]@{schema='elon.validation.receipt_profile.v1';project=$payload.project;command=$payload.command;domain=$payload.domain;target_dir=$payload.target_dir;execution_options=$payload.execution_options}
    return Get-ValidationSha256 -Text ($stable|ConvertTo-Json -Depth 8 -Compress)
}

function Get-ValidationReceiptPath {
    param([Parameter(Mandatory)][string]$StateRoot,[Parameter(Mandatory)]$FingerprintDetails)
    Join-Path $StateRoot ("receipts\"+(Get-ValidationReceiptProfileId $FingerprintDetails)+'.json')
}

function Write-ValidationReceipt {
    param(
        [Parameter(Mandatory)][string]$StateRoot,
        [Parameter(Mandatory)]$FingerprintDetails,
        [Parameter(Mandatory)][string]$EvidenceSummaryPath,
        [string]$NetworkReportPath
    )
    $summary=Get-Content -Raw -LiteralPath $EvidenceSummaryPath -Encoding UTF8|ConvertFrom-Json
    if($summary.status -ne 'success' -or [string]$summary.fingerprint -ne [string]$FingerprintDetails.fingerprint){throw 'Cannot issue a push receipt from non-success or mismatched evidence.'}
    $path=Get-ValidationReceiptPath $StateRoot $FingerprintDetails
    $receipt=[ordered]@{schema='elon.validation.receipt.v1';profile_id=Get-ValidationReceiptProfileId $FingerprintDetails;fingerprint=$FingerprintDetails.fingerprint;status='success';created_utc=[DateTime]::UtcNow.ToString('o');evidence_summary=[IO.Path]::GetFullPath($EvidenceSummaryPath);network_report=if($NetworkReportPath){[IO.Path]::GetFullPath($NetworkReportPath)}else{$null};fingerprint_inputs=$FingerprintDetails.payload}
    Write-ValidationJsonAtomic -Path $path -Value $receipt
    return $path
}

function Test-ValidationReceipt {
    param([Parameter(Mandatory)][string]$StateRoot,[Parameter(Mandatory)]$FingerprintDetails)
    $path=Get-ValidationReceiptPath $StateRoot $FingerprintDetails
    if(-not(Test-Path -LiteralPath $path)){return [pscustomobject]@{valid=$false;code='missing';path=$path;receipt=$null}}
    try{$receipt=Get-Content -Raw -LiteralPath $path -Encoding UTF8|ConvertFrom-Json}catch{return [pscustomobject]@{valid=$false;code='invalid_json';path=$path;receipt=$null}}
    if($receipt.schema -ne 'elon.validation.receipt.v1' -or $receipt.status -ne 'success'){return [pscustomobject]@{valid=$false;code='invalid_schema_or_status';path=$path;receipt=$receipt}}
    if([string]$receipt.fingerprint -ne [string]$FingerprintDetails.fingerprint){return [pscustomobject]@{valid=$false;code='fingerprint_changed';path=$path;receipt=$receipt}}
    if(-not(Test-Path -LiteralPath ([string]$receipt.evidence_summary))){return [pscustomobject]@{valid=$false;code='evidence_missing';path=$path;receipt=$receipt}}
    try{$summary=Get-Content -Raw -LiteralPath ([string]$receipt.evidence_summary) -Encoding UTF8|ConvertFrom-Json}catch{return [pscustomobject]@{valid=$false;code='evidence_invalid';path=$path;receipt=$receipt}}
    if($summary.status -ne 'success' -or [string]$summary.fingerprint -ne [string]$receipt.fingerprint){return [pscustomobject]@{valid=$false;code='evidence_mismatch';path=$path;receipt=$receipt}}
    return [pscustomobject]@{valid=$true;code='valid';path=$path;receipt=$receipt}
}

Export-ModuleMember -Function Get-ValidationReceiptProfileId, Get-ValidationReceiptPath, Write-ValidationReceipt, Test-ValidationReceipt
