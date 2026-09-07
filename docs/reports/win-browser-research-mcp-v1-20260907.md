---
version_status: current
reviewed_at: 2026-09-07
implementation_status: in_progress
---

# Win 浏览器研究 MCP V1 验证记录

后续 23:29 起的正式安装版取证见[币安运行合同研究](binance-grid-runtime-contract-20260907.md)：原项目登录 Profile 已读到列表、详情和页面采集的创建/修改/结束响应。下面“登录、业务回执待验证”是早期验收记录；后续研究成功仍不代表交易执行或 APK 同步已验收。

本报告记录实现证据，不能代替[需求](../requirements/win-browser-research-mcp-v1.md)。截至 2026-09-07，真实 Windows WebView2 已完成无交易样例的自动采集、MCP 搜索/正文读取和暂停恢复，并读出币安实际加载脚本里的 U 本位网格私有接口候选。本人币安首次登录及认证后的列表业务回执仍待验证，因此整体需求保持进行中；没有执行真实金融交易。

## 已验证

| 检查 | 结果 | 本机日志名称 |
|---|---|---|
| `elon-pc-node` 正式入口 check | 通过 | `browser-research-node-check-final-20260907-055710-734` |
| `elon-desktop` 研究模块测试 | 27/27，通过实际 Windows 编译 | `browser-research-desktop-tests-final-20260907-055554-355` |
| 宿主初始化修复后的强制 Windows 编译 / 研究测试 | 32/32，未复用旧验证结果 | `browser-research-native-handshake-forced-tests-20260907-063540-929` |
| 短 Profile、SPA 状态及有界突发队列回归 | 41/41，实际 Windows 编译 | `browser-research-live-fixes-tests-20260907-070715-724` |
| 桌面构建输入变更使缓存失效 | 36 项指纹断言通过 | `browser-research-validation-fingerprint-20260907-063842-676` |
| 独立 HTTP/MCP/队列 harness | 12/12 | `browser-research-harness-tests-final-20260907-055328-209` |
| 前端行为测试 | 17/17 | `browser-research-owner-error-tests-20260907-055059-147` |
| 初始化状态回执 / 前端回归 | 18/18，定向 lint 和生产构建通过 | `browser-research-handshake-ui-tests-20260907-063307-810`、`browser-research-handshake-ui-lint-20260907-063344-684`、`browser-research-handshake-ui-build-20260907-063321-127` |
| 前端完整 TypeScript / 生产构建 | 通过 | `browser-research-owner-error-build-20260907-055125-823` |
| 前端定向 lint | 通过 | `browser-research-owner-error-lint-20260907-055059-133` |
| 格式入口回归 | 通过，含 desktop 定向文件与既有默认范围 | `browser-research-format-workflow-final-20260907-054132-418` |
| 增量源码大小 / 文档模块化 | 通过，暂存文件也已复核 | 源码 49 项，正式文档 3 项 |

日志由仓库统一命令包装器写入 Git 元数据的 `ai-command-logs`；Rust 完整证据写入共享缓存 `validation-v1/evidence`，没有复制账户资料到仓库。

覆盖业务未知字段与精确路径、UTF-8 分片、响应长于请求的分页、源内容完整性、owner/project 隔离、清单撤销、文档代次、过期、Windows 路径边界、读取队列上限与旧回调、凭据定向排除、重复 JSON 键与深度、项目隔离队列、claim/receipt 幂等与失败、退出/换账号后的延迟回执失效。

## 已修复的问题及测试边界

第一次 desktop 测试为 22/23：Windows verbatim PathBuf 在调用前已归一化测试中的父路径片段，导致用例不能表达目标输入。改用保留原始片段的输入并增加检查后，所有回归通过；后续新增队列与 JSON 检查，总计 27 项。

最初节点 `--bin elon-pc-node` 单测编译遇到 303 条既有 cfg/可见性错误，主要在 sqlite VFS/managed_fs。非 test 正式入口仅有 11 条同根因错误：`shm.rs` 三处 cfg 属性直接修饰赋值表达式。仅将这三处包成相同条件下的语句块，正式入口 check 随后通过；未扩散修改历史测试可见性问题。

