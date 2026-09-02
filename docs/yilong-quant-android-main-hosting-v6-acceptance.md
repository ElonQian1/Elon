---
version_status: current
reviewed_at: 2026-09-02
delivery_status: code_verified
---

# 一龙量化 Android 主服务器托管 V6 验收矩阵

| 能力 | 实现 | 验证 | 交付 | 验收 | 当前证据 | 剩余缺口 |
|---|---|---|---|---|---|---|
| 主 APK 显示本人 ESK Paper | implemented | integration_passed | pushed | pending | Android ESK 卡片与 V15 双仓余额回归 | 承载版本正式 APK 尚未核对线上发布身份 |
| 项目广场安装/更新/打开 APK | implemented | offline_passed | pushed | pending | 既有 Android 安装器与签名冲突保护 | 量化正式 APK 尚未进入真实目录 |
| 主服务器公开返回受管量化 APK | implemented | offline_passed | not_started | pending | `yilong_quant_android_managed_release_payload_requires_exact_size_and_sha256` 通过 | Server 发布与真实 APK 下载尚未完成 |
| 重启后保留最新 release | implemented | offline_passed | not_started | pending | `yilong_quant_android_official_catalog_reapplies_latest_release` 通过 | 生产重启与目录取回尚未验收 |
| 量化正式 APK 工件 | partial | deferred | not_started | deferred | 子项目 V16 Android 源码批次 | 正式 HTTPS、签名、干净提交和上传回执缺失 |
| 打开量化 APK 查看公开 Paper 结果 | partial | deferred | not_started | deferred | 子项目 PWA、Paper runtime、回测和受限 WebView | 量化 HTTPS 环境及设备验收未完成 |
| 独立量化 APK 查看本人 ESK/仓位 | missing | deferred | not_started | deferred | 主 APK 资产页继续可用 | 后续一次性授权码/应用绑定协议 |

当前批次不会把 `pushed`、Debug APK 或离线 fixture 写成 `published`/`deployed`。

代码回归：`scripts/validate-rust.ps1 ... yilong_quant_android_` 通过 2 个定向测试；`scripts/test-official-project-catalog.js` 通过。为恢复该基线的测试编译，`quant_paper_access` 测试改用既有只读公钥方法，不扩大运行时能力。
