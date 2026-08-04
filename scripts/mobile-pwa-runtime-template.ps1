function New-ElonMobilePwaRuntimeTemplate {
    param(
        [Parameter(Mandatory = $true)]
        [string]$TemplatePath,
        [Parameter(Mandatory = $true)]
        [string]$StylesPath,
        [Parameter(Mandatory = $true)]
        [string]$CacheScriptPath,
        [Parameter(Mandatory = $true)]
        [string]$ScriptPath,
        [Parameter(Mandatory = $true)]
        [string]$OutputPath
    )

    $utf8 = [System.Text.UTF8Encoding]::new($false)
    $template = [System.IO.File]::ReadAllText($TemplatePath, $utf8)
    $styles = [System.IO.File]::ReadAllText($StylesPath, $utf8)
    $cacheScript = [System.IO.File]::ReadAllText($CacheScriptPath, $utf8)
    $script = [System.IO.File]::ReadAllText($ScriptPath, $utf8)
    $styleReference = '<link rel="stylesheet" href="/assets/project_plaza.css" />'
    $cacheScriptReference = '<script src="/assets/project_plaza_cache.js"></script>'
    $scriptReference = '<script src="/assets/project_plaza.js"></script>'

    if (-not $template.Contains($styleReference)) {
        throw "Mobile PWA template is missing the project plaza stylesheet reference: $TemplatePath"
    }
    if (-not $template.Contains($scriptReference)) {
        throw "Mobile PWA template is missing the project plaza script reference: $TemplatePath"
    }
    if (-not $template.Contains($cacheScriptReference)) {
        throw "Mobile PWA template is missing the project plaza cache script reference: $TemplatePath"
    }
    if ($styles -match '(?i)</style\s*>') {
        throw "Project plaza styles cannot be embedded safely: $StylesPath"
    }
    if ($script -match '(?i)</script\s*>') {
        throw "Project plaza script cannot be embedded safely: $ScriptPath"
    }
    if ($cacheScript -match '(?i)</script\s*>') {
        throw "Project plaza cache script cannot be embedded safely: $CacheScriptPath"
    }

    $styleBlock = "<style data-elon-runtime-asset=`"/assets/project_plaza.css`">`n$styles`n</style>"
    $cacheScriptBlock = "<script data-elon-runtime-asset=`"/assets/project_plaza_cache.js`">`n$cacheScript`n</script>"
    $scriptBlock = "<script data-elon-runtime-asset=`"/assets/project_plaza.js`">`n$script`n</script>"
    $runtimeTemplate = $template.Replace($styleReference, $styleBlock)
    $runtimeTemplate = $runtimeTemplate.Replace($cacheScriptReference, $cacheScriptBlock)
    $runtimeTemplate = $runtimeTemplate.Replace($scriptReference, $scriptBlock)
    $outputDirectory = Split-Path -Parent $OutputPath
    if (-not [string]::IsNullOrWhiteSpace($outputDirectory)) {
        [System.IO.Directory]::CreateDirectory($outputDirectory) | Out-Null
    }
    [System.IO.File]::WriteAllText($OutputPath, $runtimeTemplate, $utf8)
    Get-Item -LiteralPath $OutputPath
}
