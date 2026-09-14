#requires -Version 7.0

function Test-ChatGptFreshBackgroundAdmission {
    param($Before, $Current)
    foreach ($value in @($Before.attempts, $Current.attempts, $Current.stream_events)) {
        if (($value -isnot [int] -and $value -isnot [long]) -or $value -lt 0 -or $value -gt 65535) { return $false }
    }
    foreach ($value in @($Before.pending, $Before.armed, $Current.pending, $Current.armed, $Current.accepted, $Current.dispatched)) {
        if ($value -isnot [bool]) { return $false }
    }
    return $Before.schema -ceq 'elon.fresh_text_trial.v1' -and
        $Current.schema -ceq 'elon.fresh_text_trial.v1' -and
        $Before.pending -ceq $false -and $Before.armed -ceq $false -and
        $Current.pending -ceq $true -and $Current.accepted -ceq $true -and
        $Current.dispatched -ceq $true -and $Current.armed -ceq $false -and
        $Current.phase -ceq 'streaming' -and $Current.stream_events -gt 0 -and
        $Current.attempts -eq ($Before.attempts + 1)
}

function Invoke-ChatGptFreshBackgroundResume {
    param([Parameter(Mandatory)]$Runtime)
    if (!(Test-WebChatNativeChatSurfaceForeground -Runtime $Runtime)) { throw 'foreground_package_mismatch' }
    $pidBefore = (Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments @('shell','pidof','com.elon.app')).Trim()
    if ($pidBefore -cnotmatch '^[0-9]+$') { throw 'background_process_identity_missing' }
    Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments @('shell','input','keyevent','KEYCODE_HOME') | Out-Null
    Start-Sleep -Seconds 3
    $foreground = Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments @('shell','dumpsys','activity','activities')
    # Never reclaim a different foreground app if the user took over during the pause.
    if ($foreground -cnotmatch '(?m)^\s*topResumedActivity=.*com\.miui\.home/') { throw 'background_launcher_not_owned' }
    $opened = Invoke-ChatGptWebSmokeAction -Runtime $Runtime -EnsureMainActivity -Action open_social_ai_chat `
        -Arguments @{wait_for_target_bind_ms=12000}
    if ($opened.control_ok -cne $true -or !(Test-WebChatNativeChatSurfaceForeground -Runtime $Runtime)) {
        throw 'background_native_return_failed'
    }
    $pidAfter = (Invoke-ChatGptWebSmokeAdb -Runtime $Runtime -Arguments @('shell','pidof','com.elon.app')).Trim()
    if ($pidBefore -cne $pidAfter) { throw 'background_process_recreated' }
    return [ordered]@{exercised=$true;launcher_observed=$true;native_returned=$true;process_retained=$true;pause_seconds=3}
}