独立 `server/tests/browser-research-harness` 导入本轮真实 hub/contract/API/MCP 源码，只简化 NodeRuntime 与 McpRequest 外壳。其通过不代表全节点历史单测已通过，也不覆盖既有 loopback 鉴权中间件或官网登录。

## 现场验收状态

先前安装提交 `680b4d247603c8f36e3727a9848b83567e4fd99c` 已完成 PC、节点与 desktop 发布及本机激活，但现场仍报告 `host_native_not_attached`。其 Profile 路径长达 236 字符，WebView2 只生成启动标记，未建立正常资料目录；缩短为三个范围的完整 SHA-256 目录后，同一真实链路随即成功。旧目录未删除或搬移。

以下成功证据来自正式验证入口构建并直接运行的 Windows 开发候选；最终安装包发布/激活状态以统一发布和收尾回执为准，不能将开发候选等同于已经安装的版本。

| 项目 | 当前现场状态 |
|---|---|
| PC 前端、节点及 desktop 初次发布 | 上述已发布版本及激活已确认；最终修复另行发布 |
| MCP 站点登记与会话控制通道 | `register_site`、`open`、`status`、`pause`、`resume` 已实际走通 |
| 原生研究窗口就绪及采集握手 | 短目录候选已通过，明确 ACK 后进入 observing |
| 无交易异构站点，经 MCP 自动采集、搜索、读取 | 稳定候选会话 `91fc7b666038d0f21bf54efe453c77b86ff1459b8fab3e5a5d06ff9c986c746a`：18 资源、1 POST；源码标记、业务路径、未知集合、ESK、12.3400、中文值及凭据排除全部通过 |
| 暂停恢复 | 先前成功会话代次 2→3 paused→4 resuming→observing；资源 18→30，读取仍通过 |
| 币安网页加载 JS 经 MCP 读取 | 已通过；首次成功会话采集 133 资源、29 请求，读出带源码哈希的 13 项接口候选，见独立报告 |
| Win 独立 Profile 的本人币安登录 | 待验证 |
| U 本位网格列表真实路径、参数、响应 | 待验证 |
| 创建、修改、结束各自业务合同 | 已有静态候选；认证调用及业务合同未验证 |

首次币安采集出现 `event_queue_full`，已改为 256 件/32 MiB 双上限入口并补 7 项背压、断开及并发回归；SPA 同文档跳转也补就绪回执。稳定候选币安会话 `23d2d1c82ba3fbb59e6f7b63ce32e54b4da04cec9db0220141097d52c2824c64` 进入 observing，采集 181 资源/34 请求，无队列溢出，并再次经 MCP 命中 `update-grid-range`。仍如实报告 `observed_request_failed`、`body_not_available`，不声称每个网站请求都成功。固定覆盖提示 `coverage_top_frame_text_only_no_workers_websockets` 表示 V1 未覆盖 Worker/WebSocket，不等于工具未启动。

[币安 U 本位网格源码线索](binance-futures-grid-source-discovery-20260907.md)记录实际脚本资源、哈希、位置与候选方法。本机合成验收回执保存在 `.ai-tmp/fixture-verification-short-profile.json` 和 `.ai-tmp/fixture-verification-stable.json`，没有手动导出网页或把账号资料写入仓库。认证后的列表仍待用户在独立 Win Profile 登录，不用公共推荐策略冒充本人网格。

## 资源定位证据的边界

资源 SHA-256、`size_bytes` 和搜索/分片读取的字节 `offset` 对应凭据处理后落地的 UTF-8 文本。它们证明这份本机材料的身份和位置，不是原始网络响应或浏览器原始脚本的哈希与字节坐标。凭据替换及 JSON 重写可能改变长度、行列和字段排列。

CDP `initiator` 中的脚本行列来自浏览器原始脚本，不能直接等同于处理后本机文本的位置。本批没有自动坐标换算证据；后续现场记录须分别标明原始脚本来源/CDP 坐标和本机资源哈希/`offset`，避免错误关联。

原 Chrome 标签页与已安装观察扩展均未改动。本报告不把源码字符串、按钮存在或 HTTP 200 当成业务成功证据。
