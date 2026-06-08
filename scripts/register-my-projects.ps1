#!/usr/bin/env pwsh
<#
.SYNOPSIS
    把 PC 上的本地项目注册到 elon 项目广场（一次性脚本，注册后 APK 即可看到）。

.DESCRIPTION
    1. 用你的账号密码登录，换取 token
    2. 向服务器注册各本地项目路径
    3. 服务器若检测不到路径（项目在 PC 上），会尝试找在线的 PC 节点接管
       如果 PC 节点不在线，会先把项目注册为"服务器本机路径失败"之外的情况。
       所以推荐：先启动 elon-node-agent，再跑本脚本。

.EXAMPLE
    .\scripts\register-my-projects.ps1
    # 按提示输入账号和密码
#>
param(
    [string]$Account = "",
    [string]$BaseUrl = "http://43.139.149.158:8080"
)

$ErrorActionPreference = 'Stop'

# ── 1. 登录 ──────────────────────────────────────────────────────────────────
if (-not $Account) {
    $Account = Read-Host "elon 账号（手机号/邮箱）"
}
$SecurePassword = Read-Host "elon 密码" -AsSecureString
$Password = [System.Runtime.InteropServices.Marshal]::PtrToStringAuto(
    [System.Runtime.InteropServices.Marshal]::SecureStringToBSTR($SecurePassword))

Write-Host "正在登录..." -ForegroundColor Cyan
$loginResp = Invoke-RestMethod -Method Post -Uri "$BaseUrl/api/auth/login" `
    -ContentType "application/json" -NoProxy `
    -Body (ConvertTo-Json @{ account = $Account; password = $Password; device_name = "register-script" })

$token = $loginResp.token
if (-not $token) { Write-Error "登录失败：响应中无 token"; return }
Write-Host "✅ 登录成功，user_id: $($loginResp.user.id)" -ForegroundColor Green

# ── 2. 要注册的项目列表 ──────────────────────────────────────────────────────
# 修改这里来添加或移除项目
$projects = @(
    @{ id = "elon-self"; name = "一龙项目"; path = "D:\rust\active-projects\elon cli"; desc = "一龙项目主仓库" }
    @{ name = "bb64a";       path = "D:\rust\active-projects\bb64a";       desc = "bb64a 项目" }
    @{ name = "fb2";         path = "D:\rust\active-projects\fb2";          desc = "fb2 项目" }
    @{ name = "江西吉安商会"; path = "D:\rust\active-projects\江西吉安商会"; desc = "江西吉安商会项目" }
)

# ── 3. 批量注册 ──────────────────────────────────────────────────────────────
foreach ($proj in $projects) {
    Write-Host "`n正在注册: $($proj.name)  ($($proj.path))" -ForegroundColor Cyan
    try {
        $body = @{
            name           = $proj.name
            workspace_path = $proj.path
            description    = $proj.desc
            is_public      = $true         # 发布到项目广场
            join_mode      = "approval"    # 申请加入需审批
        }
        if ($proj.id) {
            $body.project_id = $proj.id
        }
        $r = Invoke-RestMethod -Method Post -Uri "$BaseUrl/api/projects/external" `
            -Headers @{ Authorization = "Bearer $token" } `
            -ContentType "application/json" -NoProxy `
            -Body (ConvertTo-Json $body)

        $flag = if ($r.reused_existing) { "(复用已有)" } else { "(新建)" }
        Write-Host "  ✅ $($proj.name) $flag  id=$($r.project.id)  node=$($r.node_id)" -ForegroundColor Green
    } catch {
        $status = [int]$_.Exception.Response.StatusCode
        $errBody = $_.ErrorDetails.Message
        Write-Host "  ❌ $($proj.name)  HTTP $status : $errBody" -ForegroundColor Red
        if ($status -eq 400 -and $errBody -match "没有在线 PC CLI 节点") {
            Write-Host "  ⚠️  提示：服务器找不到在线的 PC 节点来接管本地路径。" -ForegroundColor Yellow
            Write-Host "     解决方案：先下载并启动 elon-node-agent.exe，登录后再重跑本脚本。" -ForegroundColor Yellow
            Write-Host "     下载地址: http://43.139.149.158:8080/api/node-agent/download/windows" -ForegroundColor Yellow
        }
    }
}

Write-Host "`n完成！打开 APK『项目广场』应能看到刚注册的项目。" -ForegroundColor Green
