---
version_status: current
reviewed_at: 2026-09-26
---

# Win 会话读取无人值守验收

用户授权 Codex、Copilot 或 Claude 更新一龙 Win 并测试指定 ChatGPT 会话后，使用正式入口
`scripts/win-conversation-acceptance/run.mjs` 完成整条链路，不再临时拼接更新脚本。
它复用 [Win 语义控制](win-codex-control.md) 与
[个人会话 MCP](personal-web-conversation-reader.md)，由当前代理直接执行。

## 多代理发现入口

Codex 从 `AGENTS.md`/`CODEX.md` 进入共享 `.github/copilot-instructions.md`；Copilot 读取该共享规则；
Claude 从仓库根 `CLAUDE.md` 进入同一共享规则。三者均按本说明执行，不依赖某一个模型的个人记忆。
已有任务授权涵盖的启动、精确更新、重连、导航和只读验收直接执行，不因每一步需要开窗口或重启而重复询问。
默认入口测试指定 ChatGPT 会话；`--update-only` 只更新 Win，不要求官网登录、不读取或发送聊天。
其他 Win 功能按各自合同验收，不能用版本检查冒充功能验收。
执行需要本机命令能力、Node.js 和现有 Win 控制通道；只读聊天模式或没有这些工具时明确报告缺口。
底层 `win_control` 的 `codex_mcp` 是现有固定控制来源标记，入口复用该通道，不伪造模型身份或新增绕过路径。
入口更新适用于读取最新项目规则的任务；旧会话需要重新读取，其他项目不会自动继承。

## 一次调用

需要 Windows、Node.js 18+、已安装一龙、有效的 ChatGPT 登录以及可达的官网网络。
发布目标取自正式发布收据，必须是 `version+完整 Git SHA`，不能传 `latest`、下载地址或程序路径。
只有节点和新鲜桌面进程心跳都匹配目标时才跳过更新，不能用磁盘清单或节点版本代表桌面版本。

### 只更新 Win，不操作聊天

已本机编译的包直接复用正式激活队列及 SHA-256 校验、owner 门禁、安装锁和回滚，
不需要等远端上传，也不手工复制正在运行的 EXE。首次升级旧守护器时仍可能走旧下载路径；
本版以后从安装身份绑定的数据根定位本地包，不再只查旧的 LOCALAPPDATA 缓存目录。

```powershell
# 本机编译、安排远端异步发布，并通过 MCP 自动更新/重启/验收本机 Win。
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/publish-node-agent.ps1 -UpdateRunningDesktop

# 已有发布包时只做更新，不重复编译。不导航、不刷新页面、不发聊天。
node scripts/win-conversation-acceptance/run.mjs `
  --project-root 'D:\path\to\project' `
  --target-release '正式发布收据中的version+完整GitSHA' --update-only
```

未指定 `-UpdateRunningDesktop` 的普通发布仍不打断用户桌面，会明确输出
`NODE_AGENT_DESKTOP_RUNTIME_STATUS=not_verified` 和可执行的更新命令；不能据此说客户端已更新。
显式更新必须收到 `NODE_AGENT_DESKTOP_RUNTIME_STATUS=verified` 或 update-only 的 `passed` 收据。
收据保留节点/桌面完整 SHA 和实际桌面 PID；缺失、混合版本、过期心跳都不能过关。
重复执行已匹配版本时不重启。若回复丢失，用相同参数和 `--resume <run_id>` 只核实原动作，
不再提交第二次更新。超时保留检查点并返回非零，不能自动强杀任务或假报成功。

```powershell
node scripts/win-conversation-acceptance/run.mjs `
  --project-root 'D:\path\to\project' `
  --reference 'chatgpt-conversation://明确授权的UUID' `
  --target-release '正式发布收据中的version+完整GitSHA'
