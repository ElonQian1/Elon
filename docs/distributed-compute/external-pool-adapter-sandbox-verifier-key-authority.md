---
title: 外部矿池 Adapter 沙箱验证者信任根权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 沙箱验证者信任根权威

## 结论

V237 建立独立的沙箱验证者 RSA 公钥登记、四眼激活、撤销和 currentness。它只回答“后续动态沙箱与能力一致性报告应由哪一个当前有效的验证者身份签名”，不运行制品、不生成 conformance 报告，也不授予 Adapter、route、凭据、派发或结算权限。

供应商签名钥、漏洞扫描器钥和沙箱验证者钥必须三方分离。同一公钥不能同时证明“谁发布制品”“谁检查依赖漏洞”和“谁执行动态验证”。

## 信任根合同

- 仅接受 2048 至 8192 位 RSA 公钥；
- API 可接收 SPKI 或 PKCS#1 PEM，保存前规范为 LF 结尾的 SPKI PEM；
- `key_id` 为 canonical SPKI DER 的 SHA-256；
- 根记录和状态转换使用 RFC 8785 JCS、domain-separated SHA-256 与不可变 JSON 投影；
- 登记者不能激活自己登记的密钥，必须由另一名平台管理员激活；
- 只有已激活密钥才能撤销，撤销后不能恢复；
- 服务和 SQLite 触发器同时拒绝三类信任角色复用同一密钥。

Store 为下一阶段签名报告提供不可序列化的 current authority。新报告必须精确提交 `key_record_id`、`key_record_digest` 与 `key_id`，且根仍为 `active`；历史报告可以用不可变 historical authority 重验签名，但不能借已撤销密钥创建新证据。

## 数据与管理 API

V237 新增不可变 root、append-only transition 和派生 current view：

- `pending_activation | active | revoked` 由 activation/revocation 历史动态派生；
- 唯一约束、外键、四眼触发器、状态顺序触发器和 JSON projection 共同保护收据；
- Store 每次读回都会重算 canonical 摘要，并核对结构化列、根引用和时间顺序。

仅 `admin|owner` 可调用：

- `POST /api/admin/compute/external-pool-adapter-sandbox-verifier-keys`；
- `POST .../:key_record_id/activate`；
- `POST .../:key_record_id/revoke`；
- `GET .../:key_record_id/currentness`。

响应不返回 PEM、幂等 scope 或幂等 key。登记、激活和撤销均需显式确认，并使用按管理员和操作隔离的幂等域。

## 明确无效果

所有 V237 收据均固定：

- `conformance_report_effect=none`；
- `vulnerability_report_effect=none`；
- `adapter_effect=none`；
- `route_effect=none`。

所以“sandbox verifier key 已激活”只表示一个签名身份进入可用状态，不表示验证器进程可信、沙箱已运行、六项能力通过或 Adapter 可以安装。

## 下一硬门卫

后续动态证据必须至少绑定：

1. 当前 V233 静态安全收据和当前 V236 漏洞报告；
2. 当前 V237 验证者根、验证者产品和运行策略版本；
3. exact artifact、入口、六能力声明和受控测试向量；
4. 禁网、只读文件系统、CPU/内存/时间限制及实际观测；
5. 每项能力的输入、输出、退出状态和 transcript digest；
6. 签名挑战、不可变收据、有效期与撤销后的历史/currentness 语义。

即使该报告完成，credential verifier、Adapter 采用、Sidecar IPC、v213 route、Worker/ACK、真实外部矿池、计量和付款仍须独立闭环。

## 当前边界

本批已验证 migration 重复执行、HTTP 鉴权、四眼生命周期、幂等重放、三角色密钥隔离、撤销和响应脱敏。未验证生产密钥托管、HSM、真实验证器、沙箱隔离、真实制品执行、生产数据库升级、真实 TCP、MCP/PC、部署与灾难恢复。

验收证据见 [`external-pool-adapter-sandbox-verifier-key-acceptance.md`](external-pool-adapter-sandbox-verifier-key-acceptance.md)。
