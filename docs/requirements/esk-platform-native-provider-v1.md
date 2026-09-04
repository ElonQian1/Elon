---
title: "正式 ESK 原生摘要授权提供端 V1"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, android, quant-integration]
---

# 正式 ESK 原生摘要授权提供端 V1

## 本批范围

为后续量化原生正式资产页交付主 APK 提供端。独立 action、协议与显式同意 Activity，
不修改旧 Paper 17 字段协议，也不向旧量化页面发送正式数量。
复用 `esk/platform` 的正式本人读取、严格解析、原子会话快照和请求门禁。
唯一数据源仍为 `GET /api/me/assets/esk/platform?limit=20`，只投影总量与登记笔数。

本批不是双 APK 联调完成，不创建量化账号映射，不提供持续登录或远程授权服务。
量化接收端、签名 APK 上传、真实本人联调独立交付。主项目完整流水页保持独立。

## 安全与语义

- 新协议固定 `yilong.esk.platform_android_snapshot.v1`；来源只能 `platform_recorded`，
  `not_deployed`、`simulated=false`、`funds_moved=false`，全部五项操作能力固定关闭。
- 返回精确六位数量、最小单位与登记笔数；不伪造可用余额、占用、净值、汇率、收益或 ledger revision。
- 不返回账户标识、昵称、登录凭据、付款证据、分录或原始服务响应。Nonce 仅关联本次请求。
- 只接受官方量化包当前固定签名、版本至少 4、独立非别名原生组件
  `com.elon.quant.assets.platform.EskPlatformAssetsActivity`；调用方由操作系统提供。
- 主提供端独立 exported Activity，不设 intent-filter；拒绝重建、别名、转发、重定向式 Intent。
  用户必须核对当前主账户并点击“确认并读取正式摘要”，没有自动同意或后台读取。
- HTTPS origin 检查必须先于读取会话及凭据；遵循现有默认系统证书验证，不配置证书、
  不降级 HTTP、不改变公共行情和下载入口。HTTP 只显示不可用，不显示零余额。
- 同意前后、请求完成、结果返回前均校验前台状态、官方调用方和同一会话修订；
  A→B→A、会话过期、超时、退出和后台均取消。每次网络读取最长 15 秒。
- 请求窗口 120 秒；摘要使用本机单调时钟，显示期限大于 0 且最多 60 秒，并受
  已知主会话剩余有效期约束。不可用 epoch 时间替代单调时间。
- 一次性结果返回后主端无法远程撤回：量化端必须标明“本次主项目确认账户的临时摘要”，
  不声称等同量化网页登录用户或实时余额；离页、超时、重建时清空，不持久化、不进入 WebView。
- FLAG_SECURE、禁状态恢复、取消时清空账户标签；确认按钮拒绝遮挡触摸，不输出私有日志。

## UI 与网页

沿用现有轨道深色资源、可滚动原生布局、至少 52dp 且可换行的操作按钮。
明确平台登记、尚未上链、非付款自动核验，不以法律股权、固定收益或随时兑现描述本切片。
网页仅同步原生能力及接收端待接入的说明，不模拟 Android 身份校验或跨应用结果。

## 验收

1. 新协议严格键、类型、金额、计数、来源、能力、Nonce 和时间窗口回归；Paper 互相拒绝。
2. 官方组件、当前签名、版本、启用状态和别名拒绝；请求结构严格验证，结果仅白名单字符串。
3. 正式 reader/session/gate 回归通过；来源读取仅发生在有效显式同意后，过期及重复回调不返回。
4. APK 编译、正式发布和公开身份检查；无 Renderer 真帧则只记录视觉延期。
5. 文档及 Feature Registry 分别记录提供端发布、量化接收端、安装联调与真实账户验收状态。

注册 ID：`esk-platform-native-provider-v1`。共享协议见
[Android 正式摘要合同](../contracts/esk-platform-android-snapshot-v1.md)。
