---
title: "ESK Paper 量化申请授权、接收回执与释放 V3"
status: accepted
implementation_status: implemented
owner: platform-assets, quant-integration, pc
priority: p0
reviewed_at: 2026-09-02
---

# ESK Paper 量化申请授权、接收回执与释放 V3

## 用户结果

登录用户在“一龙量化交易”项目主页选择本人一个仍为 `submitted` 的 ESK Paper 量化分配申请，点击安全进入后，由主项目把同次 grant、ESK V2 只读投影和单申请短期授权只交给 exact-origin 量化页面。量化端接收或释放模拟 binding 后，把独立签名回执交回父页面；主项目验签并追加原申请状态，用户随后在资产卡看到 `已被量化 Paper 接收` 或 `已释放`。

## 与 V2 的关系

本需求只扩展 `docs/requirements/esk-paper-quant-allocation-request-v2.md` 的追加式状态机和跨项目启动合同，不改写 V2 请求、取消或余额历史：

```text
submitted -> canceled
submitted -> accepted -> released
```

- `submitted`、`accepted` 继续计入量化申请占用；
- `canceled`、`released` 不计入占用；
- 只有 `submitted` 可由用户直接取消，accepted 必须从量化端释放并回传有效签名回执；
- 历史事件、请求金额和风险版本不得更新或删除。

## 非目标与安全边界

- 全过程固定 Paper：不移动资金、不发行链上 ESK、不连接 sandbox/testnet/live。
- accepted 只证明量化端建立 `esk_paper_allocation_binding` 模拟记录，不证明入金、成交、QSHARE、NAV、收益或可提现资产。
- 不把 ESK 转换为旧 NET，不调用量化历史 NET 参与账本。
- 不导入真实用户、付款、钱包或 KYC 数据；主项目不向量化端发送用户 ID、邮箱或 bearer 会话。
- 不在生产环境自动为真实用户登记余额，也不在量化公网环境尚未批准时伪造可用状态。

## 单申请授权

主项目新增 `yilong.esk.quant_allocation_authorization.v1` / `yeqa1` 签名合同，复用当前 Paper grant 的主项目 Ed25519 签名 key 和脱敏 participant，但使用独立 schema/prefix。授权必须：

1. 只为当前登录用户的 `submitted` 请求签发，并绑定 request ID、精确金额、基础单位、请求修订和风险版本；
2. 与同次 grant 的 grant ID、participant、key、签发/到期时间精确一致，最长五分钟；
3. 固定 `simulated=true`、`funds_moved=false`、`quant_units_issued=false`；
4. 仅在量化 ready capability 明确声明授权 V1 且用户选择请求时发送；普通只读启动保持兼容；
5. 只存在服务响应、父页面内存、postMessage 和量化单次 API 请求，不写 URL、浏览器持久存储、日志或主项目数据库。

## 量化回执验证

主项目新增只验公钥的 `yilong.quant.esk_allocation_receipt.v1` / `yqar1` verifier。量化签名 keyring 与主项目授权 key 完全分离，支持 1–8 个 active/retiring/revoked 公钥和签发时间窗；配置缺失、混用或无 accepting key 时回执同步失败关闭，但不影响既有只读启动。

回执必须绑定当前用户派生 participant、请求 ID、精确金额、binding ID、授权 ID、事件、修订、前序摘要和 Paper-only 布尔边界。主项目只保存回执 SHA-256、量化 key ID、binding ID、事件时间和追加式状态，不保存完整 token。

## 本人 API

- 既有 `POST /api/me/quant/paper-launches` 增加可选 `esk_quant_allocation_request_id`；未选择时保持 V12 只读行为。
- `POST /api/me/assets/esk/quant-allocation-receipts` 接收当前用户浏览器从量化页面取得的签名回执，验签后幂等追加 `accepted` 或 `released`。
- 本人请求列表返回 binding ID、接收/释放时间和 receipt key ID，但不返回完整回执、授权、participant 或内部用户 ID。

## PC 父页面

- 一键进入卡片加载本人 submitted/accepted 申请，让用户明确选择一个 submitted 请求后再生成授权；不自动选择或批量发送。
- 量化页面只有声明授权 capability 才能收到授权；父页面继续验证 `event.source`、exact origin、nonce、attempt 和过期时间。
- 新回执消息必须绑定同一 source/origin/nonce/attempt，token 形状和长度有界；父页面立即 POST 到主项目并清除内存。
- 同步失败时明确显示“量化已返回回执，但主项目尚未确认”，允许重新进入/重试，不宣称状态已经同步。
- 资产卡显示 submitted/accepted/canceled/released，accepted 文案明确“模拟绑定、仍占用、无份额/收益”，released 文案明确占用已恢复。

## 数据与并发

1. 新事件仍在立即事务内与卖回、量化请求共享余额真源；accepted 不改变占用，released 原子释放。
2. 回执 digest 全局唯一；同一回执重放返回原状态，不同内容重用 receipt ID、binding ID 或非法修订必须冲突。
3. submitted 在签发授权后仍允许取消，直到量化 accepted 回执先提交；取消与 accepted 回执并发时至多一个终态分支成功，失败方获得明确冲突。
4. released 只接受当前 accepted binding 的 revision 2 回执，前序摘要必须匹配已保存 accepted receipt digest。

## 验收标准

1. 主项目授权测试覆盖本人/他人、状态、金额、grant/participant/TTL、capability 和 V12 无选择兼容。
2. 回执 verifier 覆盖 keyring、篡改、错误用户/请求/金额、accepted/released 顺序、幂等和并发取消冲突。
3. 账户余额测试证明 submitted/accepted 继续占用，canceled/released 释放，卖回与新申请不能合计超额。
4. PC 合同和生产构建覆盖选择、内存授权、回执 postMessage、同步成功/失败/重试和状态文案。
5. 双仓两个 Schema 逐字节一致，量化 E2E 证明接收/重启/释放/回执重放，主项目 HTTP E2E 证明回执同步和用户隔离。
6. 主项目发布不启用真实资产；量化独立 HTTPS 未部署时继续报告 deferred。

## 预计实现范围

- `contracts/quant/esk-paper-allocation-authorization-v1.schema.json`
- `contracts/quant/esk-paper-allocation-receipt-v1.schema.json`
- `server/src/esk_asset/`、`server/src/store/common/`、新迁移
- `server/src/quant_paper_launch.rs`、授权签发与量化回执 verifier
- `pc-frontend/src/features/conversation/QuantPaperLaunch.tsx`
- `pc-frontend/src/features/assets/`
- 双仓合同/E2E 脚本、当前事实、接入文档和交付证据

## 回滚

先从 PC 启动 capability 移除授权 V1 并停止签发新单申请授权；继续保留回执同步与量化端释放路径，直到 accepted binding 均释放。主项目不得通过回滚直接把 accepted 改回 submitted/canceled，也不得删除回执摘要或释放事件。
