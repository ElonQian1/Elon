---
version_status: current
reviewed_at: 2026-09-26
implementation_status: released
---

# Win MCP 币安网格读取验证

需求：[win-binance-grid-mcp-v1](../requirements/win-binance-grid-mcp-v1.md)。

## 实现

现有 `browser_research` 增加固定 `binance_grid_list`、`binance_grid_detail` 命令。
全局 Win 桥接器分发到独立读取服务，复用原网格附件的身份、观察时间、列表和详情校验，
再调用 Binance WebView 内已捕获有效上下文的固定只读私有 API。原生壳只增加后台打开能力声明和选项。
不使用桌面点击、任意 JS、调用方提供的 URL/请求头，不增加交易写入。

读取使用显式 start、pending 查询、不可变分页、5 分钟有效期、16 项容量限制；详情先核对本人最新列表。
前端和节点端分别验证固定字段、精确策略 ID、request_id、页码和有效期，结果不含账号或会话凭据。
旧宿主没有后台打开能力声明时明确拒绝，避免因忽略新参数而意外显示窗口。

## 离线证据

- 读取服务与研究桥接共 31 项 Node 测试通过，覆盖重复查询、分页、账号/文档变化、过期、容量、错误 ID、私密字段过滤和旧快照拒绝。
- 原“附带网格”11 项回归通过，仍只在点击时读取。
- 后台打开 3 项测试通过，覆盖原生 background 回执、不同 owner/显示意图隔离、旧宿主拒绝。
- `server/tests/browser-research-harness` 真实合同/队列模块 30 项测试通过，包含回执与已认领命令绑定的集成用例。
- PC 生产构建、TypeScript 校验、定向 ESLint 和 Win 原生 `cargo check` 通过。首次 ESLint 无输出被日志包装器拒绝，改用 JSON 输出后成功。

## 交付与现场验收

本次代码与已发布工件固定为 `8e2187d116bd9dfee8025fcb6eed9e117ccd419b`。
这份验收记录在完成发布后单独归档，不改变已验收业务源码。

| 对象 | 实际证据 | 结果 |
|---|---|---|
| 云端与在线 PC 前端 | `0.3.1778`，两者 SHA 均为上述代码提交；`/pc` HTTP 200、health OK | passed |
| 本机节点与桌面 | 运行身份均为 `0.3.69+8e2187d116bd9dfee8025fcb6eed9e117ccd419b`；7799 监听进程 PID 31096，桌面 PID 18060 | passed |
| 自动更新 | `a20806ca-5e35-49b2-bf98-9ec1550fb0f8` 回执 `passed / workflow_complete=true`；读取后再次通过 MCP 核对活进程身份 | passed |
| 新列表及分页 | 首次显式 start 返回新观察；完整分页后条目数、唯一 ID 数量与 total 相符，末页 next_offset=null；两页保持相同采集时间 | passed |
| 精确详情 | 按列表内所选策略 ID 返回同一策略的固定字段；未知项保持 null，十进制值保持字符串 | passed |
| 重复查询 | 相同 request_id 的详情再次查询，采集时间与明细均保持不变 | passed |
| 不存在的策略 | 返回 failed / strategy_not_found，不带 row，不返回其他策略 | passed |

上述读取全程使用本机 MCP 和 WebView 内固定私有 API，没有桌面点击。新列表、详情的观察时间分别为
`2026-09-26T05:56:54.542Z`、`2026-09-26T05:57:41.311Z`；不将账户 ID、策略 ID、实际参数、盈亏或凭据写入 Git。
正式 `PcFrontend`、`NodeAgent` 发布检查均通过；业务发布阶段统一收尾返回 `BUSINESS_STATUS=complete`、`FINALIZABLE=true`。

## 本机 AI 客户端接入

- Codex：通过官方 CLI 登记 `yilong_browser_research`，回读 enabled=true / stdio / browser_research；
  真实 stdio initialize、tools/list 握手成功。当前任务没有重新加载工具列表，新登记入口应在客户端重载后发现；本次业务读取已通过同一真实 MCP 协议完成。
- Claude 桌面：原 `yilong_web_conversations` 配置保留。新增研究通道的配置写入被自动审批拒绝（blocked by policy），
  命令没有执行；已向用户单独提出明确授权请求，尚未配置或声称 Claude 新通道已验收。

本功能不代表 ChatGPT Rspack 兼容、自动发送或 APK 新入口已经完成；这些保持独立验收。
