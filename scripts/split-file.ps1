param(
    [string]$SrcFile,
    [int]$StartIdx,
    [string]$ImplFile
)
$enc = [System.Text.UTF8Encoding]::new($false)
$FULL = [System.IO.File]::ReadAllLines($SrcFile)
Write-Host "src: $($FULL.Count) lines, extracting from index $StartIdx (L$($StartIdx+1))"
$dir = Split-Path $SrcFile -Parent
$implPath = "$dir/$ImplFile"
$implLines = @("use super::*;", "") + $FULL[$StartIdx..($FULL.Count - 1)]
[System.IO.File]::WriteAllLines($implPath, $implLines, $enc)
$c = [IO.File]::ReadAllText($implPath)
$c = $c -replace '(?m)^fn ', 'pub(super) fn '
$c = $c -replace '(?m)^async fn ', 'pub(super) async fn '
$c = $c -replace '(?m)^const ', 'pub(super) const '
$c = $c -replace '(?m)^struct ', 'pub(super) struct '
$c = $c -replace '(?m)^enum ', 'pub(super) enum '
[IO.File]::WriteAllText($implPath, $c, $enc)
$mainLines = $FULL[0..($StartIdx - 1)] + @("", "#[path = `"$ImplFile`"]", "mod impl_funcs;", "use self::impl_funcs::*;")
[System.IO.File]::WriteAllLines($SrcFile, $mainLines, $enc)
Write-Host "main:$(([System.IO.File]::ReadAllLines($SrcFile)).Count) impl:$(([System.IO.File]::ReadAllLines($implPath)).Count)"
