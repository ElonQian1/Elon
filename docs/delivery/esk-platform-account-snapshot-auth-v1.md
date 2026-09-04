---
version_status: current
reviewed_at: 2026-09-04
---

# 正式 ESK 摘要会话同快照修复交付记录

需求：[摘要与会话同快照读取 V1](../requirements/esk-platform-account-snapshot-auth-v1.md)。
Feature：`esk-platform-account-snapshot-auth-v1`。

## 真实问题与修复

基线 `a4fd333061f898895974cc845d5453d274e39087`：HTTP 预检验证登录后丢弃 token，
独立摘要事务只检查用户 active，无法拒绝在预检后被撤销的会话。
此外，已固定政策而本人无分录时，旧摘要跳过政策摘要完整性复核。

两个合成 SQLite 回归在修改生产源码前实际失败：撤销后旧摘要仍返回夹具 10 ESK，
损坏政策下旧摘要仍成功返回零。它们不是生产余额，也不表示发生过实际用户泄漏。
红测：`0 passed / 2 failed / 58 filtered`；指纹
`4a8eaa6854ac85a02cdef52ad15d6c104036fd702120ae74837900a292f97f64`，
日志名 `esk-account-auth-red-regression-20260904-200501-986`。

修复保留 API 本次 token，Store 显式要求它；摘要直接投影已实现的 history 第一页，
会话、政策和全部分录在同一 SQLite 只读快照内验证。旧重复扫描已移除，
但原 JSON 18 字段、entry 6 字段、capabilities 5 字段及双 APK 合同不变。
Store 前 N 条规范化保持 1–100；公开 HTTP 非法 limit 继续拒绝。

线性化点为读取快照建立时，不宣称能追回已经开始或已发送的响应；
客户端原有换号、后台、超时和退出清理继续保留。
Store 严格只读；HTTP 既有鉴权可能节流更新 `last_seen_at`，本修复不改变该全局行为。

## 四类状态

| 维度 | 当前状态 | 未完成项 |
| --- | --- | --- |
| implementation_status | implemented：正式摘要 token 传递及同源快照投影 | 无资金或卖回功能新增 |
| verification_status | environment_passed：本机 81 项通过，线上身份、健康及未登录保护检查通过 | 真实本人验收另列，不以本机测试或公开 smoke 冒充用户验收 |
| delivery_status | deployed：源码已推送，Server 官方发布及独立线上核验通过 | 本轮未重发 APK；量化上传单独交接 |
| acceptance_status | pending | 真实用户、受保护连接、付款审核入账 |

本轮 81 项通过；基线 2 项失败是修复前的复现证据，不计入通过总数。
不更改 Android/PWA 画面、共享 IPC、数据库迁移、生产政策、真实用户或任何资金。
上一轮主 APK `1.1.1511` 无需因该后端兼容修复重新构建；实时版本以公开清单为准。

## 验证与恢复边界

- 修复后 harness：`65 passed / 0 failed`，指纹
  `d7a5c88dddc85fa9182a038d7cf90f4011c33a958bf718969a5a77009a1744cc`；
  日志名 `esk-account-auth-harness-20260904-201000-193`，实际测试正文已复核。
- 本机生产代码进程内 Router：`16 passed / 0 failed / 2333 filtered`（新增 5 项），指纹
  `66e71c96d1c4761a02d06393331283995b420517a80822b43cb9527c25685686`；
  日志名 `esk-account-auth-production-router-20260904-201129-978`，含完整 Server 源码编译，
  整个命令 610.8 秒、实际测试 38.08 秒，期间沿同一进程等待，未重复启动。
- 回归通过当前生产 Store/Router 和真实临时 SQLite，不以字符串检查代替会话边界。
- 验证空、分页、不同用户、会话撤销/到期/重绑、政策及页面外分录损坏。
- 正式 5 表、Paper 和会话权限等比较完整持久行；Store 查询另检查 `total_changes()`。
- 修复没有迁移或写入操作；恢复使用 Git/官方后端发布流程，不修改生产数据库。
- 旧实现测试签名增加合成 token；缺失/停用/虚拟用户的 Store 摘要预期变为
  Unauthorized，付款准备/记账的 UserUnavailable 预期保留。
