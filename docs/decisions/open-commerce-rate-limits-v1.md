---
title: 开放商业商户可控调用配额 V1
status: accepted
date: 2026-08-01
owners: backend, product
---

# 开放商业商户可控调用配额 V1

## 背景

商户主动发布到开放目录后，第三方 App 和消费者 AI 可以调用其公开或已授权能力。只有身份、授权和审计仍不足以形成可运营的公共入口：错误循环、失控代理或单个 App 的突发请求可能占满商户运行时，并把失败成本转移给商户。

V1 需要让商户直接控制能力的调用节奏，同时保持现有沙盒兼容，不把尚未建设的全网风控描述成已完成。

## 决定

1. 配额归商户项目控制，策略维度为“商户能力 + 指定 App 或全部 App”。
2. 指定 App 的策略优先于全部 App 策略；全部 App 策略按调用主体分别计数，不形成所有 App 争抢一个总额度的隐式全局锁。
3. 采用数据库持久化的固定时间窗和原子 UPSERT。每个策略与主体只保留一个当前计数行，避免随时间窗无限增长。
4. 默认没有策略时继续允许现有调用，避免升级后静默中断已发布能力。商户必须显式创建策略，且可以停用和重新启用。
5. 仅对外部调用执行配额；商户项目编辑者在本项目内调试能力时绕过配额。
6. 幂等重放先读取原调用结果，不重复占用配额。只有已认领的新调用进入限流判断。
7. 超限调用记录为 `failed/rate_limited`，计量单位和金额均为 0，并写入不含原始请求值的审计事件。
8. HTTP 返回 `429 Too Many Requests` 和可读重试时间；MCP 与领域服务保留同一类错误语义。

## 主体规则

- 已注册开发者 App：以稳定 `app_id` 作为计数主体。
- `pc-web` 与 `mcp-client`：按系统入口和用户摘要隔离，避免一个登录用户耗尽所有系统入口额度；摘要不写入项目总览或审计。
- 指定 App 策略只匹配该 App；未命中时再匹配全部 App 策略。

## 安全和隐私

- 配额管理要求项目编辑权限。
- 策略必须引用当前项目商户的真实能力，不能为其他项目写入策略。
- 审计只记录策略 ID、能力键、窗口、上限和重试秒数，不记录请求正文、Token 或用户明文标识。
- 超限失败不会进入业务处理器，也不会生成可收费单位。

## 非目标

V1 不是全网 DDoS 防护、设备指纹、IP 信誉、验证码、生产应用审核、动态风险评分或跨数据库一致的分布式限流。多实例部署若使用不同数据库，仍需要共享计数基础设施或边缘网关；这一点不能由当前 SQLite 策略替代。

## 实现入口

- Schema：`server/src/open_commerce_rate_limit_migration.rs`
- 持久化与原子认领：`server/src/store/open_commerce_rate_limits.rs`
- 领域规则：`server/src/open_commerce_rate_limit_service.rs`
- HTTP 管理接口：`server/src/open_commerce_rate_limit_api.rs`
- AI/MCP 管理入口：`server/src/open_commerce_mcp.rs`、`server/src/open_commerce_mcp_tools.rs`
- 商户工作台：`pc-frontend/src/features/open-commerce/OpenCommerceRateLimitManager.tsx`
- 验收：`docs/open-commerce-rate-limits-v1-acceptance.md`
