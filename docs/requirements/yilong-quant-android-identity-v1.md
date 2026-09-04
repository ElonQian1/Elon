---
version_status: current
reviewed_at: 2026-09-05
owner: android-platform
---

# 官方量化 APK 身份校验 V1

> **当前版本门禁：** 本文最初审核 `versionCode=2` 时建立的是包名、Activity 和发布
> 证书身份基线。当前产品已由[新版唯一公共下载 V2](yilong-quant-android-public-download-v2.md)
> 把最低版本提高到 `versionCode=5`；除历史版本阈值外，本文的身份与启动约束继续有效。

## 目标与范围

作为 [首批用户资产路线图](esk-first-user-delivery-roadmap-v1.md) 的 Android 授权前置切片，
保证主项目项目广场的官方 `yilong-quant` 安装和打开操作指向经过审核的量化客户端。
本批只校验应用身份，不签发授权、不传主项目 bearer、ESK 余额或任何用户资料。
量化公共 HTTP 页面、其他项目的 APK 行为和原有安装兼容性检查保持不变。

## 信任合同

- 官方项目只认稳定 ID `yilong-quant`，不能用显示名称认定身份。
- 包名固定 `com.elon.quant`，打开入口固定 `com.elon.quant.MainActivity`；
  忽略历史包名映射和同名 Launcher，不能回退打开其他应用。
- 发布证书 SHA-256 固定为
  `019a3d95366fb4c6fe578c1f7f26fb96e462dc54f41b9a7c7b5a715052e418bb`。
  来源：量化 Git `a80fbe845b5e307722d38dc189998d98d880e5ec` 的 `AI_CURRENT.md`，
  已发布 `0.2.0 (2)` / `rel_5b6250972a7c45f093ace0a5e82db212`；不是从 HTTP 下载响应学习证书。
- 首次审核的历史最低版本为 `versionCode=2`，只用于说明既有发布证书的来源，不是
  当前兼容承诺。当前最低版本以 V2 的 `versionCode=5` 为准；首次安装也必须校验，
  拒绝不同包名、空证书、不匹配证书、多签、仅历史签名匹配和所有低于当前门禁的
  旧版。轮换须独立审核并更新主 APK。
- Android 28+ 使用 APK **当前内容签名者**，不把历史签名集合交集当成官方身份。
  旧 Android 使用当前 `PackageInfo.signatures`。原通用升级兼容检查仍保留。
- 每次打开重新读取已安装包身份、版本和明确 Activity 的可用/导出状态；
  新建无 extras、无 URI 的显式 Intent，不复用来源不明的 Intent。
- 身份校验失败时停止安装或打开并说明需从官方项目获取正确版本；不得自动卸载应用。

## 验收

1. 官方包+当前唯一受信证书+支持版本可以进入原安装兼容检查，并打开固定 Activity。
2. 同名应用、错误包、空/错证书、多签、只历史命中、旧版均不能作为官方量化应用通过。
3. 非官方项目不受新增 pin 限制；原安装签名冲突测试继续通过。
4. 安装前和打开前均接入校验，不只是新增未调用的 helper；打开不携带凭据或资产。
5. 单元/源码接线合同及 Android 构建通过；APK 发布和设备验收另记真实状态。

## 后续接缝

这不是 APK 登录协议。后续仍需独立的双向应用绑定、用户确认、短期一次性 nonce、
会话切换/撤销处理和只读资产投影；不得给现有公共 HTTP WebView 注入资产或 bearer。
签名 pin 也不证明 ESK 已发行、付款真实、行情准确或应用免于运行时篡改。

Android 签名语义参考：[SigningInfo](https://developer.android.com/reference/android/content/pm/SigningInfo)。
