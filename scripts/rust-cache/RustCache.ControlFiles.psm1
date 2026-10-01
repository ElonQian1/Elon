function Get-RustCacheControlBytesHash {
    param([Parameter(Mandatory)][AllowEmptyCollection()][byte[]]$Bytes)
    $sha=[Security.Cryptography.SHA256]::Create()
    try{([BitConverter]::ToString($sha.ComputeHash($Bytes))).Replace('-','').ToLowerInvariant()}finally{$sha.Dispose()}
}

function Write-RustCacheBoundControlFile {
    param([Parameter(Mandatory)][string]$Path,[string]$ExpectedSha256,
        [Parameter(Mandatory)][AllowEmptyCollection()][byte[]]$Bytes,[switch]$ExpectedMissing)
    if($Bytes.Length -gt 1MB -or (-not $ExpectedMissing -and $ExpectedSha256 -notmatch '^[a-fA-F0-9]{64}$')){
        throw 'RUST_CACHE_CONTROL_ARGUMENT: A bounded payload and exact previous digest are required.'
    }
    $full=[IO.Path]::GetFullPath($Path);$cursor=[IO.FileInfo]::new($full)
    while($null -ne $cursor){
        if(Test-Path -LiteralPath $cursor.FullName){$item=Get-Item -LiteralPath $cursor.FullName -Force;if($item.Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'RUST_CACHE_CONTROL_REPARSE: Linked control paths are refused.'}}
        $cursor=if($cursor -is [IO.FileInfo]){$cursor.Directory}else{$cursor.Parent}
    }
    $mode=if($ExpectedMissing){[IO.FileMode]::CreateNew}else{[IO.FileMode]::Open}
    try{$stream=[IO.File]::Open($full,$mode,[IO.FileAccess]::ReadWrite,[IO.FileShare]::Read)}
    catch{throw 'RUST_CACHE_CONTROL_BUSY: The exact control file cannot be opened exclusively against other writers and deletion.'}
    $previous=$null;$writing=$false
    try{
        if(-not $ExpectedMissing){
            if($stream.Length -gt 1MB){throw 'RUST_CACHE_CONTROL_SIZE: Existing control file exceeds the bound.'}
            $previous=New-Object byte[] ([int]$stream.Length);$offset=0
            while($offset -lt $previous.Length){$read=$stream.Read($previous,$offset,$previous.Length-$offset);if($read -eq 0){throw 'RUST_CACHE_CONTROL_READ: Incomplete control read.'};$offset+=$read}
            if((Get-RustCacheControlBytesHash $previous) -ine $ExpectedSha256){throw 'RUST_CACHE_CONTROL_DRIFT: Control content changed before the guarded write.'}
        }
        # Keep this handle open throughout comparison, overwrite and durable flush.
        # This is an exclusive guarded write, not an atomic file replacement.
        $writing=$true;$stream.Position=0;$stream.Write($Bytes,0,$Bytes.Length);$stream.SetLength($Bytes.Length);$stream.Flush($true)
        [pscustomobject]@{path=$full;written=$true;created=[bool]$ExpectedMissing;sha256=(Get-RustCacheControlBytesHash $Bytes)}
    }catch{
        if($writing -and -not $ExpectedMissing){
            try{$stream.Position=0;$stream.Write($previous,0,$previous.Length);$stream.SetLength($previous.Length);$stream.Flush($true)}
            catch{throw 'RUST_CACHE_CONTROL_RECOVERY_REQUIRED: Guarded write and same-handle restoration failed; preserve the external backup.'}
        }
        throw
    }finally{$stream.Dispose()}
}

Export-ModuleMember -Function Get-RustCacheControlBytesHash,Write-RustCacheBoundControlFile
