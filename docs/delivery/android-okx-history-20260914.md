---
version_status: current
reviewed_at: 2026-09-14
implementation_status: implemented
verification_status: partial
delivery_status: published
---

# 欧易历史网格只读主端交付

实现[历史托管需求](../requirements/android-okx-history-read-v1.md)。新增 capabilities_v2、history_v1 和独立历史 JSON schema；旧 V1 运行列表、详情、授权和撤销保持兼容。量化拥有历史页，主 APK 没有扩建网格业务 UI。

固定官方历史 GET，每页 50 条、规范 after、非空短页继续、空页末尾；重复、逆序游标、非 stopped 和过量拒绝。读取前后复用现有实际账号/平台会话/授权代次核验、网络互斥与固定传输，不导出凭据。stopped 是已停止，不能据此声称已平仓或净收益已经对账。

主端欧易 25 项单测通过，本批新增 7 项；覆盖分页合同、原 V1、撤销与两层换号。正式 1.1.1714 / 1714 已由官方发布入口发布，源码 `c00060c6db7beda42c3aa704b3b94730126fa3f1`；服务器 APK 大小/摘要与响应核验通过。本地 APK SHA-256 为 `04bfd63276a46677d1a222d930f9fa116c341f2b55d7cbd888d5fe8f2c75bec0`，原证书 SHA-256 为 `f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`。量化配套 0.7.56 已发布。

手机离线，真实账户、跨 APK 调用和双包安装待验，不把本地测试当成实际账号验收。未重启 Win/Chrome 或清理登录；发布配置关闭自动安装，避免占用其他模拟器。发布末尾自动清理报告缺少 Branch 属性，发布已成功，工作区保留并由统一收尾另核对。
