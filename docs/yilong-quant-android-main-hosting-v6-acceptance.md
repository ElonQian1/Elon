---
version_status: current
reviewed_at: 2026-09-03
delivery_status: published
---

# 一龙量化 Android 主服务器托管 V6 验收矩阵

| 能力 | 实现 | 验证 | 交付 | 验收 | 当前证据 | 剩余缺口 |
|---|---|---|---|---|---|---|
| 主 APK 显示本人 ESK Paper | implemented | integration_passed | published | device_deferred | 线上主 APK `1.1.1413 (1434)` 已覆盖当前 Android 构建输入 | 本轮未在真实设备复查资产页 |
| 项目广场安装/更新/打开 APK | implemented | integration_passed | published | device_deferred | `yilong-quant.latest_apk_url` 非空；公开下载大小与摘要匹配 | 真实设备安装/打开待用户验收 |
| 主服务器公开返回受管量化 APK | implemented | production_passed | published | accepted | Server `v0.3.1721 / 725f91f0a`；真实下载 `200`、`618356` 字节、SHA-256 匹配 | 无 |
| 重启后保留最新 release | implemented | production_passed | published | accepted | 移除临时发布凭据并重启后，项目目录仍返回同一下载地址 | 无 |
| 主服务同栈托管 `/quant/` | implemented | production_passed | published | accepted | Axum 页面、安全头、静态缓存、四个公开接口与敏感路径 `404` 均经公网验证 | 无 |
| 量化正式 APK 工件 | implemented | integration_passed | published | offline_verified | `com.elon.quant 0.1.0 (1)`、子项目 `f2d9690`、独立 v2 签名及服务器发布回执一致 | 设备安装证据待补 |
| 打开量化 APK 查看公开 Paper 结果 | implemented | network_passed | published | device_deferred | `/quant/`、runtime、研究快照和确定性回测已公网通过 | WebView 真机打开待用户验收 |
| 独立量化 APK 查看本人 ESK/仓位 | missing | deferred | not_started | deferred | 主 APK 资产页继续可用 | 后续一次性授权码/应用绑定协议 |

当前批次不会把 `pushed`、Debug APK 或离线 fixture 写成 `published`/`deployed`。

代码回归：`scripts/validate-rust.ps1 ... yilong_quant_android_` 通过 2 个定向测试；`scripts/test-official-project-catalog.js` 通过。新增 `quant_http_preview` 主服务定向测试通过 2 项，证明路由只开放健康状态、运行时、研究快照和回测，敏感量化路径不进入主路由。为恢复原基线的测试编译，`quant_paper_access` 测试改用既有只读公钥方法，不扩大运行时能力。

当前测试部署固定为主项目 Rust/Axum `http://43.139.149.158:8080/quant/`，量化 API 只监听 `127.0.0.1:8787`；不使用 Nginx。HTTP 入口只展示公开 Paper 研究数据，不接收账号、ESK 投影、交易所密钥或真实资金操作。量化 APK 发布回执为 `rel_96e1226e37bd43ada6a3944a4c09f0b0`，SHA-256 为 `12daac597cceeef131194928b8b07a93722bc7f0fd8c38bb8a4c8dfb792133ca`。
