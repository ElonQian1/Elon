#requires -Version 7.0

param(
    [string]$MainBase = "",
    [string]$MainToken = "",
    [string]$Fb2Base = "",
    [string]$Fb2Token = "",
    [string]$Fb2Username = "",
    [string]$Fb2Password = "",
    [string]$GroupId = "",
    [string]$MentionText = "",
    [string]$SelectedMessageText = "",
    [int]$RequestTimeoutSec = 45,
    [int]$PollTimeoutSec = 90,
    [int]$PollIntervalSec = 3,
    [switch]$AllowVisibleMessages,
    [switch]$SkipMention,
    [switch]$SkipSelectedMessage
)

$ErrorActionPreference = "Stop"

if (-not $MainBase) { $MainBase = $env:ELON_MAIN_BASE }
if (-not $MainBase) { $MainBase = "http://43.139.149.158:8080" }
if (-not $MainToken) { $MainToken = $env:ELON_MAIN_TOKEN }
if (-not $Fb2Base) { $Fb2Base = $env:FB2_API_BASE }
if (-not $Fb2Base) { $Fb2Base = "http://123.207.48.146:8080" }
if (-not $Fb2Token) { $Fb2Token = $env:FB2_USER_TOKEN }
if (-not $Fb2Username) { $Fb2Username = $env:FB2_VISIBLE_SMOKE_USERNAME }
if (-not $Fb2Password) { $Fb2Password = $env:FB2_VISIBLE_SMOKE_PASSWORD }

$MainBase = $MainBase.TrimEnd("/")
$Fb2Base = $Fb2Base.TrimEnd("/")
$script:Failed = 0
$script:Skipped = 0

function Write-Check {
    param([string]$Status, [string]$Name, [string]$Detail = "")
    if ($Detail) {
        Write-Output "$Status`t$Name`t$Detail"
    } else {
        Write-Output "$Status`t$Name"
    }
}

function Pass {
    param([string]$Name, [string]$Detail = "")
    Write-Check "OK" $Name $Detail
}

function Fail {
    param([string]$Name, [string]$Detail = "")
    $script:Failed += 1
    Write-Check "FAIL" $Name $Detail
}

function Skip {
    param([string]$Name, [string]$Detail = "")
    $script:Skipped += 1
    Write-Check "SKIP" $Name $Detail
}

function Assert-True {
    param([bool]$Condition, [string]$Name, [string]$Detail = "")
    if ($Condition) {
        Pass $Name $Detail
    } else {
        Fail $Name $Detail
    }
}

function Encode-PathSegment {
    param([string]$Value)
    [System.Uri]::EscapeDataString($Value)
}

function Invoke-Json {
    param(
        [Parameter(Mandatory = $true)][string]$Url,
        [hashtable]$Headers = @{},
        [string]$Method = "GET",
        [object]$Body = $null
    )

    $params = @{
        Uri = $Url
        Method = $Method
        Headers = $Headers
        TimeoutSec = $RequestTimeoutSec
    }
    if ($null -ne $Body) {
        $params["ContentType"] = "application/json"
        $params["Body"] = ($Body | ConvertTo-Json -Depth 12 -Compress)
    }
    Invoke-RestMethod @params
}

function Get-MessageId {
    param([object]$Payload)
    if ($Payload.message.id) { return [string]$Payload.message.id }
    if ($Payload.id) { return [string]$Payload.id }
    if ($Payload.data.message.id) { return [string]$Payload.data.message.id }
    if ($Payload.data.id) { return [string]$Payload.data.id }
    return ""
}

function Get-Messages {
    param([string]$BearerToken, [string]$TargetGroupId)
    $headers = @{ Authorization = "Bearer $BearerToken" }
    $groupPath = Encode-PathSegment $TargetGroupId
    $payload = Invoke-Json -Url "$MainBase/api/me/groups/$groupPath/messages?limit=80" -Headers $headers
    return @($payload.messages)
}

function Wait-For-AiReply {
    param(
        [string]$BearerToken,
        [string]$TargetGroupId,
        [string]$AfterMessageId,
        [string[]]$KnownMessageIds,
        [string]$Scenario
    )

    $deadline = (Get-Date).AddSeconds($PollTimeoutSec)
    $known = [System.Collections.Generic.HashSet[string]]::new()
    foreach ($id in $KnownMessageIds) {
        [void]$known.Add($id)
    }

    while ((Get-Date) -lt $deadline) {
        $messages = Get-Messages -BearerToken $BearerToken -TargetGroupId $TargetGroupId
        $seenAnchor = [string]::IsNullOrWhiteSpace($AfterMessageId)
        foreach ($message in $messages) {
            $id = [string]$message.id
            if ($id -eq $AfterMessageId) {
                $seenAnchor = $true
                continue
            }
            if (-not $seenAnchor) {
                continue
            }
            $sender = [string]$message.sender_user_id
            if (($sender -eq "usr_elon_ai" -or $id.StartsWith("gai_")) -and -not $known.Contains($id)) {
                return $message
            }
        }
        Start-Sleep -Seconds $PollIntervalSec
    }

    throw "$Scenario did not receive an AI reply within $PollTimeoutSec seconds"
}

