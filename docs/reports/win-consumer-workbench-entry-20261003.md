---
version_status: current
reviewed_at: 2026-10-03
implementation_status: released
---

# Windows 消费者工作台入口交付记录

## 范围与发布身份

- 需求：[消费者工作台入口与恢复 V1](../requirements/win-consumer-workbench-entry-v1.md)。
- 功能：`win-consumer-workbench-entry-v1`；默认 `/ai`，开发工具分层，连接恢复不切换工作台。
- 右侧“全站用户”保持默认展开，并尊重用户已保存的布局偏好。
- 已发布源码 SHA：`2dc7d72e16b0f03d1c7a0b86899ae5475d177e40`。
- Windows 节点及桌面身份：`0.3.69+2dc7d72e16b0f03d1c7a0b86899ae5475d177e40`。
- 本报告与注册表状态属于后续文档提交；其基线为 `5d68a4ded7a312a70017c029840cdc2d8a2c9349`。
  文档提交 SHA 不代表重新构建或发布了客户端，不能代替上述源码发布 SHA。

## 已验证结果

- 消费者入口、默认登录回跳和错误返回复用现有 AI 工作台；明确开发工具深链仍可使用。
- 连接模型读取真实 readiness JSON；首次失败、四次失败迟滞、串行重试、取消和原地恢复通过。
- 账号刷新区分网络错误和 401，旧账号延迟响应不会覆盖新账号；连接行为测试 14 项通过。
- 消费者导航 12 种组合、目录恢复、本机任务及路由错误边界定向回归通过。
- TypeScript 检查、变更范围 lint 和生产构建通过；正式发布重构建覆盖最终源码字节。
- 云端 PC 发布及完成检查通过：release marker 指向发布源码 SHA，页面 HTTP 200。
- Windows 本机构建及精确更新通过；验收 run：`7b6c1b8f-1fc3-48ad-bcee-26776567372a`。
- 节点、桌面发布身份均与目标完全一致，Tauri 和前端在线，桌面 PID 为 `33480`。
- 远端 outbox 第 1 次执行进入 `synced`；Windows ZIP、安装器、节点 EXE 下载均为 HTTP 200。

## Windows 现场入口证据

- 更新后未先执行 `navigate`，新鲜前端 heartbeat 已报告 `/pc/ai`。
- 调用正式安装启动器重复打开后仍为 `/pc/ai`，桌面 PID 仍为 `33480`，未新增桌面进程。
- 两次 `capture_state` 动作均成功；路由来自对应新鲜 heartbeat，不来自动作回执。
- 首轮探测误以为 `capture_state` 含路由而失败；修正测试读取位置后通过，不属于应用故障。
- 持久现场证据位于 Git common directory 的
  `ai-task-evidence/win-consumer-workbench-entry-v1/native-entry.json`。

## 浏览器验收与限制

- 5 个 Edge React fixture 场景覆盖本机/云端入口、全站用户默认与偏好、断网提示、草稿和恢复。
- 四张持久截图位于同一证据目录：`local-visitor.png`、`local-signed-in.png`、
  `local-offline-draft.png`、`local-signed-in-recovered.png`。
- 断网与恢复使用受控浏览器 fixture，并非 Windows 实机物理断网测试。
- 未执行真实第三方账号登录验收、进程/页面崩溃注入；未取得原生桌面截图。
- 完整 `test:user-browser` 仍有既有 catalog 断言失败：
  `pc-frontend/scripts/test-local-ai-private-transport-catalog.cjs:230`，实际 `36`、期望 `17`。
- 该测试及相关两份 catalog 源码在父提交 `f5a64bdc4dde36857055ae885eadd74f16799cf5`
  与本次发布提交的 Git blob 相同；此项保留为既有测试债务，不宣称全套通过。

## 发布收据与收尾

Git common directory 下保留以下结构化日志，均为 `status=passed`、`exit_code=0`：

- `ai-command-logs/win-consumer-pc-publish-20261003-181555-614.result.json`
- `ai-command-logs/win-consumer-node-publish-dtemp-20261003-183114-129.result.json`

源码 `NodeAgent` 统一收尾已返回 `BUSINESS_STATUS=complete`、`LOCAL_MAIN_STATUS=current`、
`FINALIZABLE=true`。本报告和 feature 的 `released` 状态另走新预检产生的 `DocsOnly` 收尾，
不会把文档提交冒充新的节点版本；文档提交与收尾由本批主代理完成。
