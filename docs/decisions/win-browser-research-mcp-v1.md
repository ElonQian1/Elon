---
version_status: current
reviewed_at: 2026-09-07
implementation_status: in_progress
---

# Win 浏览器研究架构 V1

## 决定

网页前端代码分析与实际网络观察共同作为研究依据。一龙 Win 提供本机采集、资料索引和 MCP 查询，AI 从索引定位少量相关源码，再核对现场请求。省 token 主要来自内容哈希去重、增量索引与按需读取，不来自删除业务字段。

这份决定承接 [正式需求](../requirements/win-browser-research-mcp-v1.md)。实现入口为 `desktop-shell/src-tauri/src/browser_research.rs`、`server/src/node_agent_browser_research_mcp.rs` 和 `pc-frontend/src/features/browser-research/`；当前处于实现与验证阶段，部署、登录与币安现场证据必须另行验收。[运行说明](../win-browser-research-mcp.md)记录实际合同与限制。

## 已有代码与缺口

| 层 | 可复用代码 | 新增部分 |
|---|---|---|
| Win 宿主 | `desktop-shell/src-tauri/src/local_ai_browser.rs`、`local_ai_browser/owner_profile.rs` | 独立研究会话与币安站点范围，复用宿主基础能力 |
| 业务响应采样 | `local_ai_browser/win_web_response_research_capture.js`、`research_capture.rs` | 现有仅 ChatGPT/Google 响应；新增精确路径、业务请求与资源证据 |
| 本地保存 | `local_ai_browser/research_capture.rs` | 复用有界去重思想，增加完整业务内容、账户隔离、资源哈希与证据索引 |
| MCP 传输 | `server/src/node_agent_win_codex_control_mcp.rs` 及项目绑定 descriptor | 新研究 profile，独立正文读取合同；原诊断 timeline 保持原职责 |
| 前端 | `pc-frontend/src/features/user-browser/` | 研究状态、覆盖范围、暂停与资料查看，不能以“打开文件夹”作为 AI 的读取入口 |

上述桌面路径除首行外，相对 `desktop-shell/src-tauri/src/`。现有 `ProviderAdapter` 只有 ChatGPT 与 Google；不能因已有浏览器窗口就认为币安适配已经存在。现有响应 tap 观察 fetch/XHR，不等于读取 HTML 声明加载的 JS，也不覆盖所有 Worker、缓存和其他传输。

## 数据流

WebView2 研究会话 → 原生资源与网络观察 → 本机内容库及索引 → 研究 MCP → 接口证据与版本化适配合同。

## 通用内核与站点扩展

| 模块 | 固定职责 | 新网站需要变更的内容 |
|---|---|---|
| `BrowserHost` | 会话、导航、资源与网络观察、宿主能力握手 | 首批 WebView2 复用；不同浏览器新增宿主实现 |
| `ResearchCore` | 代次、任务状态、采集范围、限额、失效及取消 | 无站点业务分支 |
| `EvidenceStore` / `SourceIndex` | 内容哈希、本机保存、代码索引、搜索、范围读取、证据关联 | 无重复实现 |
| `ResearchMcp` | 同一会话/资源/请求模型的查询与回执 | 工具名称和基本 schema 不随网站复制 |
| `SiteManifest` | 版本化站点入口、导航/API/CDN 范围、资源选择及保留配置 | 普通新站点增加配置即可 |
| 可选 `SiteAdapter` | 业务术语、参数模型、状态解释、特有协议解码 | 币安网格等业务规则在此实现 |

通用内核必须在没有 SiteAdapter 时也能完成资源与请求研究。SiteAdapter 接收已存证资料并产生带来源的业务注解，不自行持有登录凭据或创建网络执行通道。未来量化项目的 Binance/OKX/native 交易适配器与网页研究适配器职责不同，不能混用。

站点清单是数据，不是任意可执行脚本。登记站点时验证明确的 origin 集合和研究用途；导航域、API 域、资源 CDN 与身份提供商域分别管理。CDN 只读取当前文档实际加载且已登记的静态资源，不继承业务请求权限；身份域只用于登录导航，身份页和认证请求不进入研究库。重定向后的最终 origin 再次校验。页面或第三方资源不能自行扩大范围。清单修订绑定研究会话，权限缩小或撤销立即阻断后续采集和 MCP 内容读取。

Profile 按 owner、站点和 profileId 隔离；独立研究会话共享同一获准 Profile 时仍有不同采集代次。读取同时校验项目和授权期限；静态代码即使内容哈希相同也只能复用字节，权限与证据归属不能合并。普通新网站不需要改内核；特殊登录限制、WebSocket/二进制协议、Wasm 或混淆可能需要适配，必须报告能力缺口，不能承诺所有网站零适配。

