# Windows 会话同步修复发布回执

日期：2026-09-15。

- 功能：`pc-social-chat-recovery-v1`。
- 功能源码提交：`aec17a1ba7733cc76a75e37dc20726d06f4f54ba`，已推送 `origin/main`。版本配对核验工件：`c1e9e31dc7fe3a8779a8ed6f9c99ed6ce7580795`；两者前端输入完全一致，后者增加发布文档和注册表状态。
- 发布：`scripts/publish-pc-frontend.ps1 -ReuseLiveServer`，正式发布入口返回 `PC_FRONTEND_RELEASE_STATUS=published`。
- 最终发布方式：`frontend_only / direct_server_ancestry`。兼容在线后端 `0.3.1756`，提交 `45238e1e2e1ab4d17e4148890561814674d0e8ee`；脚本验证前端变更不依赖未发布后端合同。
- 版本配对核验时，在线 `/pc/assets/release.json` 报告上述工件及后端 SHA；构建时间 `2026-09-15T07:31:02.4981125Z`。
- `/pc`、`/pc-next`、聊天资源均返回 HTTP 200；缓存/同步事件所在资源和 FriendsPage 资源与本次构建的 SHA-256 一致。
- 构建产物门禁通过；全站体积有既存软阈值告警，未阻止发布。本次未修改全站分包策略。

首次发布成功绑定后端 0.3.1755；收尾期间并行任务发布了 0.3.1756，严格版本配对校验拒绝旧绑定。已通过同一正式发布入口重建并发布相同前端源码，绑定最终后端，无手改线上元数据或绕过校验。

PC 收尾校验还要求线上提交包含本回执的 Git 提交，因此文档回执提交后必须再运行正式前端发布，随后直接执行收尾，不再追加新的回执提交。最终发布 SHA 以 `/pc/assets/release.json` 和 `finish-ai-task.ps1` 的一致性核验为准；该最终发布与上面已核验工件的前端源码相同。

已打开的旧 Windows 页面需退出并重新打开一次以加载新前端；后续前后台切换由新的恢复逻辑自动同步。无需重新安装 Windows 安装包。

浏览器交互、兼容回归与真实设备边界见 `pc-social-chat-recovery-validation.md`。本回执是线上静态部署核验，不冒充用户实体 Windows 的长期休眠验收。