- 独立实现审查未发现新 P1/P2；事务中失效沿既有 Unauthorized 映射返回 403，
  请求预检失效仍为 401；客户端对 403 显示不可用并清除旧内容，不回退 Paper。
- 历史 released 功能保留原发布快照证据，不能手改旧回执或自动洗掉指纹漂移。
  本修复有意更新的签名与摘要实现以该新 Feature、完整回归及发布证据继续追溯。

## 发布身份与线上只读验证

功能源码 `53a32b33b598765cf5a8c082dc63f9622944e6ed` 已推送 `origin/main`。
首次推送实际 non-fast-forward 后，按流程 rebase；上游仅有底部导航变更，
本轮 ESK 源码和测试与原提交 `271e07f35` 逐字节一致，13 项证据均有效。

官方 Server 发布版本 `0.3.1726`，源 SHA 与上述提交一致；
日志名 `esk-account-auth-publish-server-20260904-202528-502`，
结果 `passed`，1495.5 秒，`timed_out=false`、`stalled=false`；
同一编译进程等待至结束，没有重复启动，优化构建耗时 24 分 02 秒。
发布器输出 `SERVER_RELEASE_STATUS=published`，版本由服务器分配，未手改 Git 版本。

2026-09-04 12:52:30 UTC 独立无凭据检查：
`/api/server/version` 返回 200，`versionName=0.3.1726`、`gitSha=53a32b33…` 精确匹配；
`/health` 返回 200，正文 `OK`。
12:51:03 UTC，正式摘要和 history 路由均对未登录请求返回 401，
具有 `Cache-Control: no-store`、`Pragma: no-cache`、`Referrer-Policy: no-referrer`，
错误仅为“需要真实用户登录”。公开 smoke 不查询私有账户，也不证明本人余额已送达。

本轮使用 `publish-server.ps1 -SkipPcFrontend`，未重发 APK 或 PC 前端，
未更改 Android/PWA 源码、数据库迁移、生产政策、真实余额或任何资金。
既有 HTTP 公开源保持原样；没有配置证书或放宽私有读取的传输保护。
所有线上身份是本批时间点快照，后续发布应重新核验，不能拿旧回执冒充最新版本。

## 构建、本机与交接

复用项目共享 Rust 缓存；验证开始前 doctor 检查时，源码/安装/启动器/Skill 一致，
且没有活跃 Cargo 写者；此结果不代表后续发布期间没有编译进程。
项目 `elon-cli`，domain `agent-validation`，共享验证分区 `validation-heavy`；
缓存平台指纹 `ceee6c6eb858eb0ebdbc4b13c8457147a223d66fd295288e089d75b6157685a8`。
磁盘余量约 8.73% 告警；本轮没有 GC、系统配置变更或未知文件处置。
临时任务根由正式预检分配，主同步 checkout 原有改动保持不动。
最终仓库收尾以本报告推送后的 `finish-ai-task.ps1 -Kind Server` 回执为准。
本次交接文档修订限定上述交付记录和首批用户路线图；相对 Markdown 链接检查通过。
当前工具未提供 `project_docs_test_retrieval`，不宣称已完成该检索验收。

## 下一步仍以首批用户闭环为目标

- 广场公开量化下载经本轮无凭据完整字节验证仍是 V20 `0.2.0 (2)`；
  V28 `0.4.0 (4)` 签名包存在但未上传，需要现有项目编辑者发布凭据的安全注入。
  本轮仅检查当前进程是否已配置 `YILONG_PROJECT_RELEASE_TOKEN`，结果为 false；未读取值。
  不能在后续文档提交上冒充 APK 内嵌源码；不得重做已完成的正式摘要接收端。
- 正式卖回申请/占用/取消尚未实现，Paper 占用不能计入正式可售额。
  入账 history 摘要不包含未来占用事件，不能拿它证明可售额度未变；
  新申请开关关闭时，合法取消应仍可释放占用，取消不能追加 ESK 入账。
  生产启用前需确认额度、期限/取消、占用期参与权重及用户条款，不默认报价或兑付。
- 真实用户和付款资料/审批、私有传输、Sui 授权/终局性与双 APK 本人验收仍待完成。
  不从本轮继续开发推导真实入账、资金操作或生产启用授权；总 Goal 保持进行中。
