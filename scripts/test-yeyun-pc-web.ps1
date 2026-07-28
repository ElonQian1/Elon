param(
    [string]$BaseUrl = "http://43.139.149.158:8080",
    [string]$Token = "",
    [string]$Account = "",
    [string]$Password = "",
    [string]$NodeHint = "",
    [string]$ProjectId = "",
    [string]$ChannelId = "",
    [ValidateSet("auto", "route_c", "route_c2", "route_c3")]
    [string]$RuntimeRoute = "route_c3",
    [switch]$RunChatProbe,
    [switch]$RunProjectProbe
)

$ErrorActionPreference = "Stop"

function Write-Step($name, $status, $detail = "") {
    $line = "[{0}] {1}" -f $status, $name
    if ($detail) { $line = "$line - $detail" }
    Write-Host $line
}

function Join-ApiUrl($path) {
    $base = $BaseUrl.TrimEnd("/")
    if ($path.StartsWith("/")) { return "$base$path" }
    return "$base/$path"
}

function ConvertTo-JsonBody($body) {
    if ($null -eq $body) { return $null }
    return ($body | ConvertTo-Json -Depth 30 -Compress)
}

function Read-ErrorBody($response) {
    if ($null -eq $response) { return "" }
    try {
        if ($response.Content -and $response.Content.GetType().GetMethod("ReadAsStringAsync")) {
            return $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        }
        $stream = $response.GetResponseStream()
        if ($null -eq $stream) { return "" }
        $reader = New-Object System.IO.StreamReader($stream)
        return $reader.ReadToEnd()
    } catch {
        return ""
    }
}

function Invoke-ElonJson($method, $path, $body = $null, [bool]$auth = $true) {
    $headers = @{ "Accept" = "application/json" }
    if ($auth -and $script:Token) {
        $headers["Authorization"] = "Bearer $script:Token"
    }
    $json = ConvertTo-JsonBody $body
    $params = @{
        Method = $method
        Uri = Join-ApiUrl $path
        Headers = $headers
        UseBasicParsing = $true
        TimeoutSec = 30
    }
    if ($null -ne $json) {
        $params["ContentType"] = "application/json"
        $params["Body"] = $json
    }
    try {
        $res = Invoke-WebRequest @params
        if (-not $res.Content) { return $null }
        return ($res.Content | ConvertFrom-Json)
    } catch {
        $bodyText = ""
        if ($_.ErrorDetails -and $_.ErrorDetails.Message) {
            $bodyText = [string]$_.ErrorDetails.Message
        }
        if (-not $bodyText) {
            $bodyText = Read-ErrorBody $_.Exception.Response
        }
        if ($bodyText) {
            throw "$($_.Exception.Message) :: $bodyText"
        }
        throw
    }
}

function Find-Node($nodes, $hint) {
    if ($null -eq $nodes) { return $null }
    $cleanHint = [string]$hint
    if (-not $cleanHint.Trim()) { return $null }
    $needle = $cleanHint.Trim().ToLowerInvariant()
    foreach ($node in @($nodes)) {
        $haystack = @(
            $node.node_id,
            $node.agent_id,
            $node.short_id,
            $node.display_name,
            $node.device_name,
            $node.label,
            $node.owner_user_id
        ) -join " "
        if ($haystack.ToLowerInvariant().Contains($needle)) {
            return $node
        }
    }
    return $null
}

Write-Host ""
Write-Host "Elon PC web / YeYun node smoke test"
Write-Host "BaseUrl: $BaseUrl"
Write-Host ""

try {
    $health = Invoke-WebRequest -UseBasicParsing -Uri (Join-ApiUrl "/health") -TimeoutSec 10
    Write-Step "server health" "PASS" $health.Content.Trim()
} catch {
    Write-Step "server health" "FAIL" $_.Exception.Message
    exit 1
}

try {
    $version = Invoke-ElonJson "GET" "/api/server/version" $null $false
    Write-Step "server version" "PASS" ("{0} {1}" -f $version.versionName, $version.gitSha)
} catch {
    Write-Step "server version" "FAIL" $_.Exception.Message
}

