---
version_status: current
reviewed_at: 2026-09-07
---

# 币安网格观察工具 V1 交付记录

## 当前结果

独立 Chrome MV3 工具代码已实现：手动启用当前币安合约/合约网格页面，观察后续候选 fetch/XHR，将有界脱敏结构暂存在 Chrome 会话中，支持停止、清除及手动导出。版本为 `0.1.0`。真实 Chrome 加载、真实币安列表请求和导出尚未现场验收；未形成可执行币安端点合同，也未接通量化业务接口。

当前选 Chrome 是为了复用现成登录会话和独立交付。低 token 来自本地过滤、白名单字段类型、路径模板、采样和去重；Win WebView 可以采用同样方法，但现有 Win provider 只有 ChatGPT/Google，本轮不改共享 provider 或重新编译 Win/APK。

## 实现与证据

- 需求：`docs/binance-grid-observer-v1.md`；注册功能：`binance-grid-observer-v1`。
- 扩展：`tools/binance-grid-observer/`，包括 MAIN 观察器、ISOLATED 消息桥、worker、会话存储 reducer、共享脱敏器和中文弹窗。仅 `activeTab`、`scripting`、`storage`，没有 debugger、持久站点、Cookie 或远程服务权限。
- 包装：`scripts/package-binance-grid-observer.ps1 -OutputDirectory <新目录>`，只接受已提交源码，保留既有输出；生成 `extension/`、zip、SHA-256 和 `artifact.json`，记录源码提交及全部文件哈希。工件明确 `live_browser_verified=false`。
- 手册：`tools/binance-grid-observer/README.md`，先加载本地扩展，再在原登录页启用；首次仅由用户查看运行中/历史列表。不使用 F12，不刷新、关闭、注销或复制登录页。

2026-09-07 定向验证通过：`node --test scripts/test-binance-grid-observer-core.cjs scripts/test-binance-grid-observer-extension.cjs scripts/test-binance-grid-observer-popup.cjs`，37 项通过，0 失败、0 跳过。经 `invoke-ai-logged-command.ps1` 记录，日志标识 `binance-grid-observer-final-20260907-041721-386`；6 个 JS 语法检查及包装脚本 PowerShell 解析通过。

测试使用 Node 的 VM/浏览器接口桩与实际源码，覆盖：原 fetch promise/响应/接收者/异常、XHR 同步事件与异常、Request 流及 getter 不读取、停用和非候选请求不解析、真实页面路径匹配、相对路径、字节/时间/并发/结构上限、代次和导航后的迟到响应、跨文档与伪造弹窗消息拒绝、worker 失败回滚、摘要去重和上限、重启恢复、真实报告导出及 Blob URL 撤销。这些结果不是实际 Chrome 网络采集证据。

独立审查发现并修复了合约网格页匹配遗漏、停用后 XHR 请求体仍被解析、相对路径模板不准确、导出上限与存储不一致等问题。最后补充的兼容回归还确认注入不会覆盖页面自己的 CommonJS 导出。现有登录页未被本轮工具操作，真实交易、保证金、账号权限和量化部署均无变化。

## 下一步验收

用户手动加载工件并启用观察，在官网查看运行中/历史列表后停止，导出一份脱敏摘要。只核对候选路径、方法和响应结构，先证明读取链路；无法观察时记录覆盖缺口，不猜测接口地址、不生成重放请求。启用前请求、缓存、预先保存的传输函数、Worker、WebSocket、非 JSON/非文本 XHR 等不在完整覆盖范围。零样本不能判断为零网格。

MAIN 页面可以伪造消息；报告统一标记 `untrusted_page_observation`，不证明账户归属、交易成功或授予执行权限。停止或 15 分钟到期后不再解析请求；原请求始终交由官网处理。
