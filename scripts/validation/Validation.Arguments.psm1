function Split-ValidationCargoArguments {
    param(
        [Parameter(Mandatory)][object[]]$Arguments,
        [Parameter(Mandatory)][hashtable]$ValueOptions,
        [Parameter(Mandatory)][AllowEmptyCollection()][string[]]$SwitchOptions
    )
    $wrapper = @{}
    $cargo = [Collections.Generic.List[string]]::new()
    $afterSeparator = $false
    for ($index = 0; $index -lt $Arguments.Count; $index++) {
        $argument = [string]$Arguments[$index]
        if ($afterSeparator) { $cargo.Add($argument); continue }
        if ($argument -eq '--') { $afterSeparator = $true; continue }
        if ($ValueOptions.ContainsKey($argument)) {
            if (++$index -ge $Arguments.Count) { throw "Missing value for wrapper option: $argument" }
            $wrapper[$ValueOptions[$argument]] = [string]$Arguments[$index]
            continue
        }
        $switchName = $SwitchOptions | Where-Object { $_ -ieq $argument } | Select-Object -First 1
        if ($switchName) { $wrapper[$switchName.TrimStart('-')] = $true; continue }
        # Backward compatibility: the first non-wrapper token starts Cargo args.
        $afterSeparator = $true
        $cargo.Add($argument)
    }
    [pscustomobject]@{ wrapper = $wrapper; cargo = @($cargo) }
}

Export-ModuleMember -Function Split-ValidationCargoArguments
