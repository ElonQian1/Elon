function New-ElonMobilePwaRuntimeTemplate {
    param(
        [Parameter(Mandatory = $true)][string]$TemplatePath,
        [Parameter(Mandatory = $true)][string]$StylesPath,
        [Parameter(Mandatory = $true)][string]$ThemeStylesPath,
        [Parameter(Mandatory = $true)][string]$CacheScriptPath,
        [Parameter(Mandatory = $true)][string]$ScriptPath,
        [Parameter(Mandatory = $true)][string]$OutputPath
    )
    # Bundle the complete ordered startup generation, not a subset of its assets.
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    $template = [System.IO.File]::ReadAllText($TemplatePath, $utf8)
    $assetRoot = Split-Path -Parent $TemplatePath
    $overrides = @{
        'project_plaza.css' = $StylesPath
        'orbital_mobile_theme.css' = $ThemeStylesPath
        'project_plaza_cache.js' = $CacheScriptPath
        'project_plaza.js' = $ScriptPath
    }
    $pattern = '<script\b[^>]*\bsrc="(?<js>/assets/[\w.-]+\.js)(?:\?[^"<>]*)?"[^>]*>\s*</script>|<link\b(?=[^>]*\brel="stylesheet")(?=[^>]*\bhref="(?<css>/assets/[\w.-]+\.css)(?:\?[^"<>]*)?")[^>]*>'
    $runtimeTemplate = [regex]::Replace($template, $pattern, [System.Text.RegularExpressions.MatchEvaluator]{
        param($match)
        $isScript = $match.Groups['js'].Success
        $url = if ($isScript) { $match.Groups['js'].Value } else { $match.Groups['css'].Value }
        $name = $url.Substring('/assets/'.Length)
        $path = if ($overrides.ContainsKey($name)) { $overrides[$name] } else { Join-Path $assetRoot $name }
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing PWA startup asset: $name" }
        $content = [System.IO.File]::ReadAllText($path, $utf8)
        $tag = if ($isScript) { 'script' } else { 'style' }
        if ($isScript) { $content = [regex]::Replace($content, '(?i)</script', '<\/script') }
        elseif ($content -match '(?i)</style\s*>') { throw "PWA asset cannot be safely embedded: $name" }
        "<$tag data-elon-runtime-asset=`"$url`">`n$content`n</$tag>"
    })
    if ($runtimeTemplate -match '(?:src|href)="/assets/[\w.-]+\.(?:js|css)(?:\?[^"<>]*)?"') {
        throw 'Unbundled mobile startup asset; refuse a partial runtime generation.'
    }
    [System.IO.Directory]::CreateDirectory((Split-Path -Parent $OutputPath)) | Out-Null
    [System.IO.File]::WriteAllText($OutputPath, $runtimeTemplate, $utf8)
    Get-Item -LiteralPath $OutputPath
}
