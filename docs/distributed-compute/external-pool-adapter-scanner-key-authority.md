---
title: 外部矿池 Adapter 漏洞扫描器信任根权威
status: current
reviewed_at: 2026-08-12
owners: backend, security, ai-economy
design_status: design_frozen
implementation_status: implementation_partially_verified
---

# 外部矿池 Adapter 漏洞扫描器信任根权威

## 1. 冻结结论

V235 建立独立的漏洞扫描器 RSA 公钥登记、四眼激活、吊销和 currentness。扫描器密钥必须与 V230 供应商制品签名密钥不同；同一公钥不能同时证明“谁发布了制品”和“谁检查了制品”。

V235 只回答扫描报告应由哪一个已登记、当前有效的扫描器身份签名。后续 V236 已消费该信任根并记录 exact-SBOM 签名报告，但仍不运行扫描器、不下载或独立核验漏洞情报，也不授予 conformance、Adapter、route 或派发权限。

## 2. 信任根合同

- 仅接受 2048 至 8192 位 RSA 公钥；
- API 可接收 SPKI 或 PKCS#1 PEM，保存前统一规范为 LF 结尾的 SPKI PEM；
- `key_id` 固定为 canonical SPKI DER 的 SHA-256；
- canonical record、activation 和 revocation 使用 RFC 8785 JCS 与 domain-separated SHA-256；
- 登记者不能激活自己登记的密钥，激活必须由另一名平台管理员完成；
- 吊销只能发生在激活之后，吊销后不能重新激活；
- 供应商签名钥与扫描器钥实行数据库双向角色隔离。

Store 为未来签名报告事务提供不可序列化的 current authority。调用方必须同时提交精确 `key_record_id`、`key_record_digest` 和 `key_id`，且密钥仍为 `active`；历史报告回读可使用不可变 historical authority，但不得借历史密钥创建新报告。

## 3. 数据与权限边界

V235 新增三张不可变表和一个派生 current view：

- scanner key root；
- activation receipt；
- revocation receipt；
- `pending_activation|active|revoked` currentness。

唯一约束、外键、JSON projection、只增不改触发器、四眼触发器和角色隔离触发器共同保护该链。Store 读取时重新验证 canonical JSON、摘要、数据库投影、时间顺序和根引用。

管理 API 仅允许 `admin|owner`：

- `POST /api/admin/compute/external-pool-adapter-scanner-keys`；
- `POST /api/admin/compute/external-pool-adapter-scanner-keys/:key_record_id/activate`；
- `POST /api/admin/compute/external-pool-adapter-scanner-keys/:key_record_id/revoke`；
- `GET /api/admin/compute/external-pool-adapter-scanner-keys/:key_record_id/currentness`。

响应不返回公钥 PEM、幂等 scope 或幂等 key。所有写操作必须显式确认，并对同一管理员和操作使用独立幂等域。

## 4. 明确无效果

登记、激活和吊销收据均固定：

- `vulnerability_report_effect=none`；
- `artifact_security_effect=none`；
- `conformance_effect=none`；
- `adapter_effect=none`；
- `route_effect=none`。

因此“scanner key 已激活”不等于“扫描已执行”“制品无 CVE”或“Adapter 可以运行”。

## 5. 下一硬门卫

下一阶段应新增独立、已签名的漏洞情报报告，至少精确绑定：

1. V233 current artifact security receipt 和 exact V232 package digest；
2. V235 current scanner key root、扫描器运营方和产品身份；
3. 漏洞数据库来源、不可变 snapshot digest、生成时间和有效期；
4. exact dependency graph、扫描策略、扫描结果和原始报告摘要；
5. 签名字节、签名验证及吊销后的历史/currentness 语义。

即使该报告完成，动态恶意行为、隔离 sandbox conformance、credential verifier、安装/采用、Sidecar IPC、route、Worker/ACK、真实派发和结算仍需分别闭环。

## 6. 当前验收

V235 的 migration、Store 生命周期、两连接激活竞争、SQL 不可变、双向密钥角色隔离和进程内 HTTP 鉴权/脱敏均已通过；V230 受影响回归也通过。证据见 [`external-pool-adapter-scanner-key-acceptance.md`](external-pool-adapter-scanner-key-acceptance.md)。

生产数据库原位升级、真实 TCP、外部扫描器、生产密钥托管、MCP/PC、部署和灾难恢复未验证，当前状态仍为 `implementation_partially_verified`。
