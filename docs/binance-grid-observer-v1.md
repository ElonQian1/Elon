---
version_status: current
reviewed_at: 2026-09-07
---

# 币安网格被动观察工具 V1

## 目标

复用用户已登录的 Chrome 标签页，以独立、可检查的 Manifest V3 扩展观察用户在币安官网操作后产生的网格请求。先在本地提取有界、脱敏、去重的接口结构，再交给 AI 研究，以减少整页 DOM、源码和重复响应消耗。研究以用户本人授权会话为范围，不扩展已有交易权限。

## 范围与合同

- 仅用户点击扩展启用后，在 `https://www.binance.com` 的合约或合约网格顶层页面观察 fetch/XHR；不刷新、关闭、注销或复制既有登录会话。
- 权限限于 `activeTab`、`scripting`、`storage`；无持久 host、debugger、cookies、下载或网络重放权限。不需要 F12。
- 只观察实际发出的网格/策略候选请求。地址采用白名单静态段模板，去除查询与片段；HTTP 方法、状态与 JSON 字段类型用于建立候选合同，未观察到的端点不能当作真实 API。
- 请求体仅对现成 JSON 字符串提取结构，不读取 Request 流、FormData、Blob 或上传文件；响应仅读取 clone，设字节、时间、深度、字段和样本上限。字段采用白名单，未知字段归并，所有标量值丢弃。
- 请求头、响应头、Cookie、凭据、账号、订单标识和个人持仓值不进入扩展消息、存储、导出、Git 或 AI 上下文。代码可读取响应内容类型用于选择解析，但不记录或导出任何头。
- 已脱敏观察暂存于浏览器会话存储，默认不写磁盘；用户手动导出摘要。导出标记为不可信页面观察，不能授予执行、证明成功交易或账户归属。
- 捕获按标签页、文档和采集代次绑定。停止或新代次后丢弃旧响应；页面导航、采集期限到期停止。限制并发、事件速率、样本数和存储大小，去重并记录计数。
- 保持原 fetch/XHR 调用参数、接收者、返回对象和异常语义；观察错误不能影响官网。工具不发起、重放、修改或终止任何交易。

## 验收

1. 无需构建 Rust/Win/APK 即可生成可加载的扩展目录和版本工件；入口提供启用、停止、状态、清除摘要及导出操作。
2. 离线测试覆盖敏感字段/动态路径脱敏、结构与字节上限、去重、fetch/XHR 透明性、停止后迟到响应、采集代次隔离和不可信消息过滤。
3. 文档明确安装方法、权限、保留登录页的方法及覆盖限制：不会补捕开启前请求；已保存的 fetch 引用、Worker、WebSocket 或页面其他传输可能不可见；零样本不代表零网格。
4. 真实 Chrome 加载、真实币安网格列表请求及摘要导出单独记为人工现场验收；无证据时保持待验收。工具完成不等于币安网格业务接口已接通。

## 实现边界与依赖

实现位于 `tools/binance-grid-observer/`；定向测试位于 `scripts/test-binance-grid-observer*`，依赖本机 Node 和支持文档标识的 Chrome 106+。不修改 Win provider、ChatGPT/Google 适配器、量化业务执行器或 APK 发布入口；无远程代码或第三方包。

后续以真实摘要确认查询、创建准备、修改准备、结束后的状态模型，再进入量化项目版本化合同。所有实际交易由用户在官方页面完成，本工具只辅助观察；账户资产公开范围沿用总估值与更新时间的既定合同。

## 依据

- [Chrome scripting API](https://developer.chrome.com/docs/extensions/reference/api/scripting)：activeTab、MAIN/ISOLATED 注入与文档标识。
- [activeTab 权限](https://developer.chrome.com/docs/extensions/develop/concepts/activeTab)：用户操作触发的临时访问。
- [现有 Win 浏览器集成](user-browser-module-integration.md)：复用本地过滤方法，独立交付币安研究工具。
