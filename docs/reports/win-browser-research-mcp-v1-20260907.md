---
version_status: current
reviewed_at: 2026-09-07
implementation_status: in_progress
---

# Win 浏览器研究 MCP V1 验证记录

本报告记录实现证据，不能代替[需求](../requirements/win-browser-research-mcp-v1.md)。截至首批提交，代码、离线验证已完成，部署与真实 WebView2/币安现场验收仍待进行；没有执行真实金融交易。

## 已验证

| 检查 | 结果 | 本机日志名称 |
|---|---|---|
| `elon-pc-node` 正式入口 check | 通过 | `browser-research-node-check-final-20260907-055710-734` |
| `elon-desktop` 研究模块测试 | 27/27，通过实际 Windows 编译 | `browser-research-desktop-tests-final-20260907-055554-355` |
| 独立 HTTP/MCP/队列 harness | 12/12 | `browser-research-harness-tests-final-20260907-055328-209` |
| 前端行为测试 | 17/17 | `browser-research-owner-error-tests-20260907-055059-147` |
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

| 项目 | 首批提交时状态 |
|---|---|
| 匹配节点与 Win 宿主激活 | 待验证 |
| 无交易异构站点，经 MCP 自动采集、搜索、读取 | 待验证；样例 `desktop-shell/tests/browser-research-fixture.mjs` 已准备 |
| 币安网页加载 JS 经 MCP 读取 | 待验证 |
| Win 独立 Profile 的本人币安登录 | 待验证 |
| U 本位网格列表真实路径、参数、响应 | 待验证 |
| 创建、修改、结束各自业务合同 | 未验证 |

原 Chrome 标签页与已安装观察扩展均未改动。本报告不把源码字符串、按钮存在或 HTTP 200 当成业务成功证据。
