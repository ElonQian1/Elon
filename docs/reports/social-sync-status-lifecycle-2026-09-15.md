---
version_status: report
reviewed_at: 2026-09-15
implementation_status: released
---

# APK 同步提示残留修复

## 根因

`SocialChatStatus` 将空列表同步提示添加到共用的 `chatListFrame`。`MainSocialAiChatFeature.activateChatProvider` 通过 `suspendForExternalChat` 暂停好友读取并交接 AI 消息界面，但 `stopPolling` 取消请求后未清除该提示。被取消的回调因代际校验不会再回来关闭提示，因此文字可以长时间覆盖已经显示的 AI 消息。

首次同步未完成时插入待发消息，普通发送和转发也缺少同步提示清理；群聊具有相同暂停及待发路径。此问题属于状态归属和生命周期，不应通过等待超时或隐藏所有错误提示解决。

## 修正范围

- 好友和群聊暂停时同时取消读取并清理空列表提示。
- 本地待发消息插入后立即清理提示，后续后台刷新不覆盖已有消息。
- 共用提示入口拒绝在非空列表上显示空态；隐藏时不创建新视图，并清除重试回调。
- 空列表同步失败仍可重试；缓存、权限、取消迟到响应及发送不自动重试策略保持不变。

## 验证与交付

- 修复前运行新增 7 项回归，6 项按预期失败：AI 聊天交接、普通待发、转发、群聊暂停、重试后暂停和共用空态入口；正常同步用例通过。
- 修复后 `SocialChatStatusTest` 7 项、`SocialChatReadChannelTest` 3 项、`SocialChatRecoveryTest` 4 项全部通过，合计 14 项、0 失败。覆盖真实绑定视图及好友控制器的交接、读取取消、迟到响应、同步成功、后台刷新、失败重试和待发消息。
- 测试采用 Robolectric，使用隔离账号及本地 HTTP interceptor，不向生产发送消息。未进行用户账号或物理设备现场验收。
- 测试日志：`social-sync-status-red-20260915-193557-690.result.json`（回归复现）及 `social-sync-status-green-20260915-194031-985.result.json`（修复后通过）。日志保存在共享 Git 目录的 `ai-command-logs` 下。
- 正式 APK **1.1.1768 / build 1768** 已通过 `scripts/publish-apk.ps1` 发布；Release 构建成功，安装包 manifest 与分配版本一致。
- 发布来源：`87cb47386fc87ddb4f8bff7d314dadfe815df85c`。首次推送遇到并行服务端提交 `d994e3544`，按流程 rebase；Android 输入与测试时完全相同。
- 线上 `/app/version.json` 的版本、源码 SHA 和 SHA-256 与本次安装包一致，下载 HEAD 为 200，文件大小 40,649,993 字节。
- APK SHA-256：`aaf002fc5f999e9219e40426ebd271aa08a4eb2ba738035b401171eb979f7671`。
- 发布日志：`social-sync-status-publish-apk-20260915-194751-386.result.json`，退出码 0。物理设备未配置自动安装目标，本次实机复核记为 deferred；不能据此宣称用户手机已更新或现场验收通过。
- 同步版本仅影响 APK：本轮没有修改 Windows 或 PWA 源码，也没有重新发布它们。
