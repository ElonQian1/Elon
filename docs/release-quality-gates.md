# 发布质量门禁矩阵

本文档固定一龙项目的发布自测顺序。目标是让后端、PC Web 工作台、PC 节点客户端和 Android APK 发布路径都有明确门禁，避免“代码合了但用户没用上”“CI 过了但手工发布绕过预算”“部署后接口不可用仍标记成功”。

## 总原则

- 没有通过对应 `check-task-complete.ps1 -Kind ...` 前，不把用户可见任务汇报为完成。
- 发布脚本里的门禁失败时，不手工上传旧产物兜底；先修失败原因，再重新走脚本。
- `-SkipBuild` 只允许复用已知新鲜产物；复用的 PC dist 仍必须通过 bundle budget。
- Bundle budget 采用分级门禁：软线只告警并允许发布，硬线或压缩体积上限才阻断。当前最大异步 JS 的原始体积软线为 `480 KiB`、硬线为 `520 KiB`，Gzip 硬线为 `140 KiB`。
- 并发发布被更新主线超越时，默认交给最新主线发布，不强制覆盖。
- UI 截图、遮挡、交互类修复除了构建和发布，还需要本地预览、截图、DOM 或线上页面证据之一。

## 门禁矩阵

| 发布对象 | 触发场景 | 发布前本地检查 | 发布脚本 | 脚本内阻断门禁 | 完成验收 |
| --- | --- | --- | --- | --- | --- |
| Rust server | `server/` 后端行为、API、任务调度、计费、存储、AI CLI 流程变更 | `scripts\check-rust-warning-budget.ps1 -MaxWarnings 0`；`scripts\cargo-dev.ps1 test --manifest-path server\Cargo.toml`；`scripts\check-source-size.ps1` | `scripts\publish-server.ps1` | release claim；git 最新性；musl build 注入版本和 git SHA；上传前 SHA 顺序检查；flock CAS；重启后 `/health`、`/api/server/version` 阻断式 smoke | `scripts\check-task-complete.ps1 -Kind Server` |
| PC frontend `/pc` | `pc-frontend/`、用户可见 Web 工作台、路由、样式、会话页、模型/节点/项目页面变更 | `npm run build`；`npm run check:bundle-budget`；相关前端专项测试，例如 `npm run test:message-flow`、`npm run test:admin-realtime` | `scripts\publish-server.ps1` | PC dist rebuild 或复用旧 dist；bundle budget；server post-deploy smoke；上传 `/pc` dist 前预算检查 | `scripts\check-task-complete.ps1 -Kind PcFrontend` |
| PC node agent | `server/src/node_*`、Windows 客户端、launcher、节点自动更新、内置 PC 工作台包变更 | Rust/前端相关测试按变更面选择；确认 `pc-frontend` 可构建 | `scripts\publish-node-agent.ps1` | Linux/Windows agent build；内置 PC 工作台 build；bundle budget；manifest 写入；上传后 node-agent manifest 与下载端点 HEAD smoke；广播前公开开发节点握手等待 | `scripts\check-task-complete.ps1 -Kind NodeAgent` |
| Android APK | `android/`、移动端可安装能力、APK 更新、Android 原生交互或 APK 发布脚本变更 | Android/后端相关测试按变更面选择；先用 `scripts\check-task-complete.ps1 -Kind CodePushed` 确认业务提交已进入 `origin/main` | `scripts\publish-apk.ps1 -Changelog "<说明>"` | release claim 分配 `versionName/versionCode`；临时写入 Gradle 且不提交；`assembleRelease`；本地 APK manifest 版本校验；origin/main freshness；服务器 APK freshness；上传后 `/app/version.json` 和远端 APK manifest 校验 | `scripts\check-task-complete.ps1 -Kind AndroidFeature` |

## 轻量流程

1. 本地先跑统一预检：日常改动用 `scripts\check-local-quality.ps1 -Scope Static`，发版前按影响面升级为 `-Scope Server`、`-Scope Frontend` 或 `-Scope All`。
2. 先合并代码：本地改动通过必要测试后 commit/push，确保 `origin/main` 包含本次业务提交。
3. 再选择发布路径：后端和 PC frontend 走 `publish-server.ps1`；PC node agent 走 `publish-node-agent.ps1`；Android APK 走 `publish-apk.ps1`。
4. 发布脚本必须自然结束：如果脚本提示 superseded、bundle budget failed、post-deploy smoke failed、CAS conflict，都不能当作完成。
5. 最后跑完成验收：按影响面选择 `Server`、`PcFrontend` 或 `NodeAgent`。
6. 最终回复带证据：说明代码同步状态、发布状态、线上版本或 git SHA、关键 smoke 结果。

