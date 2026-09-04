---
version_status: current
reviewed_at: 2026-09-04
requirement_ref: docs/requirements/yilong-quant-android-identity-v1.md
---

# 官方量化 APK 身份切片交付

## 实现范围

主 APK 官方项目安装和打开使用固定包名、当前唯一发布证书、最低版本和显式 Activity。
同名应用及本机旧包名映射不能替代官方身份。其他项目保留原通用安装与打开方式。
官方下载使用应用私有缓存中的唯一文件，写完设为只读、验签，再通过窄范围
FileProvider 只读 URI 交给系统安装器；并发下载不覆盖已验证文件。

本切片不请求 grant、不传主项目 token/ESK 余额、不访问量化私有 API，不改资金或账本。

## 可重复证据

- 主仓基线：`2ca616077f779e64846d530cc593ebfe76bb05b4`。
- 量化已发布身份来源：`a80fbe845b5e307722d38dc189998d98d880e5ec` 的 `AI_CURRENT.md`。
- 2026-09-04 从主服务器公开下载真实量化 APK，`apksigner verify --print-certs`
  验证通过；包名 `com.elon.quant`，`versionName=0.2.0`，`versionCode=2`。
- APK SHA-256：`c17ab5abe800547f41acc95594021abb6cec92fc14cb6de3b2db202ce4b94b89`。
- 当前证书 SHA-256：`019a3d95366fb4c6fe578c1f7f26fb96e462dc54f41b9a7c7b5a715052e418bb`。
- `scripts/test-official-quant-apk-identity.js` 检查真实接线、只读文件、稳定 ID 和无凭据启动，
  含 13 个内存变异检测；源码合同不冒充 Android 运行时验收。
- Android 专项：`OfficialQuantApkPolicyTest`、`ProjectApkSignatureGuardTest`。
- 独立审查发现的校验后外部文件替换问题已修复，并经只读复审确认。

## 状态与后续

| 层级 | 本批状态 |
| --- | --- |
| 实现 | 已接入主 APK 安装与打开调用链 |
| 源码合同 | passed，13 个变异检测通过；官方目录合同通过 |
| Android 单元及构建 | passed，9 项官方策略测试 + 5 项原签名兼容测试，assembleDebug 通过 |
| 主 APK 发布 | published，`1.1.1500 (1500)`，源码 `a8f4f052d77dd462508ee0cc97f9378902729028` |
| 设备安装/打开 | device_deferred，本批不操作物理设备 |
| 本人 ESK 授权与登录 | 未实现；下一独立切片 |

正式包大小 `39,620,962` 字节，SHA-256
`a5d1d36d86d417020e78bde50bfdc2301639d63a0c2faac5698959bfc88e7f92`。
本地签名验证、Manifest 包名/版本及线上 `/app/version.json` 的 SHA/版本/源码一致；
按标准 `publish-apk.ps1` 发布，没有另行发布 Server/API，也未改变量化 APK/PWA 指针。
服务器分配版本号，源码中的 Gradle 版本已由发布脚本恢复。
本轮仅对发布进程使用 `enabled=false` 的临时 ADB 配置，没有修改用户全局配置或设备。
发布后辅助 worktree 清理报告 `Branch` 属性缺失，业务上传成功；最终收尾另经统一 finish 核验。

本机 JDK 17 在默认短路径临时目录下出现 `UnixDomainSockets.connect0 Invalid argument`；
仅对本次构建进程使用隔离工作树内的 ASCII 临时目录后进入正常编译。
未改机器全局 Java/代理配置，也未降低应用网络或签名要求。

后续代理从双方应用绑定协议开始：双向发布证书校验、用户确认、短期一次性 nonce、
仅内存资产投影、会话切换/撤销和失败恢复。当前固定入口是公共启动入口，
不得在这里直接加入 bearer 或把资产注入公共 HTTP WebView。

只读源码复核确认：主 APK `MainActivityState.kt` 的普通 OkHttp 经
`AuthManager.applyAuth` 添加 Bearer，`esk/EskAssetApi.kt` 直接读取 HTTP JSON，
当前没有应用层加密或该响应的签名验证。应用身份 pin 和本机 IPC 不能修复这个网络缺口；
下一轮必须将主站资产传输保护与本机授权分别设计，不能把 APK 已验签称为全链路安全。
