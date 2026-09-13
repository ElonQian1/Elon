---
version_status: current
reviewed_at: 2026-09-14
implementation_status: implemented
acceptance_status: partial
---

# 量化公开工作区 HTTPS 发布证据

对应[正式需求](requirements/quant-public-https-v1.md)。服务端 **0.3.1745**，源码 `b633c4bac3e66626fffd179f984ef97f2569e2bd`，已经正式 `publish-server.ps1 -SkipPcFrontend` 构建、部署并通过版本回读；只变更服务器，未重启 Win、Chrome 或手机应用。

## 实现和验收

- `QUANT_PUBLIC_HTTPS_ENABLED=true` 已使用一次配置比较交换启用。共用原账户 TLS 的 8443 和证书，账户权限不变；443 继续留给既有 ACME TLS-ALPN 续期。
- 主服务、量化 API 和证书续期 timer 均为 active。当前证书有效期为 2026-09-10 21:26:04 至 2026-09-17 13:26:03 UTC，IP SAN 匹配，客户端使用默认信任验证，没有忽略证书错误。
- 2026-09-13 20:17:54 UTC：HTTPS `/quant/?app_section=market`、两份静态资源和 `/quant/api/v1/runtime` 均返回 200；页面有 CSP 与传输标记，资源有 immutable 缓存头。
- 公开 BTCUSDT 1h 查询返回正确 schema、只读来源、240 根实际 K 线；来源为既有 `binance-spot/public-rest`，查询后观察年龄约 1746ms，缓存年龄 0。此为单次真实读取，不证明持续行情或其他标的全部可用。
- `/api/nodes`、`/mcp`、带 query 的 `/api/me`、私有网格与 Paper 订单路径均返回 404；无登录的 `/api/me` 返回 401。没有发送真实订单、账户授权或资金操作。

## 测试与资源边界

生产路由直接引用的独立 Rust harness **25 项通过**；配置比较交换与回滚 Python **4 项通过**，shell 语法及 plan 通过。完整 musl 正式构建成功，发布脚本回读版本与 SHA 匹配。已有编译警告未在本批扩展处理。

早先完整 Windows server 测试因 C 盘空间不足在 LLVM 阶段失败。确认无活动写入并审阅工具计划后，仅通过受管 GC 清理 C 盘一个注册缓存，恢复约 4.52GB；专项验证和正式发布使用已存在的 D 盘受管缓存，没有修改门禁、清理源代码或其他工作区。

独立工件组 `quant-public-https-v1` 保存发布结果、25 项测试原始结果、配置启用日志及线上结构化回执。回滚命令为配置工具的 `disable`；失败自动恢复开关已通过离线测试，未在生产注入失败或演练二次关闭。

## 明确待交付

线上仍服务既有量化网页 `d22d6fc9919ef63a7ccc7c92e6f92d3cba1f2e63`；新网页的传输提示修正、APK HTTPS origin、原签名发布及装机归量化 `quant-https-consumer-v1`。不得将此次服务端 HTTPS 验收称为 APK 已更新或完整商业上线。

当前 ADB 只发现模拟器，未发现手机/mDNS 服务；没有新装机、浏览器像素、连续断线恢复、证书自动续期或人工交易验收证据。大 Goal 保持 active。
