---
name: elon-apk-release
description: 按一龙项目规则发布 Android APK
agent: elon-implementer
argument-hint: "<发布原因或用户需求>"
---

发布 Android APK：`${input:release_reason:请输入发布原因}`。

先读 [AGENTS.md](../../AGENTS.md)，只加载 Android 发布相关路由。

- 遵守 `WF-START` 至 `WF-REPORT`；发布只使用 `publish-apk.*`，不手工处理版本、签名或上传。
- APK 版本由服务器 claim/finish 分配，不产生 release-only commit。
- 若被更新主线接管，按脚本结果汇报，不 rebase 追车或重复发布。
- 发布验证完成后使用 `AndroidFeature` 统一收尾；只同步代码时使用 `CodePushed`。
- 最终报告版本、源码 SHA、服务器校验、下载地址和全部统一收尾状态。
