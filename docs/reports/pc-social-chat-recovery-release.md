# Windows 会话同步修复发布回执

日期：2026-09-15。

- 功能：`pc-social-chat-recovery-v1`。
- 前端提交：`aec17a1ba7733cc76a75e37dc20726d06f4f54ba`，已推送 `origin/main`。
- 发布：`scripts/publish-pc-frontend.ps1 -ReuseLiveServer`，正式发布入口返回 `PC_FRONTEND_RELEASE_STATUS=published`。
- 发布方式：`frontend_only / isolated_frontend_commits`。兼容在线后端 `0.3.1755`，提交 `48b8732958bc674cd56ba76ddeb88db6131e555d`；脚本验证前端变更不依赖未发布后端合同。
- 在线 `/pc/assets/release.json` 报告上述前端、后端 SHA；构建时间 `2026-09-15T07:27:52.8897609Z`。
- `/pc`、`/pc-next`、聊天资源均返回 HTTP 200；缓存/同步事件所在资源和 FriendsPage 资源与本次构建的 SHA-256 一致。
- 构建产物门禁通过；全站体积有既存软阈值告警，未阻止发布。本次未修改全站分包策略。

已打开的旧 Windows 页面需退出并重新打开一次以加载新前端；后续前后台切换由新的恢复逻辑自动同步。无需重新安装 Windows 安装包。

浏览器交互、兼容回归与真实设备边界见 `pc-social-chat-recovery-validation.md`。本回执是线上静态部署核验，不冒充用户实体 Windows 的长期休眠验收。
