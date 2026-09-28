---
version_status: current
reviewed_at: 2026-09-28
implementation_status: implemented
---

# 微信视频返回时一龙被最小化

## 已确认根因

小米安装版 `1.1.1823`：从群聊视频卡片打开微信双开选择器，选择分身播放视频，
返回时一龙出现约 100ms，随后整个任务退到桌面。不是群聊状态丢失或 APK 主动退出。

2026-09-28 系统日志的关键顺序（不保留视频地址、聊天正文或凭证）：

1. `XSpaceResolveActivity`、微信协议入口、中转页、视频页都加入原一龙任务。
2. 13:33:30.659，`ActivityTaskManager` 记录微信分身进程调用
   `moveActivityTaskToBack`，被操作任务的 affinity 是 `com.elon.app`。
3. 视频页和 `BizComSchIntermediateUI` 结束后，一龙于 13:33:30.735 恢复。
4. 13:33:30.840，桌面恢复，一龙随同任务被最小化。
5. 12:55 的同一路径也出现此序列。因此只检查一次 `MainActivity` resumed 会误判通过。

## 修复范围

共用的 `WechatChannelsHandoff.intent` 仅添加 `FLAG_ACTIVITY_NEW_TASK`，让微信
使用自身任务而非一龙任务。群聊卡片和阅读器跳转复用该方法。

- 不添加 `CLEAR_TASK`、`CLEAR_TOP` 或 `MULTIPLE_TASK`，不清理微信原有历史。
- 不指定双开用户，保留系统选择器。
- 不修改群聊导航、草稿、滚动位置或站内阅读器的返回。
- 不延时抢回前台，不拦截全局返回，不增加悬浮窗或无障碍权限。
- 固定微信包、可信来源、新鲜链接和参数校验保持不变。

依据：[Android 任务与返回栈](https://developer.android.com/guide/components/activities/tasks-and-back-stack)。

## 验收边界

代码已实现；离线验证通过，小米微信分身原问题路径已通过用户操作及 ADB 交叉验收。

- 旧代码运行新增测试：2 项中 1 项失败，外部启动 flags 为 0；站内阅读器测试通过。
- 修复后 `WechatChannelsTaskTest` 2 项、`WechatChannelsPolicyTest` 2 项、
  `SocialLinkCardInteractionTest` 7 项全部通过（11/11）。
- `check-source-size.ps1` 与 `check-document-modularity.ps1 -Staged` 通过。

`WechatChannelsTaskTest` 检查实际启动 Intent 的独立任务标志、固定包、原链接与
浏览器类别，以及站内阅读器仍不创建新任务。此测试不模拟微信或 OEM 任务管理器。

验收检查微信视频页与一龙的 task ID 不同、返回后持续前台，不能只看瞬间 resumed。
本轮确认微信分身原问题路径；未扩大到微信本体、取消选择器或其他 ROM，
不把单一路径通过当成所有设备均已验收。

## 正式交付

- 源码：`a83f9d730da83638c902e907c056475d4b437807`，已推送 `origin/main`。
- APK：`1.1.1825` / `1825`，统一发布脚本构建、签名、上传成功。
- SHA-256：`3b6c38813f712d9be7f18d0c6d2561ed5546ce053abb164c940cb714b537a787`。
- 登记的小米已通过无线 ADB 覆盖安装，回读版本一致，MCP 页面绑定正常。
- 登记的荣耀离线；未清除应用数据、登录态或微信历史。
- 安装结束时停在桌面不属于返回验收。

## 真机验收通过

用户在原“双开选择器 -> 微信分身播放 -> 返回”路径测试后确认“没有问题了”。
测试时手机已更新到后续主线包 `1.1.1826` / `1826`，包含上述修复，版本回读一致。

- 14:28:38，一龙 `MainActivity` 位于任务 `2934`。
- 14:28:43，双开选择器位于独立任务 `2935`。
- 14:28:44，微信分身的协议入口、中转页和视频页位于独立任务 `2936`。
- 14:28:47.463，退出视频后恢复一龙任务 `2934`；至 14:29:17 的后续事件无桌面恢复，
  后续任务栈读取仍显示一龙 `topResumedActivity`。
- 原先“约 105ms 后又最小化”的现象未再出现。只记录结构化系统事件，不保存视频
  私有地址或群聊正文；没有使用循环拉前台掩盖问题。