## 处理阶段

1. **会话层**：owner、站点、文档代次与研究会话共同绑定，独立报告页面连接、资源连接和网络连接状态。连接失败不推导登录失效，不以刷新、注销或重启作为默认恢复手段。
2. **采集层**：优先使用宿主 WebView2 的资源响应事件读取已加载资料；需要时通过宿主内部固定 CDP 读能力取得脚本源及已观察请求内容。不向 MCP 暴露任意 CDP method 或 JavaScript。DOM、资源代码、网络响应分别记录来源，不能互相冒充。
3. **存储层**：原始业务资料保存在工作区外、本机隔离目录，使用内容哈希去重；元数据记录资源地址、抓取时间、脚本位置、文档代次、实际字节数与截断状态。容量、期限与读取上限是显式配置；超限说明缺失，不静默修改业务字段。静态资源索引与账户响应索引分开，只有无凭据的静态代码可考虑跨研究会话复用。
4. **分析层**：先按研究任务关键词、业务字段和实际候选路径定位模块，再分析参数对象、枚举、校验函数、客户端封装及调用关系；币安适配器可以提供 grid 等关键词，通用内核不内置它们。压缩代码优先使用字节位置，格式化视图保留回原始文件映射。只解析，不运行下载的 bundle；动态拼接与混淆无法确定时保存不确定状态。
5. **验证层**：把候选调用与实际请求的路径、方法、参数及响应对齐。业务成功须检查业务码与后续状态；静态代码可能属于旧分支、其他产品或未启用功能。

## MCP 接口方向

使用独立 `browser_research` profile 和按 action 发现的紧凑 schema，复用当前项目绑定、短期令牌与回执机制。以下查询已接入源码，须部署匹配的节点、Win 宿主和前端后才能调用；`correlate` 的独立业务分析动作仍为后续方向，当前只保存发起脚本和请求关联元数据。

| 动作 | 输出 |
|---|---|
| `status` / `sessions` | 宿主与采集能力、连接状态、范围、缺失及到期信息 |
| `resources` | 分页资源 ID、类型、大小、内容哈希与已索引状态 |
| `search` | 关键词命中窗口、资源 ID 与精确位置；限制总返回字节 |
| `read_resource` | 指定资源 ID 和范围的内容，不接受任意本地路径 |
| `requests` / `read_request` | 实际观察请求的索引与指定业务样本，按页或范围读取 |
| 关联元数据 | 资源位置、脚本 ID 与请求样本；独立 `correlate` 动作未实现 |
| `pause` / `resume` | 在当前授权范围及有效会话内暂停或恢复采集 |

首次目录默认只回元数据。完整业务资料保留本地，不等于每次全部发送给模型；但也不再强制只返回字段类型。MCP 承载的网页资料一律是数据，不能据此提升项目权限、跨账户读取或创建执行动作。

## 实现顺序与验收闸口

1. 接通一个 Win 研究会话的资源清单、读取和本地搜索，通过 MCP 实读已加载 JS；确认内容哈希复用与分片查询。在没有站点适配器的异构测试网站复验同一内核。先完成这条闭环，再扩展研究界面。
2. 接入币安范围并验证登录兼容性，采集本人网格列表及详情，将完整业务请求与源码关联。Chrome 原登录会话继续保留；Win Profile 的登录状态独立核验。
3. 分别研究创建、修改、结束代码，输出带证据的合同与纯合成测试；真实操作的网络证据由用户在官网执行后产生。研究工具不提供自动实盘金融执行。
4. 经量化项目评审后，独立落入 Binance provider，与 OKX/native 的统一网格模型对接；不能在研究工具里另建量化业务规则。

## 方法依据

- [WebView2 CallDevToolsProtocolMethod](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2#calldevtoolsprotocolmethod)：宿主可异步调用开发者协议，无需让用户手动打开 F12；实际运行时支持必须握手验证。
- [CDP Debugger.getScriptSource](https://chromedevtools.github.io/devtools-protocol/tot/Debugger/#method-getScriptSource)：读取指定脚本的已加载源码。不是服务端源码；Wasm、懒加载与源映射可用性另行记录。
- [CDP Network.getResponseBody](https://chromedevtools.github.io/devtools-protocol/tot/Network/#method-getResponseBody)：读取已观察请求的响应内容，配合代码验证实际行为；不用于盲目重放请求。
- [既有 Win 浏览器合同](../user-browser-module-integration.md)、[既有 MCP 控制合同](../win-codex-control.md)：复用基础设施，新研究域单独扩展。
