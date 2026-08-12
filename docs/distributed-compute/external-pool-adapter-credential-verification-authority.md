---
title: 外部矿池 Adapter 凭据独立签名验证回执权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 凭据独立签名验证回执权威

## 1. 目的

V243 证明独立 V241/V242 验证器曾对一份精确 external-pool 非 Bearer 凭据执行解析和上游认证，并签署了一份短时有效报告。平台服务端不自行声称访问过外部矿池，而是派生不可篡改的挑战、验证独立验证器签名，并保存可重放审计的不可变回执。

该回执只是一项采用前证据，不安装 Adapter，不激活 Provider，不创建 v213 route authority，不执行任务，也不产生计量、结算或付款。

## 2. 精确绑定

签名挑战同时绑定：

- V221 onboarding application ID、摘要、应用时间及当前未变化的 registering Provider；
- Provider 身份、所有者、结算账户、策略版本、摘要和精确 Adapter 配置；
- 非 Bearer 凭据定位符的域隔离 SHA-256 commitment 与 vault_ref 或 gateway_ref 类型，原始定位符不进入 HTTP；
- V222 当前 staged admission、制品实现摘要、能力集摘要及预期验证器声明；
- V241 验证器实现和 V242 RSA 公钥的精确记录坐标；
- 独立报告 ID、运行时间窗、到期时间、两项通过结果及上游响应证据摘要。

只有 Adapter ID、release version 和 V222 预期验证器与 V221/V241/V242 完全一致时，服务端才生成挑战。报告运行最长 10 分钟，生成延迟最长 5 分钟，有效期最长 60 分钟；创建时已经过期或任一结果不是 passed 均失败关闭。

## 3. 历史与当前性

每次验证形成一条独立追加式回执。回执、报告 ID 和幂等坐标不可更新、删除或通过 INSERT OR REPLACE 替换。历史读取会重新：

- 审计 V221、V222 和 V242 历史根；
- 从 V221 原始定位符重新计算 commitment；
- 重建签名挑战并使用历史 V242 公钥重新验签；
- 对照数据库列投影和规范化回执摘要。

回执仅在以下条件全部满足时为 verified_current：

- V221 application 仍精确，Provider 仍是相同 registering 版本；
- V222 admission 仍为精确 staged；
- V241 实现和 V242 公钥仍 active；
- 报告尚未到期。

任一条件失效后回执保留为 historical_only。后续采用流程必须在自己的写事务内重新取得私有 current authority，并携带精确回执 ID 和摘要，不能长期持有序列化 DTO 代替当前性检查。

## 4. 管理接口与脱敏

仅平台 admin 或 owner 可调用：

- POST /api/admin/compute/external-pool-adapter-credential-verifications/challenge
- POST /api/admin/compute/external-pool-adapter-credential-verifications
- GET /api/admin/compute/external-pool-adapter-credential-verifications/:receipt_id/currentness

挑战响应包含待签名消息，但不包含原始凭据定位符。写入和查询响应不返回原始签名、公钥 PEM、凭据提示、幂等材料、bearer、token 或 secret，只返回 commitment、签名摘要、证据摘要、精确根坐标和当前性状态。

## 5. 后续边界

下一阶段应建立 Adapter adoption/install 事务：同时消费当前 V243 凭据回执、V239 沙箱符合性及完整 V227-V242 制品安全链，生成新的 Provider 版本和可撤销安装权威。V243 不应直接接入 worker、派发、路由或结算。