```

仓库内较长的现场验收调用遵循现有 `invoke-ai-logged-command.ps1` 有界日志入口。
退出码 `2` 表示真实内容仍有缺口，日志包装器会将它显示为非零结束；不能为了显示绿色而隐藏。

## 自动执行顺序

1. 发现真正的本机节点；缺少节点或工作台时调用固定安装启动器一次，等待两者上线。
2. 建立项目绑定的 `win_control` MCP 会话，读取当前真实发布身份。
3. 版本不同时先保存更新意图，再提交一次 `update_and_restart`；读取精确动作回执。
4. 等待节点重连，重新申请短期 MCP 会话，核对 `capabilities.release_identity`、
   `capabilities.desktop_runtime.release_identity` 均等于目标，且 Tauri/前端在线。
   “更新已安排”不能代替“新版本已激活”。
   `--update-only` 在此再次确认后结束，不执行后面的聊天操作。
5. 执行固定 `reload_page`、`navigate /ai`、`capture_state`，逐项等待成功回执并核对导航路由。
6. 准备内容寻址的正式 MCP 代码副本并启动新 stdio 进程，只授权本次明确指定的会话。
   验证四个读取工具真实可发现；不修改全局 Codex 配置。
7. 读取所有正文页，并立即读取每个附件句柄，验证返回的真实 MCP 图片/文件字节、大小和摘要。
   所有游标固定在同一 Win 宿主和快照，不使用 APK 回退。
8. 再次核对实际运行版本，保存正文字符数/摘要、附件数量/摘要和剩余格式缺口。

WebView 只承担已有本机登录身份和私有请求执行；此流程不依靠聊天气泡截图或正文 DOM 抓取。
实际模型是否理解图片、PDF 或文件仍是模型客户端的独立能力。

## 恢复与并发

每步检查点保存在 `%LOCALAPPDATA%\Elon\win-conversation-acceptance-v1\<run_id>.json`。
只包含阶段、发布身份、绑定摘要、loopback 端点、动作 ID、数量和内容摘要；不保存聊天正文、
会话 UUID、标题、文件名、附件句柄、游标、Cookie、token 或官网下载地址。
每次写入先刷新临时文件，再原子替换检查点。

同机同一时刻只运行一个验收流程。进程退出释放锁；崩溃后只有确认原 PID 已不存在才回收锁，
PID 仍存在、锁损坏或恢复竞争均明确失败，不能只凭锁的年龄重启。

中断后用原参数加 `--resume <run_id>`。项目、授权会话和目标发布身份必须与原绑定一致。
已保存更新意图时先观察原端点并核对版本，不再提交更新，也不在更新守护器之上重开另一个工作台。
已验证版本漂移时明确失败，不自动降级。
读取阶段从第一页重新验证，旧正文、资产结果和游标均不复用。

启动等待最多一分钟；更新重连最多五分钟；单个语义动作最多45秒；单个 MCP 请求最多155秒；
正文与附件验收最多15分钟。现有更新守护器若正在等待不可中断任务，其工作继续由原守护器管理；
本入口记录超时并允许后来恢复，不强杀进程或绕过升级门禁。

## 结果合同

| 退出码 | status | 含义 |
|---:|---|---|
| 0 | `passed` | update-only：节点和桌面精确版本均验证；默认：另含功能打开、全部页和附件完整性 |
| 2 | `partial` | 已自动跑完，但源内容仍报告未支持格式等缺口 |
| 3 | `user_action_required` | 官网登录或验证必须由用户完成 |
| 1 | `failed` | 更新、连接、导航、读取、身份或摘要校验失败；按收据错误码定位 |

VPN/网络不可用时只报告连接或读取失败，不把空白页当成空会话。用户已有的更新与测试授权
无需逐步重复确认；新登录或官网验证仍须在官方页面完成。本流程不建立定时任务，也不恢复
已经暂停的跨项目任务派发、PC 自动续跑或后台自进化。

## 验证入口

`node --test scripts/win-conversation-acceptance/*.test.mjs` 覆盖冷启动、版本核对、令牌重建、
更新意图先落盘、回执丢失恢复、禁止重复更新、导航校验、分页、真实附件摘要与内容缺口。
当前实际设备证据见[无人值守现场报告](reports/win-conversation-unattended-20260925.md)；
离线故障测试不冒充现场更新重启。