try {
    $pcPage = Invoke-WebRequest -UseBasicParsing -Uri (Join-ApiUrl "/pc") -TimeoutSec 10
    Write-Step "pc web entry" "PASS" ("HTTP 200, {0} bytes" -f $pcPage.Content.Length)
} catch {
    Write-Step "pc web entry" "FAIL" $_.Exception.Message
}

if (-not $Token -and $Account -and $Password) {
    try {
        $login = Invoke-ElonJson "POST" "/api/auth/login" @{
            account = $Account
            password = $Password
            device_name = "pc-web-yeyun-smoke"
        } $false
        $Token = [string]$login.token
        $script:Token = $Token
        Write-Step "login" "PASS" ("user={0}" -f $login.user.account)
    } catch {
        Write-Step "login" "FAIL" $_.Exception.Message
        exit 1
    }
} else {
    $script:Token = $Token
}

if (-not $script:Token) {
    Write-Step "authenticated checks" "SKIP" "pass -Token, or pass -Account and -Password"
    Write-Host ""
    Write-Host "Example:"
    Write-Host "  .\scripts\test-yeyun-pc-web.ps1 -Token '<token>' -ProjectId '<project_id>' -NodeHint '<node_id_or_name>'"
    exit 0
}

try {
    $me = Invoke-ElonJson "GET" "/api/me"
    Write-Step "auth token" "PASS" ("user={0}" -f $me.user.account)
} catch {
    Write-Step "auth token" "FAIL" $_.Exception.Message
    exit 1
}

$myNodes = $null
try {
    $myNodes = Invoke-ElonJson "GET" "/api/me/nodes"
    $count = @($myNodes.nodes).Count
    Write-Step "my nodes" "PASS" ("count={0}" -f $count)
    $matchedMyNode = Find-Node $myNodes.nodes $NodeHint
    if ($matchedMyNode) {
        Write-Step "matched own node" "PASS" ("{0} online={1} clis={2}" -f $matchedMyNode.display_name, $matchedMyNode.online, (@($matchedMyNode.allowed_clis) -join ","))
    } elseif ($NodeHint) {
        Write-Step "matched own node" "WARN" ("no own node matched hint '{0}'" -f $NodeHint)
    }
} catch {
    Write-Step "my nodes" "FAIL" $_.Exception.Message
}

try {
    $allNodes = Invoke-ElonJson "GET" "/api/nodes"
    $count = @($allNodes.nodes).Count
    Write-Step "discoverable online nodes" "PASS" ("count={0}" -f $count)
    $matchedPublicNode = Find-Node $allNodes.nodes $NodeHint
    if ($matchedPublicNode) {
        Write-Step "matched discoverable node" "PASS" ("{0} online={1} routeA={2} apiRuntime={3} serverRuntime={4} clis={5}" -f $matchedPublicNode.display_name, $matchedPublicNode.online, $matchedPublicNode.route_a_ready, $matchedPublicNode.api_runtime_ready, $matchedPublicNode.server_runtime_ready, (@($matchedPublicNode.allowed_clis) -join ","))
    } elseif ($NodeHint) {
        Write-Step "matched discoverable node" "WARN" ("no discoverable node matched hint '{0}'" -f $NodeHint)
    }
} catch {
    Write-Step "discoverable online nodes" "FAIL" $_.Exception.Message
}

try {
    $projects = Invoke-ElonJson "GET" "/api/me/projects"
    Write-Step "projects" "PASS" ("count={0}" -f @($projects.projects).Count)
    if (-not $ProjectId -and @($projects.projects).Count -eq 1) {
        $ProjectId = [string]$projects.projects[0].id
        Write-Step "project auto-select" "INFO" $ProjectId
    }
} catch {
    Write-Step "projects" "FAIL" $_.Exception.Message
}

