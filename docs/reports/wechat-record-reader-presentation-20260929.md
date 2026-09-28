---
version_status: current
reviewed_at: 2026-09-29
implementation_status: implemented_native_acceptance_deferred
---

# 微信聊天记录阅读体验

范围为用户主动导入的记录阅读器，不读取微信数据库，也不改微信外部播放返回栈。能力标识 `social_wechat_record_reader_presentation_v1`。

## 展示与操作

| 原入口或内容 | 新路径 |
|---|---|
| 单独的发送者名称 | 稳定首字头像、昵称、时间；同名得到同色，不伪造微信账号或真实头像 |
| 导出的类型标签 | 根据结构化 kind 去除对应的开头标签；普通文字和未知类型保留 |
| 卡片旁的长 URL | 仅隐藏已由卡片表示的 URL；完整地址仍用于打开、复制和转发 |
| 重复文章标题 | 仅折叠与预览标题一致的导出标题行，保留用户补充说明 |
| 视频文件按钮 | 可见时读取缩略图，中央播放、可用时显示时长、下方文件名；仅点击后播放 |
| 卡片操作 | 常驻复制链接；视频号同时保留查看原网页；长按/右键/键盘菜单可转发 |
| 原始文本、撤回 | 更多菜单；撤回仍需确认，原文及存储不改写 |

Android 转发复用 ExternalShareActivity 的接收方选择与确认。PC/PWA 共用受限转发面板，调用现有群目录和消息接口，用户选择群并确认后才发送；超时或不确定结果不自动重发。可继续在群中渲染原有社交卡片，不向目标群复制整个聊天记录。

## 模块和缓存

Android：ChatRecordPresentation、ChatRecordRowView、ChatRecordLinkActions、ChatRecordVideoPoster；Activity 保持导航和组装。

PC：RecordMessage、RecordAsset；通过共享 chat_record_presentation/actions/video/media 消费与 PWA 一致的展示、媒体与转发规则。没有新增旧版 PC 页面。

继续复用已发布的结构化记录及独立附件 API；不重新解压 ZIP。缩略图第一次需要读取视频字节，之后复用权限复验后的附件缓存及派生缓存，不承诺首次零下载。Android 海报文件继承权限缓存前缀与内容摘要，随原缓存清理；浏览器以站点、账号、记录、不可变附件 ID 隔离，最多 64 项/16 MiB/7 天，内存最多 32 项。视频首帧生成排队，失败仍允许尝试播放。

旧缓存与权限机制见 [上一批证据](wechat-record-reader-cache-20260928.md)。撤回、退群、账号切换不能依赖海报缓存绕过正文/媒体权限校验。

## 验证

- 共享 JSON 用例验证 12 种标记/链接/说明保留场景，Node 共 13 项通过；Android 使用同一份 fixture。
- PC TypeScript/Vite 生产构建通过。
- PC/PWA 1280 与 390 四组真实浏览器测试通过：卡片、视频首帧、不自动播放、嵌套返回、窗口拖动、HTTP 200/304 复用、撤回后 403 清空阅读、先选择再确认转发且一次写入。
- 原 PWA 阅读器交互回归通过，保留原文、注入文本防护和账号切换关闭。
- Rust 服务端 check 通过；仅扩展静态资源注册，无业务存储或接口协议变更。
- Android 调试构建及 23 项定向回归通过，覆盖卡片点击、长按菜单、完整链接复制、浅深主题、导入和缓存；真实设备验收未通过安装阶段。
- 最终浏览器回归补验 HTTP 剪贴板降级保留完整链接、下载期间的播放点击不会丢失；PC 生产构建再次通过。

浏览器工件在任务 `.ai-tmp/chat-record-cache-ui`；均为合成记录、合成视频，不是用户群聊内容。日志由 invoke-ai-logged-command 持久化。调试专用 ChatRecordPreviewActivity 只在独立 `.recordtest` 包生效，不进入正式 APK。

2026-09-29：通过登记信息恢复小米无线 ADB 并核对硬件身份。独立验收包 `adb install -r` 返回 `INSTALL_FAILED_USER_RESTRICTED: Install canceled by user`，故未启动阅读器、未取得原生截图；没有清数据或覆盖生产包，也不把 ADB 连通记为安装成功。

## 发布边界

本任务不撤销 [主线 V2 原生运行证据门槛](mobile-design-v2-desktop-takeover-20260928.md)。阅读器的合成数据检查不能替代整个主题迁移、生产登录、键盘和业务实测。正式发布、设备安装以及未验证项须以本批最终回执更新，不能根据源码或调试包推断。

## 本批回执

- Android 源码提交 `85ecd3bae`，PC/PWA 与共享资源提交 `55ee1b753`，均已推送主线。合并保留同期封面恢复逻辑，并重跑 PC 构建、四组阅读器浏览器验收和封面恢复专项，全部通过。
- `publish-server.ps1` 正式发布 `0.3.1790`。线上服务端版本与 PC `assets/release.json` 均绑定 `55ee1b753de33c6d63b00bab99c6579c31a7e647`，四个新增 JS 资源逐一与本地源码内容比对一致。
- APK 生产发布未执行；独立调试包安装失败，真机视觉及业务验收延期。不得将网页发布回执解读为 APK 已更新，也没有确认当前已经打开的 Win 窗口热刷新。
- 可检索日志名：`record-reader-actions`、`record-reader-merged-browser`、`record-card-cover-merge`、`record-reader-merged-pc`、`record-reader-release`。这批不需要重新研究或重做已完成展示能力，只补后续安装与原生验收。