## 失败处理

| 失败点 | 处理方式 |
| --- | --- |
| `source-size guard` 拦截红区文件增长 | 不放宽门禁；抽共享脚本或模块，保持红区文件不继续变大。 |
| `check:bundle-budget` 软线告警 | 发布可继续；记录增长来源，后续优先 route/component lazy loading、拆出重页面 chunk 或移除无用依赖。 |
| `check:bundle-budget` 硬线超限 | 阻断发布并修复；预算调整必须基于实际网络传输、解析成本或功能边界，显式说明原因，不能为消除单次失败直接放宽。 |
| `publish-server.ps1` post-deploy smoke 失败 | 脚本会 `finish(success=false)`；检查服务日志、端口、版本注入和 `/api/server/version.gitSha`，修复后重新发布。 |
| `publish-node-agent.ps1` manifest/download smoke 失败 | 不广播更新；检查 `/opt/elon/data/downloads` 产物、manifest SHA、下载路由和服务器 data_dir。 |
| `publish-apk.ps1` manifest 或 freshness 失败 | 不上传或不继续覆盖；检查 Gradle 版本注入、aapt manifest、`/app/version.json`、origin/main 是否已被 Android 相关提交推进。 |
| 并发发布被更新主线超越 | 正常停止并汇报“代码已合并，发布交给最新主线”；只有明确协调发布时才重新运行。 |
| `check-task-complete.ps1` 失败 | 不做完成汇报；按错误提示补发布、补上传或等待线上版本切换。 |

## 证据模板

```text
CODE_SYNC_STATUS=synced
SERVER_RELEASE_STATUS=published|not_attempted|superseded_by_newer_main
PC_FRONTEND_RELEASE_STATUS=published|not_attempted
NODE_AGENT_RELEASE_STATUS=published|not_attempted
APK_RELEASE_STATUS=published|not_attempted
ONLINE_VERSION=<version or manifest version>
ONLINE_GIT_SHA=<git sha>
SMOKE=<health/version/download result>
```

## 脚本职责索引

- `scripts\publish-server.ps1`：server 发布、PC frontend dist 发布、版本 claim、并发保护、post-deploy smoke。
- `scripts\publish-node-agent.ps1`：PC node agent 构建、打包、上传、manifest/download smoke、广播更新。
- `scripts\publish-apk.ps1`：Android APK 版本 claim、Gradle release build、APK manifest 校验、上传和 APK 完成验收前置发布。
- `scripts\publish-server-pc-frontend.ps1`：PC frontend 构建辅助和 bundle budget 发布侧检查。
- `scripts\publish-health-checks.ps1`：发布后 server/node-agent smoke helper。
- `scripts\check-local-quality.ps1`：本地统一质量预检入口，支持 `Static`、`Server`、`Frontend` 和 `All` 四种范围。
- `scripts\check-pc-frontend-bundle-budget.js`：Vite 产物分级预算门禁；轻微原始体积增长告警，明显膨胀或 Gzip 超限阻断。
- `scripts\test-pc-frontend-bundle-budget.js`：预算门禁夹具测试，覆盖正常、软告警、原始体积硬失败和 Gzip 硬失败。
- `scripts\check-source-size.ps1`：源码体积门禁，阻止巨型文件继续扩张。
- `scripts\check-dependency-audit.ps1`：Rust/npm dependency audit 汇总门禁。
- `scripts\check-rust-warning-budget.ps1`：Rust warning budget 门禁，当前预算为 0。
- `scripts\check-release-runbook.ps1`：静态校验本 runbook 的脚本引用和 `check-task-complete.ps1 -Kind` 枚举没有漂移。
- `scripts\check-ci-quality-gates.ps1`：静态校验 GitHub Actions CI workflow 必须保留 Rust、Realtime、dependency、warning、frontend build 和 smoke 门禁。
- `scripts\check-realtime-runbook.ps1`：静态校验 Realtime 运维 runbook 必须覆盖管理接口、窗口字段、告警阈值、channel、close reason、边界模块和回归命令。
- `scripts\check-realtime-ownership.ps1`：静态校验 Realtime owner 表必须覆盖源码里的 channel、close reason 来源、owner 模块、指标写入点、诊断字典 API 和变更规则。
- `scripts\check-realtime-diagnostics-snapshot.ps1`：静态校验 Realtime diagnostics JSON 快照结构、源码 `include_str!` 绑定和 snapshot 单测，防止 `/api/admin/realtime/diagnostics` 契约漂移。
- `scripts\check-task-complete.ps1`：最终完成验收和线上 provenance 校验。
