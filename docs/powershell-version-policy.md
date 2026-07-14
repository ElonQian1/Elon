# PowerShell 版本策略

本项目同时支持两类 Windows 入口：

- `powershell.exe`：Windows PowerShell 5.1，系统自带，负责 bootstrap、git/worktree 基线、发布脚本等保守入口。
- `pwsh`：PowerShell 7+，负责较新的验证、fb2 AI Center、复杂 JSON/并发/跨平台脚本。

## 硬规则

1. 脚本头部有 `#requires -Version 7.0` 时，必须用 `pwsh` 运行。
2. PowerShell 5.1 报 `#requires` 版本不匹配时，不允许删除 `#requires`，不允许把脚本语法降级，不允许复制一份同名低版本脚本替代。
3. 没有 PowerShell 7 的设备，只能运行没有 `#requires -Version 7.0` 的脚本，或转到安装了 `pwsh` 的机器、服务器、WSL/Linux 对应脚本执行。
4. 如果确实需要给 PowerShell 5.1 设备提供入口，只能写很薄的 wrapper：检查 `pwsh` 是否存在，存在则转调原 PS7 脚本，不存在则打印安装指引并失败。不要复制 PS7 脚本业务逻辑。
5. 真正需要 PS5 原生实现时，必须新建明确命名的脚本，例如 `*-winps.ps1` 或 `*-ps5.ps1`，并补单独测试；不得改坏原 PS7 脚本。

## 安装 PowerShell 7

推荐：

```powershell
winget install --id Microsoft.PowerShell --source winget
```

验证：

```powershell
pwsh -NoProfile -Command '$PSVersionTable.PSVersion'
```

也可以先用系统 PowerShell 5.1 检查本机是否已安装：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-pwsh7.ps1
```

## 运行命令选择

PowerShell 5.1 可用：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ai-task-preflight.ps1 -CreateWorktree
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\finish-ai-task.ps1 -Kind CodePushed
```

PowerShell 7 必须用：

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\validate-fb2-ai-center-current-state.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\fb2-ai-center-refresh-current-status.ps1
pwsh -NoProfile -ExecutionPolicy Bypass -File scripts\smoke-fb2-final-acceptance.ps1 -DataOnlyAcceptance -PreflightOnly
```

## 给 AI 代理的处理方式

如果命令失败并提示脚本要求 PowerShell 7：

1. 先检查 `pwsh` 是否存在：`powershell -NoProfile -ExecutionPolicy Bypass -File scripts\check-pwsh7.ps1`。
2. 若存在，原命令改用 `pwsh -NoProfile -ExecutionPolicy Bypass -File ...` 重新运行。
3. 若不存在，停止该 PS7 脚本任务，说明需要安装 PowerShell 7 或转到有 `pwsh` 的环境。
4. 不要为了让 PowerShell 5.1 跑通而修改 PS7 脚本。

