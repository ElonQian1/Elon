$ErrorActionPreference='Stop'
$modules=Join-Path $PSScriptRoot 'validation'
Import-Module (Join-Path $modules 'Validation.Arguments.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $modules 'Cargo.Network.psm1') -Force -DisableNameChecking
Import-Module (Join-Path $PSScriptRoot 'rust-cache\RustCache.Paths.psm1') -Force -DisableNameChecking
$parsed=Split-ValidationCargoArguments -Arguments $args -ValueOptions @{'-CacheRoot'='CacheRoot';'-Domain'='Domain';'-TargetDir'='TargetDir';'-ReportRoot'='ReportRoot';'-CompileTimeoutSeconds'='CompileTimeoutSeconds'} -SwitchOptions @('-DisableSccache','-SkipOfflineFirst')
$repoRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$cacheRoot=Resolve-RustCacheRoot -ExplicitRoot $parsed.wrapper.CacheRoot -RepoRoot $repoRoot
$reportRoot=if($parsed.wrapper.ReportRoot){[IO.Path]::GetFullPath($parsed.wrapper.ReportRoot)}else{Join-Path $cacheRoot ('validation-v1\network\'+[Guid]::NewGuid().ToString('N'))}
$result=Invoke-CargoNetworkValidation -RepoRoot $repoRoot -CargoDevPath (Join-Path $PSScriptRoot 'cargo-dev.ps1') -ReportRoot $reportRoot -CargoArguments @($parsed.cargo) -ResolvedCacheRoot $cacheRoot -Domain $(if($parsed.wrapper.Domain){$parsed.wrapper.Domain}else{'agent-validation'}) -TargetDir $parsed.wrapper.TargetDir -CompileTimeoutSeconds $(if($parsed.wrapper.CompileTimeoutSeconds){[int]$parsed.wrapper.CompileTimeoutSeconds}else{3600}) -DisableSccache:([bool]$parsed.wrapper.DisableSccache) -SkipOfflineFirst:([bool]$parsed.wrapper.SkipOfflineFirst)
exit [int]$result.exit_code
