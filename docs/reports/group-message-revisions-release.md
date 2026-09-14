# 群聊消息修订发布记录

需求：[群聊文字编辑与修改记录](../requirements/group-message-revisions.md)。
验证：[测试及验收边界](group-message-revisions-validation.md)。

## 发布结果

| 交付物 | 发布版本与来源 | 核对结果 |
| --- | --- | --- |
| 后端 | 0.3.1752；`3ef6654dc7c18c7871618f408808aca655d2d983` | 版本接口返回 `status=ok`，发布脚本健康检查通过 |
| Windows 前端 | `3ef6654dc7c18c7871618f408808aca655d2d983` | PC 生产构建和包体门禁通过，线上发布标记一致 |
| 网页/PWA | 与后端同一提交 | 线上修订脚本 SHA-256 与仓库文件一致，历史接口未登录返回 401 |
| APK | 1.1.1744 / versionCode 1744；同一提交 | 发布成功，线上版本清单来源一致 |

APK SHA-256：`ac940c3988f9e4db914fa3ed4eede975b61867f11c1383338ee1a644be3c42e6`。

下载：[最新版 APK](http://43.139.149.158:8080/app/ElonAI-latest.apk)。该地址随以后发布更新，历史版本身份以上述版本、提交及哈希为准。

## 使用方式

APK 升级后，长按本人群聊文字选择“编辑”；Windows 和网页端使用消息下的“编辑”。保存后显示“已编辑 · N 次”，群成员点击即可查看完整历次原文与相邻版本差异。发生版本冲突时保留草稿，核对最新文字后才能再次保存。

普通聊天草稿、原发送时间和附件保持原样。历史追加保存；撤回后按已有规则隐藏公开正文与历史，服务端审计记录保留。

## 视觉及设备验收

PC/PWA 浏览器交互通过，覆盖桌面和 390px 手机宽度；APK 专项测试与合并后的群成员交互回归通过。没有安装或操作用户物理手机。UI 工作台能力未缺失，但 `PATCH_FREE_BUILD_VERIFY` 仍为 `PREPARATION_REQUIRED`，设备清单没有可用独立模拟器，视觉验收延期；不把生产构建成功当作真机画面验收。

```text
FIT_RUN_STATUS=VERIFICATION_DEFERRED
FINAL_VISUAL_LOSS=not_measured
VISUAL_ACCEPTANCE_THRESHOLD=not_evaluated
CROSS_PLATFORM_VISUAL_PARITY=source_aligned_runtime_deferred
BUSINESS_DELIVERY_READY=false
PLATFORM_EVOLUTION_PENDING=false
EVOLUTION_THREAD=none
REAL_DEVICE_STATUS=not_requested_not_touched
ANDROID_RENDERER=none_available
BUSINESS_STATUS=published
```

仓库任务工作区和本机 main 的最终状态以统一收尾回执为准。
