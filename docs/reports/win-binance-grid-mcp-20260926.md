---
version_status: current
reviewed_at: 2026-09-26
implementation_status: implemented
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
- `server/tests/browser-research-harness` 真实合同/队列模块测试通过；新增回执与已认领命令绑定的集成用例。
- PC 生产构建、TypeScript 校验、定向 ESLint 通过。首次 ESLint 无输出被日志包装器拒绝，改用 JSON 输出后成功。

## 交付与现场验收

发布和真实账户 MCP 读取尚待执行，不能由离线测试推断完成。需匹配节点、原生 Win 和在线 PC 前端，
并记录实际版本、采集时间、列表/精确详情成功和失败恢复行为。验收证据只记结构和结论，不提交账户数据。

本功能不代表 ChatGPT Rspack 兼容、自动发送或 APK 新入口已经完成；这些保持独立验收。