function Resolve-MainToken {
    if ($MainToken) {
        return [pscustomobject]@{
            Token = $MainToken.Trim()
            Fb2UserId = ""
            Source = "main-token"
        }
    }

    if (-not $Fb2Token) {
        if (-not $Fb2Username -or -not $Fb2Password) {
            throw "Set ELON_MAIN_TOKEN, FB2_USER_TOKEN, or provide -Fb2Username/-Fb2Password."
        }
        $login = Invoke-Json -Url "$Fb2Base/api/auth/login" -Method "POST" -Body @{
            username = $Fb2Username
            password = $Fb2Password
        }
        if (-not $login.success -or -not $login.data.token.access_token) {
            throw "fb2 login failed"
        }
        $Fb2Token = [string]$login.data.token.access_token
        $script:Fb2LoginUserId = [string]$login.data.user.id
    }

    $fb2Headers = @{ Authorization = "Bearer $($Fb2Token.Trim())" }
    $session = Invoke-Json -Url "$Fb2Base/api/main-project/session" -Headers $fb2Headers -Method "POST" -Body @{
        deviceName = "main-visible-chat-smoke"
    }
    if (-not $session.success -or -not $session.data.token) {
        throw "fb2 main-project session bridge failed"
    }

    return [pscustomobject]@{
        Token = [string]$session.data.token
        Fb2UserId = [string]$script:Fb2LoginUserId
        Source = "fb2-session-bridge"
    }
}

Write-Output "== Visible fb2 group chat smoke =="

if (-not $AllowVisibleMessages) {
    Fail "visible message permission" "Pass -AllowVisibleMessages only after explicit authorization; this script writes to the group."
    Write-Output ""
    Write-Output "== Summary =="
    Write-Output "failed=$script:Failed skipped=$script:Skipped"
    exit 1
}

$resolved = Resolve-MainToken
$token = $resolved.Token
$headers = @{ Authorization = "Bearer $token" }
Pass "main token resolved" $resolved.Source
if ($resolved.Fb2UserId) {
    Pass "fb2 user" $resolved.Fb2UserId
}

$bootstrap = Invoke-Json -Url "$MainBase/api/external/apps/fb2/chat-bootstrap" -Headers $headers
if (-not $GroupId) {
    $GroupId = [string]$bootstrap.chat.defaultGroupId
}
Assert-True ($bootstrap.aiReply.schema -eq "external_app.ai_reply.v1") "chat-bootstrap aiReply"
Assert-True ([bool]$GroupId) "target group" $GroupId

$groups = Invoke-Json -Url "$MainBase/api/me/groups" -Headers $headers
$targetGroup = @($groups.groups) | Where-Object { $_.id -eq $GroupId } | Select-Object -First 1
Assert-True ($null -ne $targetGroup) "group membership" $GroupId

$baselineMessages = Get-Messages -BearerToken $token -TargetGroupId $GroupId
$baselineIds = @($baselineMessages | ForEach-Object { [string]$_.id })
$trace = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")

if ($SkipMention) {
    Skip "visible @EL mention" "skipped by caller"
} else {
    if (-not $MentionText) {
        $MentionText = "@EL 可见smoke ${trace}: 请用 fb2 数据简短说明今天比赛怎么看，并引用来源。"
    }
    $groupPath = Encode-PathSegment $GroupId
    $sent = Invoke-Json -Url "$MainBase/api/me/groups/$groupPath/messages" -Headers $headers -Method "POST" -Body @{
        content = $MentionText
    }
    $sentId = Get-MessageId $sent
    Assert-True ([bool]$sentId) "visible @EL sent" $sentId
    $reply = Wait-For-AiReply -BearerToken $token -TargetGroupId $GroupId -AfterMessageId $sentId -KnownMessageIds $baselineIds -Scenario "@EL mention"
    Assert-True ([bool]$reply.id) "visible @EL ai reply" "$($reply.id)"
}

$latestMessages = Get-Messages -BearerToken $token -TargetGroupId $GroupId
$knownIdsForSelected = @($latestMessages | ForEach-Object { [string]$_.id })

if ($SkipSelectedMessage) {
    Skip "selected-message AI回复" "skipped by caller"
} else {
    if (-not $SelectedMessageText) {
        $SelectedMessageText = "可见smoke ${trace}: 这条消息用于长按 AI回复 验证，请判断这句说法是否合理：西班牙让两球肯定赢盘、可以重注。"
    }
    $groupPath = Encode-PathSegment $GroupId
    $plain = Invoke-Json -Url "$MainBase/api/me/groups/$groupPath/messages" -Headers $headers -Method "POST" -Body @{
        content = $SelectedMessageText
    }
    $plainId = Get-MessageId $plain
    Assert-True ([bool]$plainId) "selected-message seed sent" $plainId

    $messagePath = Encode-PathSegment $plainId
    $request = Invoke-Json -Url "$MainBase/api/me/groups/$groupPath/messages/$messagePath/ai-reply" -Headers $headers -Method "POST"
    Assert-True (($request.ok -eq $true) -or ($null -ne $request)) "selected-message ai-reply accepted" $plainId
    $reply = Wait-For-AiReply -BearerToken $token -TargetGroupId $GroupId -AfterMessageId $plainId -KnownMessageIds $knownIdsForSelected -Scenario "selected-message ai-reply"
    Assert-True ([bool]$reply.id) "selected-message ai reply" "$($reply.id)"
}

Write-Output ""
Write-Output "== Summary =="
Write-Output "failed=$script:Failed skipped=$script:Skipped"
if ($script:Failed -gt 0) {
    exit 1
}
