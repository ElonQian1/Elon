[CmdletBinding()]
param([string]$Remote='origin',[string]$Refspec='HEAD:main',[string]$CacheRoot)
$ErrorActionPreference='Stop'
$repoRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
& (Join-Path $PSScriptRoot 'prepare-push.ps1') -CacheRoot $CacheRoot
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
$temporaryRoot=Join-Path $repoRoot '.ai-tmp';New-Item -ItemType Directory -Force -Path $temporaryRoot|Out-Null
$log=Join-Path $temporaryRoot 'push.log'
& git -C $repoRoot push $Remote $Refspec *> $log
$code=$LASTEXITCODE
if($code -ne 0){Get-Content -LiteralPath $log -Tail 40;exit $code}
Get-Content -LiteralPath $log -Tail 20
exit 0
