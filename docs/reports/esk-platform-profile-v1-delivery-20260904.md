---
title: "正式 ESK 个人入口 V1 交付记录"
version_status: evidence
reviewed_at: 2026-09-04
owners: [platform-assets, android]
---

# 正式 ESK 个人入口 V1 交付记录

需求真源：[个人入口 V1](../requirements/esk-platform-profile-v1.md)。
此报告仅说明交付证据，不替代经济方案、付款事实或上线授权。

## 本批能力

| 能力 | implementation_status | verification_status | delivery_status | acceptance_status |
| --- | --- | --- | --- | --- |
| 个人中心独立正式入口与原生数量/最近流水 | implemented | offline_passed | deployed（APK 1.1.1507） | deferred：实际账户待验收 |
| 严格来源、金额与历史校验 | implemented | offline_passed（32 项） | deployed | pending：仅合成数据验证 |
| 私有传输、会话/请求代次与清除 | implemented | offline_passed（25 项） | deployed | deferred：Android 生命周期实测待补 |
| Web 对应正式来源说明/主 APK 下载入口 | implemented | environment_passed（公开入口） | deployed | pending：不读取正式私有余额 |
| 真实资金、正式入账、双 APK 正式余额 | 非本批实现 | 未操作 | 未启用 | 需独立审批与验收 |

## 源码边界

- 新代码在 `android/app/src/main/kotlin/com/elon/app/esk/platform/`：模型、解析、网络、
  会话、请求门禁、页面、View 和静态个人入口；每个新生产文件不超过 192 行。
- `MainProfileQuickActions` 在旧 Paper 卡挂载后附加入口并去重；不增长 MainActivity。
- `AuthManager.saveSession/clear` 仅各加一行同一 editor 的 UUID 修订，用于识别退出再登录。
- 正式 Activity 非导出、禁止截图、无持久化/Activity result；离开页面立即清除，
  刷新或 auth 变化废弃旧请求，显示至会话过期或最多 60 秒后清除。
- 原有 Paper API、Paper 兑换/卖回和 17 字段双 APK 合同未变。正式数量不得进入其可用余额。
- 网页只增加原生入口说明，不增加正式账户读取；当前 HTTP 显示不可用，不填零或编造余额。

## 验证证据

2026-09-04，定向 Gradle `:app:testDebugUnitTest`：**104 项，0 失败，0 错误**。

| 测试组 | 数量 |
| --- | ---: |
| 平台解析/边界 | 32 |
| 平台独立 reader | 9 |
| SessionStore / RequestGate | 16 |
| 平台 APK/Web 源码接线 | 9 |
| 既有 Paper reader / provider / 17 字段合同 / 登录到期 | 38 |

命令选取 `com.elon.app.esk.platform.*`、`com.elon.app.esk.handoff.*`、
`com.elon.eskcontract.*`、`com.elon.app.AuthManagerExpiryTest`；没有访问生产账户。
首次完整通过 103 项：`esk-platform-jvm-final-20260904-172848-852`；新增修订写入接线断言后
最终 104 项：`esk-platform-final-wiring-20260904-174159-885`（Git 元数据 ai-command-logs）。
`node scripts/check-mobile-pwa-source.js` 通过；Preview-first Views 资源编译通过。

初始本机 JDK 在旧短名 TEMP 路径创建 socket 时失败；仅把本次进程 TEMP/TMP 设为
任务 `.ai-tmp/java-temp` 后编译成功，未修改全局配置。第一次代理内临时启动器错误码
未正确传播，已核对实际 Gradle 日志拒绝其“passed”，后续设置原 wrapper 提供的
`GRADLE_EXIT_CONSOLE=true` 并验证 `BUILD SUCCESSFUL` 与 XML 测试结果。
首轮单测发现 unknown-charset 默认为 UTF-8，最终 reader 已拒绝未知字符集，重跑全部通过。

## UI 工作台与剩余验收

工作台绑定本批隔离编辑根，task `esk-platform-profile-v1`。已执行任务导入、profile、
runtime、capability 和 Views scaffold；无 TARGET_DESIGN，未伪造参考图或像素分数。
能力检测为 PREPARATION_REQUIRED，仅缺真实 runtime 构建验证；不是平台能力缺口。
本批默认不触碰物理手机，APK 发布使用任务本地的禁用自动安装配置。

