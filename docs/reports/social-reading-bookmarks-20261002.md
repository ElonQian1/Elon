---
version_status: current
reviewed_at: 2026-10-02
implementation_status: implemented
owner: conversation-platform
---

# 群聊、好友多书签交付记录

需求：[首批合同](../requirements/social-reading-bookmarks-v1.md)。用户已授权实现和上线。
本记录区分源码、测试、发布与真机验收；未填写的发布身份不能理解为已经上线。

## 能力与边界

- APK 长按消息可添加阅读书签，会话顶部有「阅读书签」；PWA/Win 消息菜单和顶部有入口。
- 每个书签分别保存固定原始消息和续读位置，可命名、备注、复制、删除及撤销删除。
- 从书签的「继续阅读」或「回到原始标记」进入后激活对应书签；向前回看可以回退续读位置，原始标记不变。
- 普通进入仍显示最新消息；普通阅读位置独立。发送或显式回最新成功后结束书签续读。
- 使用消息身份和消息内比例定位，不用页码或永久像素 offset。默认定位附近 50 条，单页最多 100 条。
- APK/PWA 窗口最多 150 条/1.5 MB，Win 最多 300 条/3 MB；单条超大消息保留，故字节上限允许单条例外。
- 双向游标随保留窗口边界更新。新消息不把历史窗口拉回底部，跳转不把跳过历史连续标为已读。
- 私人元数据不复制聊天正文、不通知群友、不扩大 AI 输入；保存和跳转都复核账号、会话权限。
- 持久离线操作保留幂等 ID；创建回执丢失后删除仍会发送删除。跨设备冲突保留候选位置，由用户选择。
- 删除锚点可定位同会话附近消息；失效书签跳转失败保留原窗口和分页协议。

当前 UI 仅接入群聊、好友。服务端模型复用四种自有消息来源的权限和投影；
自有 AI/项目页面适配仍待后续交付。外部网页 AI 不在此次接入范围。
最远位置记录以服务器收到的位置为准，不用它计算阅读百分比。
浏览器多标签同时离线写入、长图片迟到后的精细像素恢复尚未形成专项验收证据。

## 实际接口与模块

- `GET /api/me/reading-capabilities`：能力检测，旧客户端继续使用 v1。
- `GET /api/me/reading-bookmarks?kind=group|friend&id=…&after=…`：私人书签分页及普通阅读位置。
- `POST /api/me/reading-bookmarks`：`action=create|update|delete|progress|resolve`，每次携带操作 ID；
  元数据与进度分别使用 revision，进度另有设备 ID 和序号。首批采用命令接口，未实现设计稿的逐 ID REST 路由。
- `GET /api/me/message-timeline/v2`：`before|after|sync|around|bookmark` 互斥；书签可选 `resume=true`。
- `POST /api/me/message-timeline/v2/window`：重验保留窗口。列表使用小规模分页刷新，未实现书签专用增量流。
- 服务端责任位于 `store/message_timeline/reading/`、`v2.rs` 和薄路由；Web 共用
  `reading_positions.js`、`reading_bookmarks_ui.js`，React 仅适配生命周期；原生独立适配位于 `reading/`。
- SQLite 迁移可重复执行；删除清理私有文本和位置，保留身份墓碑阻止旧操作复活。

## 验证与交付矩阵

| 项目 | implementation_status | verification_status | delivery_status | acceptance_status |
| --- | --- | --- | --- | --- |
| 私人存储、权限、定位与双向 API | implemented | integration_passed | published | upgrade_smoke_passed |
| PWA/Win 书签与续读 | implemented | integration_passed | published | browser_fixture_passed |
| APK 书签与续读 | implemented | offline_passed | published | device_partial |
| 自有 AI/项目页面 UI | not_started | not_run | not_started | pending |

已取得的证据：

- JS 15 项通过：十万条有界窗口、多书签、离线重启、延迟回执、保存失败、删除不复活、冲突及首次进度同步。
- Rust 完整服务端中 16 项消息时间线/书签测试通过：`bookmarks-rust-resume-20261002-151552-733`。
  包含五项新增真实 SQLite 书签测试；十万条定位、权限隔离、幂等、回退冲突和删除降级均通过。
- PWA 浏览器测试通过：`bookmarks-browser-final-20261002-135611-997`；真实 UI/恢复控制器，API 使用合成数据。
- Win 真实 React 页面浏览器测试通过：`bookmarks-win-final-20261002-151354-229`，API 使用合成数据。
- PC TypeScript 与生产构建通过：`bookmarks-pc-production-resume-20261002-151232-245`。
- 失效书签协议恢复后的 PWA 与 TypeScript 复验通过：`bookmarks-browser-rollback-20261002-152137-117`、`bookmarks-pc-rollback-types-20261002-152311-385`。
- APK 七个测试类最终 27 项通过：`bookmarks-android-missing-target-20261002-152250-619`；
  含失效书签保留窗口/恢复协议、真实 RecyclerView 重试、控制器重入及本机离线恢复。
