# 文章币安广场渠道与Rust证书交付记录

记录时间：2026-09-15，北京时间。本文证明本次交付结果，不定义额外产品需求。

## 能力与验收边界

| 能力 | 实现 | 验证 | 交付 | 用户验收 |
|---|---|---|---|---|
| 文章中心连接币安广场，Android/PC/PWA入口 | 完成 | 编译、浏览器合成账号流程通过 | 已发布 | APK视觉待验 |
| 本人密钥绑定、轮换、解绑、加密存储 | 完成 | 隔离、脱敏及状态转换通过 | 已发布 | 真实密钥待用户绑定 |
| 长文章、文字、四图、单视频发布 | 完成 | 服务端协议和预览测试通过 | 已发布 | 真实币安发帖待验 |
| 定时队列、记录、取消、重试、结果待核实 | 完成 | 幂等、并发、恢复和配额通过 | 已发布 | 真实账号流程待验 |
| Rust ACME签发、热加载和重启恢复 | 完成 | 测试CA、正式CA及在线TLS通过 | 已启用 | 本次线上技术验收通过 |
| 自动续期 | 完成 | 调度、退避、重启状态保持通过 | 已启用 | 真实跨周期运行尚未到期 |

已有群聊文章权限与文章版本继续复用；本次未上线自有公共广场。币安公开接口未提供的读取、编辑、删除、点赞、评论通过官方页面处理，不伪造相关API。

## 工件与生产证据

- APK：v1.1.1767（1767），源提交`7fdc4d07b13bca9d4fff62333af9d4a191ec14b9`。正式脚本验证manifest版本、远端大小和SHA-256：`58903d80f77f9b4063d440aac3350f86b01128218a489e78bd4982c6ddc3252d`。未安装到真实手机。
- PC/PWA与接口首次发布：服务端v0.3.1759，源提交`6f2924106686c1d7b699607f6331eb98d2266ace`。
- ACME修复后的服务端：v0.3.1760，源提交`d994e35444b59c45a2c4055643e5d75d619f67e5`；官方Server完成检查通过。本次后续变化不涉及Android输入。
- HTTPS：`https://43.139.149.158:8443/square`返回200；未登录渠道接口401；HTTP渠道接口404；HTTPS管理接口404；两侧健康检查通过。
- 原生ACME测试签发12.1秒完成，正式签发与热加载64.1秒完成。现有legacy证书保留；确认新证书被客户端正常信任并实际加载后，旧续期定时器关闭。
- 生产证书指纹：`5bd2dc8514beb01b9031ae539806f3ad7ab809cb174bbf7cfdf97f2e53a86108`。到期2026-09-22 10:54:19，续期阈值2026-09-20 10:54:19，均为北京时间。
- 重启后证书、账号凭证摘要及续期时间不变；无重复签发；私有目录0700、文件0600；443验证端口已释放。运行时签发和续期不调用额外证书软件。
- 服务器既有代理路径能够完成到币安官方发帖入口的可信TLS连接；无认证HEAD返回400。此证据不代表真实密钥或真实发布成功。

## 测试与修复

Rust完整服务端测试构建完成，Square专项15项通过；HTTPS独立harness31项通过。Android编译、SquareProtocolTest及正式release构建通过。PC生产构建、桌面/390px手机网页发布流程通过；多图预览按用户选择顺序；丢失回执后的相同请求只生成一条任务。合并上游移动聊天恢复改动后补充相关Node与浏览器回归通过。

首次ACME测试环境失败在临时证书加载阶段。本地真实TLS回归复现`UnsupportedCriticalExtension`；专用验证证书resolver修复后通过本地及线上验收。正式业务证书的信任校验未降低。配置回滚仅恢复本次拥有且未被他人更改的设置，嵌入Python回归通过。

复查入口：`scripts/test-article-square-ui.cjs`、`scripts/test-native-acme-config.py`、`server/tests/account-https-harness/`、`scripts/configure-account-native-acme.sh status`。命令日志保存在项目Git元数据`ai-command-logs`中，本次关键名称为`native-acme-tls-fixed`、`native-acme-fix-publish-server`、`native-acme-staging-fixed`、`native-acme-production`、`native-acme-restart-acceptance`和`square-publish-apk`。

## UI工作台与后续验收

UI任务`desktop_73b5ba6281444d529f7882f5b7ec50d3`返回`DEBUG_RUNTIME_NOT_CONNECTED`。依据UI Skill的发布顺序，已完成业务发布，视觉验证延期；没有触发平台升级或真实设备操作。

```text
FIT_RUN_STATUS=VERIFICATION_DEFERRED
FINAL_VISUAL_LOSS=not_measured
VISUAL_ACCEPTANCE_THRESHOLD=not_evaluated
CROSS_PLATFORM_VISUAL_PARITY=not_visually_accepted
BUSINESS_DELIVERY_READY=false (UI工作台验收字段)
PLATFORM_EVOLUTION_PENDING=false
EVOLUTION_THREAD=none
REAL_DEVICE_STATUS=not_requested
ANDROID_RENDERER=runtime_not_connected
```

功能登记工具在本任务中不可调用，未手改注册表。下一步由用户在应用文章中心绑定自己的Square OpenAPI发帖凭证，选择愿意公开的文章进行真实发布验收；不要在聊天中发送密钥。未使用真实账号发送示例内容，未将功能测试冒充真实币安验收。
