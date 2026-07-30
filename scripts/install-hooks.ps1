# 启用本仓库的 Git 守门 hooks（pre-commit 文档模块化 + pre-push 质量门禁）。
# 任何 PC（包括服务器 codex CLI）clone 后必须执行一次。
$RepoRoot = git rev-parse --show-toplevel
if (-not $RepoRoot) { Write-Error "❌ 当前目录不在 git 仓库内"; exit 1 }
Set-Location $RepoRoot

git config core.hooksPath .githooks
if ($LASTEXITCODE -ne 0) { Write-Error "❌ git config 失败"; exit 1 }

# Windows 上 git 自动给 hook 加可执行位（依靠 sh shebang 即可），但 WSL/Linux 需要 +x
if ($IsLinux -or $IsMacOS) {
    chmod +x .githooks/pre-commit 2>$null
    chmod +x .githooks/pre-push 2>$null
}

Write-Host "✅ 已启用 .githooks/ 作为 hook 目录" -ForegroundColor Green
Write-Host "   pre-commit：阻止正式文档继续变成巨型文件" -ForegroundColor Gray
Write-Host "   pre-push：重复文档/源码门禁，并保留可选 Rust receipt" -ForegroundColor Gray
git config --get core.hooksPath