- 首次在线加载普通阅读进度时初始化版本基准，避免新设备误报冲突；已捕获或排队的位置保留原版本，真实冲突仍须选择。
  新增回归后 APK 合计覆盖 28 项，其中 ReadingPositions 两项复验通过：`bookmarks-android-initial-progress-20261002-155034-473`。
  PWA/Win 浏览器复验分别通过：`bookmarks-browser-baseline-20261002-155903-612`、`bookmarks-win-baseline-20261002-155916-015`。
- 移动 V2 自测 12 项及规范检查通过；只证明治理合同，不替代设备运行验收。
- 旧库升级补充回归后，服务端消息时间线/书签 17 项全部通过：`bookmarks-rust-upgrade-20261002-162838-385`；
  指纹 `853c7db2a21ef2ac5a73b2e1109302ca8763656a88025f04b7cdbc64f45e7bd4`。
- 实际 non-fast-forward 后合入上游网格分享改动，书签文件无冲突。PWA/Win 浏览器复验通过：
  `bookmarks-pwa-after-rebase-20261002-165502-073`、`bookmarks-win-after-rebase-20261002-165458-713`。

## 构建兼容修复

验证入口及 Cargo 子进程继承调用者 PowerShell 版本，避免 PS7 被降为 WinPS5 后
在 UNC TEMP 编译 Add-Type 失败。容量准入和锁保持启用。PS7 JSON 日期直接转换为
DateTime，避免字符串往返丢失时区而误判 Cargo 源熔断。PS5/PS7 子进程探针和源恢复回归
均通过（`bookmarks-cargo-network-ps5-final-20261002-152301-093`、`bookmarks-cargo-network-ps7-20261002-134619-151`）；与书签业务分开提交。

## 发布身份与设备

- 功能提交 `ccdb93313`，首次同步修正 `7134fe07c`，旧库迁移修正 `361c617dcf9be42b2f744f9d2a68d45f38f8de5e`，均已推送主线。
- 服务端最终 `0.3.1815`；正式发布日志 `bookmarks-server-upgrade-publish-final-20261002-163850-333`。
  健康、精确源提交和 Win 前端入口检查通过：`bookmarks-final-server-check-20261002-170407-502`、
  `bookmarks-final-pc-check-20261002-170413-340`。
- PWA 独立运行时模板经正式静态发布脚本更新，来源同为 `361c617dc`，SHA-256
  `f1d368eebea995700466e8ec7c0f4db80a0523cdd56df062cf0f51612b1ab6df`；
  日志 `bookmarks-pwa-final-generation-20261002-165951-482`。只发布后端不会替换线上已有运行时模板，因此单独核对实际 `/web` 页面。
- 本任务正式发布 APK `1.1.1851` 后，复用发布系统中同一最终提交的 `1.1.1852` 工件；
  最终 APK SHA-256 `12d73254914e85db53a860b3c063b2b5216f1d0f47a0e9b3f5feb45547ccf908`。
  完成检查 `bookmarks-final-apk-check-20261002-170858-506` 通过。
- 小米保留数据更新到 1851，随后已安装正式 1852；ADB 回读版本与安装包 SHA-256 均与线上 1852 完全一致。
  本任务 1851 的正式装机回执为 `com.elon.app-1851-6f968ff144964309bfe7960bc5ca07bf.json`，荣耀结果 `offline`，未冒充已装机。

### 真机发现与修正

1851 首次验收发现书签入口不显示：建表曾挂在已有 v310 迁移末尾，已执行 v310 的旧库不会重跑。
现恢复 v310 原职责并新增独立 v311。新增回归模拟已升级到 v310 的 SQLite，调用正式迁移入口两次，
验证只登记一次 v311、原消息保留、书签能够创建并列出。线上日志确认在 `2026-10-02T09:02:58Z` 执行 v311。

在小米正式 1852 上已确认群聊「阅读书签」入口、长按「添加阅读书签」、名称/备注编辑器和输入焦点。
随后设备被其他操作切换到 AI/网格页面，已停止输入并请求独占验收时间。
多书签跳转、滚动更新、退出恢复与旧消息翻页的完整真机流程仍待执行；不能把入口可见当作全部通过。
待恢复时先检查并清理本任务临时名 `QA-20261002-A`（本次未确认保存成功），再执行 A/B 独立续读及撤销删除。
本任务没有发送测试群消息或清空账号数据。浏览器运行证据来自真实页面/组件配合合成 API；真实登录浏览器跨设备联动尚未专项验收。
