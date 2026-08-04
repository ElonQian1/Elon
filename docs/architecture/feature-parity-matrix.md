# 功能迁移矩阵

状态含义：`主实现` 表示该客户端的开发归属；`兼容` 表示入口仍需保留；`结构已核对` 表示代码入口和服务端锚点已由自动检查确认；`功能待验收` 表示还需要真实用户流程验证；`可退役` 表示满足清理条件后可以删除。

| 功能 | PC 新入口 | 移动 Web | Android | 服务端/API | 当前结论 | 下一步 |
|---|---|---|---|---|---|---|
| 首页总 AI | `pc-frontend/src/features/ai/` | `server/src/assets/web_page.html` 对应聊天逻辑 | `MainActivity` 主聊天、Agent 聊天 | `lm_chat.rs`、`home_ai_*` | 结构已核对；PC 主实现；移动 Web/Android 为并行兼容入口；功能待验收 | 用真实会话验证上下文、工具调用、流式回复和错误降级 |
| 项目广场 | `features/plaza/ProjectPlazaView.tsx` | `project_plaza.js/css` | `ProjectPlazaActivity.kt` | `project_store.rs` | 结构已核对；三端并存，不能全量删除移动 Web 或 Android | 对照浏览、搜索、进入项目、分享和下载流程 |
| 项目中心 | `features/projects/` | `project_home.js/css` | `MainProject*` 与原生项目浏览 | `project_api.rs`、`project_store.rs` | 结构已核对；PC 主实现；移动 Web/Android 仍有运行责任 | 对照创建、打开、设置、成员和项目会话流程 |
| 电脑医生 | `features/doctor/` | 移动 Web 保留镜像/入口 | Agent/节点诊断能力 | `node_router.rs` 及节点诊断 API | PC 新入口主实现；其他端能力边界待验收 | 验证离线、失败、重试、只读检查和修复授权状态 |
| PC AI 开发任务 | `features/dev/`、`features/local-tasks/` | 不属于移动 Web 主路径 | 通过 API 展示或调用 | `ai_cli/`、`node_agent_*` | 结构已核对；PC 主实现；功能待验收 | 验证任务创建、过程、取消、恢复、审批和产物验收 |
| 登录/账号 | `features/auth/`、`features/account/` | Web 登录链路 | Android 登录 | `auth_api.rs` | 多端保留；结构已核对；协议一致性待验收 | 使用同一账号验证登录、退出、过期和权限边界 |

## 使用规则

- 新功能只能填入 PC 新入口、移动 Web、Android 中明确的主实现列。
- 旧入口如果仍然运行，必须标记原因和退役条件。
- 没有完成状态和测试证据的行，不得把 legacy 标记为可删除。
- `scripts/check-feature-parity.ps1` 只检查结构入口和归属锚点，不把“文件存在”误判为功能完成。
- 只有“功能待验收”对应的真实流程完成并留有测试证据后，才允许把兼容实现推进到退役评审。

## 当前审计结果

截至 2026-08-03，自动检查已确认 7 组功能的 PC、移动 Web、Android 或服务端实现锚点存在，且没有重新跟踪旧的 `server/src/assets/pc_*.html/js/css`。这说明“改到旧 PC 代码”的风险已被结构门禁拦住，但不代表三端行为已经完全一致；功能验收仍按上表的“下一步”执行。
