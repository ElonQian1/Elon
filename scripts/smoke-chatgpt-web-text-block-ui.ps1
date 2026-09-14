#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$DeviceSerial,
    [Parameter(Mandatory)][string]$ExpectedHardwareSerial,
    [switch]$CreateFixture,
    [string]$ExistingFixturePath='',
    [ValidateSet('code','writing_block')][string[]]$RequiredKinds=@('writing_block','code'),
    [ValidateSet('md','txt','docx','pdf')][string]$WritingFormat='md',
    [switch]$ReturnHome,
    [string]$Adb='D:/Android/sdk/platform-tools/adb.exe'
)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'chatgpt-web-smoke-runtime.ps1')
. (Join-Path $PSScriptRoot 'invoke-android-semantic-acceptance.ps1')
. (Join-Path $PSScriptRoot 'chatgpt-text-block-docx-evidence.ps1')
if(-not $CreateFixture -and -not $ExistingFixturePath){throw 'fixture_creation_required'}
if($WritingFormat -cin @('docx','pdf') -and (-not $ExistingFixturePath -or $RequiredKinds.Count -ne 1 -or
    $RequiredKinds[0] -cne 'writing_block')){throw 'docx_requires_existing_writing_fixture'}
$docxFixture=if($WritingFormat -cin @('docx','pdf')){Get-ChatGptTextBlockDocxFixture}else{$null}
$r=New-ChatGptWebSmokeRuntime -Adb $Adb -DeviceSerial $DeviceSerial -ExpectedHardwareSerial $ExpectedHardwareSerial
$report=[ordered]@{schema='elon.text_block_ui.v1';passed=$false;required_kinds=$RequiredKinds;stage='prepare';sent=0;blocks=@();restored=$false;awake_restored=$false;cloud_writes=0;private_content_exported=$false}
$origin=$null; $changed=$false; $opened=$false; $resetHash=''; $fixturePath=''
$prompt='ELON_EXTENDED_TOOL_ACCEPTANCE_V1: For a native block editor test, draft a short email in your editable writing block. Put the exact marker ELON_TEXT_BLOCK_ACCEPTANCE_V1 in the email body. Then add one separate Python fenced code block containing only print("ELON_TEXT_BLOCK_ACCEPTANCE_V1"). Do not use Canvas or execute anything.'
function Main { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state -MainState }
function Web { Invoke-ChatGptWebSmokeMcp -Runtime $r -Tool ui_state }
function Act([string]$Name,[hashtable]$Arguments=@{}) { Invoke-ChatGptWebSmokeAction -Runtime $r -Action $Name -Arguments $Arguments }
function Ui([string]$Step,[hashtable]$Parameters=@{}) {
    $report.ui_step=$Step
    Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass TextBlockUiAcceptance -Step $Step -Parameters $Parameters -ResultPrefix TEXT_BLOCK_UI_RESULT
}
function Stage([string]$Name) { $report.stage=$Name; Write-Host "TEXT_BLOCK_STAGE=$Name" }
try {
    Assert-ChatGptWebSmokeTrustedDevice -Runtime $r
    if(-not (Get-ChatGptWebSmokeUserReadiness -Runtime $r).ready){throw 'device_locked'}
    $origin=Main; $before=Web
    if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_package_mismatch'}
    if($origin.active_surface -ne 'social_ai' -or $origin.social_chat.web_chat_provider_id -ne 'chatgpt_web' -or
        $origin.input.has_text -ne $false -or -not $before.authenticated -or $before.streaming -or $before.dictation_active -or
        [int]$before.input.official_draft_length -ne 0){throw 'idle_native_chat_required'}
    if($docxFixture -and $before.private_voice_native_research.phase -cne 'idle'){throw 'idle_native_voice_required'}
    Assert-ChatGptWebSmokeAdapterVersion -State $before -ExpectedAdapterVersion (Resolve-ChatGptWebSmokeExpectedAdapterVersion)
    Start-ChatGptWebSmokeAwakeLease -Runtime $r|Out-Null
    $changed=$true
    if($ExistingFixturePath){
        if($ExistingFixturePath -cnotmatch '^/c/[a-f0-9-]{36}$'){throw 'invalid_fixture_path'}
        Act 'open_web_chat_conversation' @{conversation_path=$ExistingFixturePath}|Out-Null
        $report.reused_fixture=$true
    }else{
        Act 'start_new_web_chat_conversation'|Out-Null
        Wait-ChatGptWebSmokeState -Runtime $r -MainState -TimeoutSec 35 -Description 'new fixture conversation' -Predicate {
            param($s) $s.social_chat.web_chat_composer_ready -and $s.social_chat.message_count -eq 0
        }|Out-Null
        Stage 'native_send'
        Act 'set_input_text' @{text=$prompt}|Out-Null
        Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass CanvasUiAcceptance -Step send_fixture -ResultPrefix CANVAS_UI_RESULT|Out-Null
        $report.sent=1
    }
    $deadline=[DateTimeOffset]::UtcNow.AddSeconds(100)
    $parts=@()
    do {
        $web=Web
        $users=@($web.conversation.messages|Where-Object role -eq user)
        if($users.Count -eq 1 -and $users[0].content -ceq $prompt -and -not $web.streaming){
            $answers=@($web.conversation.messages|Where-Object { $_.role -eq 'assistant' -and $_.state -eq 'completed' })
            if($answers.Count){
                $fixturePath=([uri]$web.conversation.url).AbsolutePath
                foreach($answer in $answers){
                    for($i=0;$i -lt @($answer.parts).Count;$i++){
                        $part=$answer.parts[$i]
                        if($part.type -in @('code','writing_block')){ $parts+=@{message=$answer.id;index=$i;part=$part} }
                    }
                }
                break
            }
        }
        Start-Sleep -Seconds 2
    }while([DateTimeOffset]::UtcNow -lt $deadline)
    if(-not $parts.Count){throw 'provider_text_block_sample_missing'}
    foreach($item in $parts|Where-Object {$_.part.type -in $RequiredKinds}|Select-Object -First 2){
        Stage "native_$($item.part.type)"
        Act 'chatgpt_reveal_message' @{message_id=$item.message;part_index=$item.index;target='message'}|Out-Null
        $messageKey=([string]$item.message -creplace '[^A-Za-z0-9_.:-]','_')
        $selector="web-chat-message-part:chatgpt_web:${messageKey}:$($item.index):$($item.part.type)"
        $opened=$true
        $original=(Ui open @{selector=$selector}).body
        $edited=if($docxFixture){(Ui edit_docx_fixture @{expected_hash=$original.sha256;source_base64=$docxFixture.source_base64}).body}
            else{(Ui edit @{expected_hash=$original.sha256}).body}
        $resetHash=$edited.sha256
        if($docxFixture -and $edited.sha256 -cne $docxFixture.source_sha256){throw 'docx_fixture_not_applied'}
        $history=Ui history @{expected_hash=$edited.sha256;original_hash=$original.sha256}
        if(-not $history.undo -or -not $history.redo){throw 'native_history_unconfirmed'}
        $stem='elon-block-acceptance-'+[Guid]::NewGuid().ToString('N').Substring(0,16)
        $extension=if($item.part.type -eq 'writing_block'){$WritingFormat}else{'py'}
        $export=Ui export @{expected_hash=$edited.sha256;stem=$stem;extension=$extension}
        if($export.extension -cne $extension){throw 'export_format_mismatch'}
        $names=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','find','/sdcard/Download','-maxdepth','1','-name',"elon-*-$stem.$extension") -Label 'locate owned block export'
        $paths=@($names.Trim() -split "`r?`n" | Where-Object {$_})
        if($paths.Count -ne 1 -or $paths[0] -cnotmatch "^/sdcard/Download/elon-[a-f0-9-]+-$stem\.$extension`$"){throw 'owned_export_missing'}
        $checksum=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','sha256sum',$paths[0]) -Label 'verify owned export bytes'
        $fileHash=($checksum -split '\s+')[0]
        $docxEvidence=$null
        $pdfEvidence=$null
        if($docxFixture){
            if($WritingFormat -ceq 'docx'){Assert-ChatGptDocxExportReceipt $export $docxFixture}
            elseif($export.exported -ne $true -or $export.source_sha256 -cne $docxFixture.source_sha256){
                throw 'pdf_export_receipt_mismatch'
            }
            $size=Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('shell','stat','-c','%s',$paths[0]) -Label 'check owned document size'
            if($size.Trim() -cnotmatch '^[0-9]+$' -or [long]$size -gt 1048576){throw 'docx_file_boundary'}
            $output=Join-Path (Split-Path -Parent $PSScriptRoot) ".ai-tmp/$stem"
            New-Item -ItemType Directory -Force -Path $output|Out-Null
            $local=Join-Path $output "export.$extension"
            Invoke-ChatGptWebSmokeAdb -Runtime $r -Arguments @('pull',$paths[0],$local) -Label 'read owned document export'|Out-Null
            if($WritingFormat -ceq 'docx'){
                $docxEvidence=Assert-ChatGptDocxExportFile -Path $local -ExpectedFileHash $fileHash -Fixture $docxFixture
            }else{
                $bytes=[IO.File]::ReadAllBytes($local)
                if($fileHash -cnotmatch '^[a-f0-9]{64}$' -or $bytes.Length -lt 8 -or
                    $bytes.Length -ne [long]$size.Trim() -or
                    (Get-FileHash -LiteralPath $local -Algorithm SHA256).Hash.ToLowerInvariant() -cne $fileHash -or
                    [Text.Encoding]::ASCII.GetString($bytes,0,5) -cne '%PDF-'){throw 'pdf_export_bytes_mismatch'}
                # Byte/container admission only; a real PDF parser/renderer must inspect this owned artifact.
                $pdfEvidence=@{file_sha256=$fileHash;bytes=$bytes.Length;source_sha256=$docxFixture.source_sha256;
                    parser_verified=$false;artifact=$local}
            }
        }elseif($fileHash -cne $export.sha256){throw 'export_bytes_mismatch'}
        $actions=Ui export_actions @{expected_hash=$edited.sha256}
        if(-not $actions.export_actions_available){throw 'native_export_actions_missing'}
        $share=Ui share_export @{expected_hash=$edited.sha256}
        if(-not $share.share_picker_opened -or $share.file_sent){throw 'native_share_picker_unconfirmed'}
        $reset=(Ui reset @{expected_hash=$edited.sha256}).body
        if($reset.sha256 -cne $original.sha256){throw 'native_reset_mismatch'}
        $resetHash=''
        Ui close|Out-Null; $opened=$false
        $reopened=(Ui open @{selector=$selector}).body; $opened=$true
        if($reopened.sha256 -cne $original.sha256){throw 'local_copy_changed_source'}
        Ui close|Out-Null; $opened=$false
        $report.blocks+=@{kind=$item.part.type;native_editor=$true;edited=$true;undo=$history.undo;redo=$history.redo;export_extension=$extension;export_bytes_match=$true;docx=$docxEvidence;pdf=$pdfEvidence;export_actions_available=$actions.export_actions_available;share_picker_opened=$share.share_picker_opened;reset=$true;source_unchanged=$true}
    }
    $kinds=@($report.blocks|ForEach-Object kind)
    if(@($RequiredKinds|Where-Object {$_ -notin $kinds}).Count){throw 'provider_variant_sample_missing'}
    Stage 'complete'
} catch {
    $message=[string]$_.Exception.Message
    $report.error=if($message -cmatch '^[a-z0-9_]+$'){$message}
        elseif($message -match '^Semantic UI acceptance failed: ([a-z_]+)$'){$Matches[1]}
        elseif($message -match '^APK MCP tool failed: .*error=([a-z0-9_]+)'){$Matches[1]}
        else{'text_block_acceptance_failed'}
    $report.error_line=$_.InvocationInfo.ScriptLineNumber
} finally {
    try {
        if(-not (Test-WebChatNativeChatSurfaceForeground -Runtime $r)){throw 'foreground_changed_skip_cleanup'}
        if($opened){
            if($report.ui_step -in @('export','export_actions','share_export')){try{Ui cancel_export|Out-Null}catch{}}
            if($resetHash){
                $cleanupBody=(Ui inspect).body
                if($cleanupBody.sha256 -ceq $resetHash){Ui reset @{expected_hash=$resetHash}|Out-Null}
                elseif($cleanupBody.sha256 -cne $original.sha256){throw 'local_edit_cleanup_owner_changed'}
            }
            Ui close|Out-Null
        }
        $current=Main
        if($changed -and $current.input.has_text){
            Invoke-AndroidSemanticAcceptance -Runtime $r -TestClass CanvasUiAcceptance -Step clear_fixture_draft -ResultPrefix CANVAS_UI_RESULT|Out-Null
        }
        if($changed -and $origin){$report.restored=Restore-WebChatNativeConversation -Runtime $r -ProviderId chatgpt_web -ConversationPath $origin.social_chat.web_chat_conversation_path -TimeoutSec 45}
        $after=Main
        $report.restored=$report.restored -and $after.input.text_length -eq $origin.input.text_length
        if($ReturnHome -and $report.restored){Act show_conversation_home|Out-Null}
    }catch{$report.cleanup_failed=$true}
    try {$report.awake_restored=Stop-ChatGptWebSmokeAwakeLease -Runtime $r}catch{$report.awake_restored=$false}
    $report.fixture_retained=($fixturePath -ne '')
    $report.passed=$report.stage -eq 'complete' -and $report.restored -and $report.awake_restored -and -not $report.cleanup_failed
    $report|ConvertTo-Json -Depth 5 -Compress
}
if(-not $report.passed){exit 1}
