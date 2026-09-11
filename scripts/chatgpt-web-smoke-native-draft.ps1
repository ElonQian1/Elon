#requires -Version 7.0

function Test-ChatGptWebNativeDraftEmpty {
    param([AllowNull()]$State)

    function Field($Value, [string]$Name) {
        if ($null -eq $Value) { return $null }
        if ($Value -is [System.Collections.IDictionary]) { return $Value[$Name] }
        $property = $Value.PSObject.Properties[$Name]
        if ($null -eq $property) { return $null }
        return $property.Value
    }

    $inputState = Field $State 'input'
    $present = Field $inputState 'has_text'
    $length = Field $inputState 'text_length'
    # Main MCP intentionally omits draft text. Missing or contradictory metadata
    # must not authorize navigation, mutation, or a successful restoration.
    return $present -is [bool] -and -not $present -and
        ($length -is [int] -or $length -is [long]) -and $length -eq 0
}