发布后工作台仍为 BOOTSTRAP/disconnected；未返回可用 rendererResourceId 或空闲租约。
遵照先容量后准备规则，未启动、抢占或安装模拟器（preparation attempts=0）。
门禁显示 PREPARATION_REQUIRED，不能用正式 APK 构建替代工作台真帧证据。

| UI 字段 | 实际状态 |
| --- | --- |
| FIT_RUN_STATUS | NOT_REQUIRED_WITHOUT_CLEAN_TARGET（没有提供目标图） |
| FINAL_VISUAL_LOSS | unavailable |
| VISUAL_ACCEPTANCE_THRESHOLD | unavailable |
| CROSS_PLATFORM_VISUAL_PARITY | deferred；Web 仅原生入口说明，未宣称完整界面一致 |
| BUSINESS_DELIVERY_READY | false（工作台缺 runtime/sourceProof） |
| PLATFORM_EVOLUTION_PENDING | false |
| EVOLUTION_THREAD | none（没有创建平台进化任务） |
| REAL_DEVICE_STATUS / ANDROID_RENDERER | not_requested / verification_deferred |

业务 APK/Web 已发布，视觉验证延期；不宣称视觉或真实用户验收通过。

## 发布回执

- 正式代码：`1314be7148b6563bfbbf3a611d9fa998efd8989d`；模型/传输基础
  `8bbfaf63c179d01855a0b3724f534d01cdb9eb14`。主线并发只有一条无关 Android 文档更新，
  实际 non-fast-forward 后 rebase 无冲突；原测试源内容不变。
- APK：`1.1.1506` / build `1506`，39,665,591 字节，SHA-256
  `ac3302061d2501d17278ca2f1b2bf2d2a00ad50a5bc54d5b4eff5a6a0d4ea5fc`。
- 下载：[主项目 APK](http://43.139.149.158:8080/app/ElonSpeed-latest.apk)。公开 version.json
  返回以上精确版本、Git SHA、大小与哈希，正式 publisher 完成远端文件原地哈希校验。
- PWA 静态模板先发布（7.6 秒），APK 后发布（233.4 秒）；共 242.5 秒，无 Rust 重建。
  根页面 HTTP 200，正式来源说明、主 APK 下载和 HTTP 不可用提示均检出。
- 日志：`esk-platform-ui-publish-20260904-173541-375`。自动 ADB 安装明确 disabled；
  publisher 的附带 worktree 清理出现 Branch 属性 warning，后续统一 finish 单独验收。
- 发布后把 Web 下载链接复用既有块级 `profile-row`（76px 触控），不改变 Android 生产输入；
  发布器将 Android 单测也纳入输入范围，因此按正式流程重新封装，并未绕过来源检查。
- **最终版本**：APK `1.1.1507` / build `1507`，来源
  `3012c5fbd717bd8db862590fe5c240a6099c9225`，39,665,591 字节，SHA-256
  `be960d17660e84ffb5498b39292febbf5bfc3f7dcd607da666fdfa70e35562ab`。
  公开 version.json、实际本地文件哈希和正式签名均核对通过；签名证书 SHA-256
  `f79567cf8a7e610e218aa4b7a1292be93a9623d9bc06a9bafbf47b030f99010c`。
- 最终 PWA 静态模板与 APK 两阶段均 passed，日志
  `esk-platform-final-publish-20260904-174548-451`，耗时 310.9 秒。
  公网页面 HTTP 200，再次检出正式入口、块级下载、原生私有说明与 HTTP 不可用提示。

本批按路由读取约 19 份入口、需求和领域规范（粗估约 24k 文本 token）；未全读
docs、Prompt、Agent、Skill 目录，未把历史讨论或报告提升为需求真源。

安全传输仍待用户选择；未安装证书、未更改 HTTP 服务，也未读取真实登录凭据。
历史用户仍需核对付款、映射、用途和审核后才可登记。当前量化 V24 的正式签名 APK
上传及双 APK 本人授权待独立发布凭据与安全来源；这不是本批新页面已经接通的能力。

主服务平台账本前置版本为 `0.3.1724` / `5fc7869b5b2560417af26b33f0b09ca749fc9bb1`，
本批再次只读核对公开版本一致。量化行情 V25 由独立任务发布，不在此重复部署。

## 后续代理入口

以 Feature Registry `esk-platform-profile-v1` 和需求哈希认领，不依据聊天猜已完成状态。
下一步优先补真实已授权账户的读取/换号/失效验收，以及量化正式来源的新版本合同；
不得放宽 Paper 协议或在当前 HTTP 公共量化通道转送主账户 token。