if ($ProjectId) {
    $space = $null
    try {
        $space = Invoke-ElonJson "GET" ("/api/projects/{0}/space" -f [uri]::EscapeDataString($ProjectId))
        Write-Step "project space" "PASS" ("channels={0} members={1}" -f @($space.channels).Count, @($space.members).Count)
        if (-not $ChannelId) {
            $devChannel = @($space.channels) | Where-Object { $_.kind -eq "ai_development" } | Select-Object -First 1
            if ($devChannel) {
                $ChannelId = [string]$devChannel.id
                Write-Step "ai development channel" "INFO" ("{0} {1}" -f $ChannelId, $devChannel.name)
            }
        }
    } catch {
        Write-Step "project space" "FAIL" $_.Exception.Message
    }

    try {
        $healthPath = "/api/projects/{0}/workspace/health" -f [uri]::EscapeDataString($ProjectId)
        $health = Invoke-ElonJson "GET" $healthPath
        Write-Step "workspace health" "PASS" ("node={0} online={1} canRun={2} cliReady={3}" -f $health.node_id, $health.node_online, $health.can_run_on_pc, $health.cli_ready)
    } catch {
        Write-Step "workspace health" "FAIL" $_.Exception.Message
    }

    try {
        $nodesPath = "/api/projects/{0}/ai/available-nodes" -f [uri]::EscapeDataString($ProjectId)
        $available = Invoke-ElonJson "GET" $nodesPath
        Write-Step "project available nodes" "PASS" ("count={0} canAuthorize={1}" -f @($available.nodes).Count, $available.can_authorize_nodes)
        $matchedProjectNode = Find-Node $available.nodes $NodeHint
        if ($matchedProjectNode) {
            Write-Step "matched project node" "PASS" ("{0} online={1} authorized={2} clis={3}" -f $matchedProjectNode.display_name, $matchedProjectNode.online, $matchedProjectNode.authorized, (@($matchedProjectNode.allowed_clis) -join ","))
        } elseif ($NodeHint) {
            Write-Step "matched project node" "WARN" ("no project node matched hint '{0}'" -f $NodeHint)
        }
    } catch {
        Write-Step "project available nodes" "FAIL" $_.Exception.Message
    }

    if ($RunProjectProbe) {
        if (-not $ChannelId) {
            Write-Step "project probe" "FAIL" "ChannelId is required and no ai_development channel was found"
        } else {
            try {
                $taskPath = "/api/projects/{0}/channels/{1}/ai-tasks" -f [uri]::EscapeDataString($ProjectId), [uri]::EscapeDataString($ChannelId)
                $probe = Invoke-ElonJson "POST" $taskPath @{
                    content = "Smoke test only. Reply with YEYUN_PROJECT_OK and do not modify files."
                    agent = "codex"
                    runtimeRoute = $RuntimeRoute
                    conversation_id = ("pc-web-yeyun-smoke-" + [guid]::NewGuid().ToString("N"))
                    conversation_title = "YeYun node smoke test"
                }
                Write-Step "project AI task probe" "PASS" ("task={0} conversation={1} route={2}" -f $probe.task_id, $probe.conversation_id, $RuntimeRoute)
            } catch {
                Write-Step "project AI task probe" "FAIL" $_.Exception.Message
            }
        }
    }
}

if ($RunChatProbe) {
    try {
        $chat = Invoke-ElonJson "POST" "/api/llm/chat" @{
            messages = @(@{ role = "user"; content = "Reply exactly YEYUN_AI_OK." })
            runtimeRoute = "route_c"
            conversation_id = ("pc-web-yeyun-chat-" + [guid]::NewGuid().ToString("N"))
            scope = "chat_memory"
        }
        $reply = [string]$chat.reply
        if (-not $reply) { $reply = [string]$chat.content }
        if ($reply.Contains("YEYUN_AI_OK")) {
            Write-Step "AI chat probe" "PASS" $reply
        } else {
            Write-Step "AI chat probe" "WARN" ("unexpected reply: {0}" -f $reply)
        }
    } catch {
        Write-Step "AI chat probe" "FAIL" $_.Exception.Message
    }
}

Write-Host ""
Write-Host "PC web URL: $($BaseUrl.TrimEnd('/'))/pc"
Write-Host "Done."
