---
version_status: current
reviewed_at: 2026-09-26
---

# APK 网格读取与聊天交付记录

本批基于 `4cee21ea0`，复用手机 BinanceHostRuntime、身份验证和固定只读适配器。
保留 Win 已发布能力、量化宿主合同及现有 ChatGPT 文本/文件发送器。

## 实现与离线验证

- 原生 MCP `binance_grid_read` 返回实际允许字段，显式开始、稳定分页、五分钟过期；
  详情先取新列表确认精确策略，账号/文档/主账号变化失败关闭。
- 新服务不调用交易准备/提交、不输出凭据或账号，不用 Win 数据填补手机结果。
- 原生 ChatGPT “+ → 附带网格”读取、选择、预览并插入可见草稿；发送只验证，
  不刷新或重复附加。原生 `binance_grid_attachment` 支持结构化状态、打开、移除。
- 供应商无关 stdio 增加 APK 列表/详情工具；注册包包含新依赖并保存到持久内容寻址目录。
- Node 最终 67 项检查通过：MCP 传输、原会话功能、资产注册及币安固定接口回归，
  包含真机发现的休眠/无线断连错误归一化，无凭据回显或自动重放。
- Android 定向 28 项单元测试通过：请求/快照 8、草稿 5、原宿主状态 15。
  初次编译发现跨文件私有 schema helper 引用，已修复并通过后续编译。
- Android 最终定向验证日志：`apk-grid-native-final-20260926-151443-827`，passed。
- Node 最终日志：`apk-grid-final-node`；日志保存在主仓库 Git 元数据的 `ai-command-logs`。

## 设备范围

按主项目设备登记和实际硬件身份探测：小米 23116PN5BC 无线 ADB 在线，荣耀 AAK-AN00
离线。安装前基线小米为 `1.1.1816 (1816)`。

正式包 `1.1.1817 (1817)` 已发布并在小米保留数据更新；安装后回读版本一致。
源提交：`1a46207d090b0cf068b766fe7c904ebcc46f5175`。
APK SHA-256：`a2f18e7f89542175473f6418bb2faa012163cd409bfdaecfc616ef6842b21b01`。
下载来源：`http://43.139.149.158:8080/app/ElonAI-latest.apk`（可变 latest 地址，需核对版本）。
发布日志：`apk-grid-release-20260926-152304-993`，passed，正式资源/manifest 校验通过。
设备收据：`com.elon.app-1817-87577c233ace4da6b3bcc0944f4987c5.json`，
登记的两台设备中小米 updated、荣耀 offline，安装失败数为 0；整体安装状态 `partial_offline`。

现有宿主实际识别手机为已验证子账户，列表为空。用户明确选择保留手机当前账户，
不切换为 Win 网格所在账户；不创建测试交易，不用假数据冒充真实非空网格。

## 当前真机结构化证据

以下均来自同一正式 APK 和已核对硬件身份的小米手机，风险级别为 safe：

| 入口/测试 | 期望 | 实际 | 恢复 |
|---|---|---|---|
| 原生 `binance_grid_read` 冷启动 | 本机会话返回新列表 | pending → ready，source=android_webview，total=0，trading_enabled=false | 无手动打开币安页面 |
| 持久注册的 stdio MCP | 从安装包读取手机列表 | tools/list 为原有 4 工具加列表/详情共 6 个；ready，total=0，diagnostics_bytes=0 | 重复轮询时间和条目不变 |
| `binance_grid_detail` 错误策略 | 先验证归属，拒绝详情 | failed，strategy_not_found | 未触发交易或换用 Win 数据 |
| 未启动的原生 request_id | 不隐式请求网站 | failed，snapshot_not_found | 没有创建新快照 |
| start 使用错误类型 | 拒绝输入 | isError=true，invalid_request | 不开始读取 |
| `binance_grid_attachment open/status/remove` | 与按钮共用原生控制器 | idle → reading_list → empty → idle | active=true，draft_present=false，auto_refresh=false |
| `binance_host_status` | 保留当前账户 | main_session_current=true，adapter_bound=true，list_verified=true，account_kind=sub，row_count=0 | 未切换账户 |

原生按钮的稳定 ID 是 `attachment-action-binance-grid`；本轮通过同一控制器的 MCP
open/remove 路径验收，未使用坐标点击。返回后保留原生 ChatGPT 聊天，无草稿、无消息发送。

无线 ADB 曾显示 device 但 shell/MCP 超时，同时观测到手机 Dozing。
按登记目标断开重连、重新核对硬件身份、恢复 forward 并发送 WAKEUP 后，原读取快照及
后续原生入口恢复。没有修改 VPN、省电策略、锁屏设置或清除应用/网站数据。
这是本次可复现的设备可达性限制，不能据此宣称锁屏休眠时永久在线。
适配器已将这类传输异常归一为 `apk_transport_unavailable` 并给出身份核验/重连提示；
不会把异常详情或凭据交给 AI，也不会自动重放已开始的操作。

Codex 持久注册已经刷新，并以实际注册路径运行了上述 stdio 验收。当前客户端工具发现仍
需要重载工具或新建任务。Claude Desktop 本轮未改配置、未做客户端发现验收；共享协议
可复用，不将协议测试等同于 Claude 已连接。

## 交付矩阵

所有行 `code_gap` 为空；未执行的真实场景在 `verification_gap` 单列保留。

| 能力 | code_status | verification_status | verification_gap |
|---|---|---|---|
| 冷启动实际空列表、MCP 返回及稳定轮询 | implemented | device_verified | 无 |
| 不存在策略拒绝、参数验证 | implemented | device_verified | 无 |
| 原生手动入口空列表及移除恢复 | implemented | device_verified | 无 |
| 非空列表多页/精确详情、字段与隔离 | implemented | offline_verified | 当前账户无网格，真实非空详情和多页待验证 |
| 非空选择、预览、加入和过期/变更发送门禁 | implemented | offline_verified | 草稿合同已测试，真实有网格界面未验收 |
| 当前 ChatGPT 发送器收到网格并产生回复 | implemented | deferred | 用户保留空账户，真实网格发送/回复未验收 |
| 荣耀设备安装与验收 | implemented | deferred | 登记设备当前离线 |

功能登记保留在 implemented，不由编译、发布或空列表通过推导整个功能已 verified/released。
后续仅需在用户希望使用且确有网格的手机账户上补真实详情、预览和一次授权的发送/回复验收；
不要为了补验证而切换当前账户或创建测试交易。
