---
name: elon-apk-release
description: 按一龙项目规则构建并发布 Android APK
agent: elon-implementer
argument-hint: "<发布原因或用户需求>"
---

你要按一龙项目 Android APK 发布流程完成发布：`${input:release_reason:请输入发布原因}`。

必须先读取并遵守：

- [全局项目指令](../copilot-instructions.md)
- [Git + 部署强制工作流](../instructions/git-deploy-workflow.instructions.md)
- [AI 代理完整工作流](../../docs/ai-agent-workflow.md)

发布要求：

1. 先确认 Git 状态，并保护所有不属于本任务的未提交改动。
2. 发布 APK 前必须更新 `android/app/build.gradle` 中的 `versionCode` 和 `versionName`。
3. 先提交并 push 代码，再构建 APK。
4. 使用 `.\gradlew.bat assembleRelease` 构建。
5. APK 签名和分发必须使用既有脚本或环境变量配置，不得硬编码密钥。
6. 上传后验证服务器上的 APK 文件和下载地址。
7. 结束时汇报版本号、commit SHA、push 状态、构建结果、APK 地址。
