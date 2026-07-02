# 启用本仓库的 git 守门 hooks（pre-push: source-size guard + locked cargo-dev check + 拦截未 add 的 .rs）
# 任何 PC（包括服务器 codex CLI）clone 后必须执行一次。
$RepoRoot = git rev-parse --show-toplevel
if (-not $RepoRoot) { Write-Error "❌ 当前目录不在 git 仓库内"; exit 1 }
Set-Location $RepoRoot

git config core.hooksPath .githooks
if ($LASTEXITCODE -ne 0) { Write-Error "❌ git config 失败"; exit 1 }

# Windows 上 git 自动给 hook 加可执行位（依靠 sh shebang 即可），但 WSL/Linux 需要 +x
if ($IsLinux -or $IsMacOS) {
    chmod +x .githooks/pre-push 2>$null
}

Write-Host "✅ 已启用 .githooks/ 作为 hook 目录" -ForegroundColor Green
Write-Host "   pre-push 守门已激活：source-size guard + locked cargo-dev check + 拦截未 add 的 .rs" -ForegroundColor Gray
git config --get core.hooksPath
