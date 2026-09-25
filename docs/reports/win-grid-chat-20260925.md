# Win 手动网格附件现场记录

## 当前结论

只在点击“附带网格”后读取，选择策略、预览并将一次性快照交给共用发送器已实现。
真实 Binance 列表和详情读取通过；ChatGPT 实际发送及回复未通过，整体未完成。
不记录私人策略 ID、金额、账号、Cookie、令牌、原始问题或响应。

## 已验证

- 生产 Win 读取到 32 个运行网格，同合约多个策略按方向、杠杆、格数和 ID 明确区分。
  用户指定的做空策略详情产生新序号、新时间，并显示采集时间和可展开预览。
- 关闭币安窗口时显示恢复入口；打开原登录窗口后可再次手动读取，不需要重新登录。
- adapter 211 和 212 各实际尝试一次。两次均未获得发送接受回执，问题和快照完整回到草稿；
  未自动重发，未进行任何交易操作。
- `74baa51534ead13b05a47e0ae8391fe5cf6c1e05` 已正式发布，并回读该精确 Win 版本、
  Tauri 和前端在线。该 SHA 的 `NodeAgent` 完成检查通过；这不是业务全链路验收通过。
- 11 项网格回归、真实 Win 适配器装配测试、97 项共享发送与身份等相关回归通过。
  PC 类型检查/生产构建、定向 ESLint、Tauri check 通过。

## 修复与剩余问题

Win 清单缺失听写、布局、发送、身份等共享依赖，已逐项补齐；普通与群组共用
`text_bootstrap`，身份模块先于捕获它的传输模块初始化。真实清单 VM 测试覆盖初始化
和缺依赖回归，避免单测人为注入依赖掩盖生产缺口。

adapter 212 实机 bootstrap 正常、输入框和会话绑定就绪，但 ChatGPT 发送报
`runtime_fallback:runtime_not_observed`。官网本人会话正文和登录可见；一龙投影仍为
账号未确认、目录及消息为零。未把它归因于 VPN，未清 Cookie，未绕过版本和身份校验。
独立研究 Profile 只能取得访客页面资源，不能冒充本人会话的运行版本证据。

后续补充复用当前 owner 的个人 ChatGPT 文档进行只读研究，保留生产 `evaluate` 禁用，
并把失败直接展示在输入框旁。兼容修复仍须检查当前实际脚本，再验证消息被接受、回复
回到一龙和同一会话历史；不能仅更换资源 URL 或猜测 minified 导出名称。

## 当前登录文档的运行时证据

`220e10975ebd7c2117fb876bcccc162eef7df365` 已正式发布、本机自动激活，精确版本、
Tauri 和前端在线回读通过，`NodeAgent` 发布检查通过。进入原生聊天后，原官网会话
和未发送测试草稿仍在。`site_id=chatgpt` 的只读研究回执为 `host_mode=ai_window`，
证据来自本人当前文档，不是独立访客 Profile。收集后已暂停，回读 `active=false`。

这份文档加载 `manifest-492fbfe6.js`，入口是 `908190.44b0fc59dd.js`，公开入口导出
`__rspack_esm_id`、`__webpack_modules__`，并从 `633146.03cad12214.js` 导入
`__webpack_require__`。现有 bindings 33 只识别旧 `c2675c8c-*` 等明确构建，无法把
新的模块注册表当作旧 ESM 命名导出读取。这解释 `runtime_not_observed`，不是登录失败
或当前 VPN 未连接。此次未尝试第三次发送，未执行未知模块或放宽版本/身份门控。

公开 CDN 原始文件单独无凭据下载核验；下列 SHA-256 不混用本机凭据过滤后的正文哈希。
所有文件位于 `https://chatgpt.com/cdn/assets/`：

| 文件 | 原始文件 SHA-256 |
|---|---|
| `manifest-492fbfe6.js` | `7a287c1ec724ca0d81094ec8556c90de937f206a06ab2e13807bd7fce9b95648` |
| `908190.44b0fc59dd.js` | `3970d87cb066da0a0e1af97955fd34b193ad05986b8658fea9c0bdabdfe2bcc1` |
| `633146.03cad12214.js` | `e1c8be6ea49317892798a70d98882449c009708fcd6698a49aaa38ef760c52e2` |

采集得到 115 份资源，其中 107 份 CDN 脚本；存在 `body_read_queue_limit`、
`script_index_limit`、`event_queue_full` 和仅顶层文本覆盖缺口，不代表全部源码、全部
请求或回复传输已采集。API origins 留空，请求正文计数为零。私人 HTML、策略和草稿
正文没有加入 Git。尚未完成新版运行时的身份、目录/历史、发送接受、流式回复和重复
发送对账适配，功能登记改为 `blocked`，整体仍未通过验收。

既有全量检查缺口：全量 PC lint 的 `GroupAssistantDialog.tsx` 未用 disable、
`test-local-ai-browser-contract.cjs` 旧认证文本断言；均非本批修改，不能称全量通过。
