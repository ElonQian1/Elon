---
version_status: current
reviewed_at: 2026-09-04
implementation_status: in_progress
---

# 正式 ESK 本人审核流水分页 V1

## 目标与边界

主项目已具有正式 ESK 账户 V1，但原生页只显示最近 20 笔。
新增本人完整审核流水只读分页，明确区分全账户总额与当前页记录范围。
复用正式账本、主项目真实会话及现有设计色，不创建第二套余额。
本期不增加正式消费、占用、卖回、转账或链上操作，不变更 Paper、QSHARE、
账户 V1、双 APK 摘要协议、权限、网络来源或证书配置。

## 后端合同

- `GET /api/me/assets/esk/platform/history?limit=20[&cursor=...]`。
- `limit` 为 1..100 整数，默认 20；未知、重复查询参数拒绝。
- 只接受当前真实用户 Bearer 会话；不接受 cookie、静态 owner/admin token 或查询用户 ID。
- 每页同一 SQLite 读事务重新检查会话、用户、正式账本完整性与固定策略。
- 返回 `Cache-Control: no-store`、`Pragma: no-cache`、`Referrer-Policy: no-referrer`。
- 独立 schema `yilong.esk.platform_history.v1`，不扩展账户 V1 的严格字段集合。

固定字段：`asset_id=esk`、`symbol=ESK`、`decimals=6`（JSON 数字）、
`source=platform_recorded`、`chain_status=not_deployed`、`simulated=false`、
`funds_moved=false`、`verification_basis=authenticated_operator_review`、
`external_payment_verified=false`。

其余字段：`snapshot_digest`（64 位小写十六进制）、`total`（六位小数）、
`total_base_units`、`entry_count`、`range_start`、`range_end`（规范非负整数字符串），
`updated_at`（最新审核时间或 null）、`entries`、`has_more`（布尔）、
`next_cursor`（字符串或 null）。无其他字段。

每条 entry 完全沿用账户 V1：`entry_id`、`allocation_id`、`amount`、
`amount_base_units`、`created_at`、`kind=approved_payment_allocation`。
顺序为数据库原始 UTC 时间文本 `created_at DESC, entry_id DESC`。
账户摘要是全历史，范围为当前页一基闭区间；空账户仅首请求返回 0/0。
每笔为严格正数，金额、计数采用已检查的 i64 运算，禁止浮点计算。

## 快照与游标

- 格式：`ephp1.<snapshot_digest>.<eskp_entry_加32位小写hex>`。
- 指纹使用 SHA-256：版本域、本人 user_id、固定 policy_digest、完整有序分录。
  各文本均以 UTF-8 字节长度 u64 大端前缀编码；无策略用独立标记，不能与空串混淆。
- 指纹与游标只描述分页位置，不是链证据、付款证明、授权凭据或对外签名。
- 每次仍只查询已认证本人，完整校验和重算摘要；不在游标中信任用户身份。
- 仅在还有下一页时返回下一游标，锚定当前页最后分录。
- 不缓存全历史，只保存当前最多 100 条；每次扫描复杂度为 O(本人分录数)，
  不宣称数据库 keyset 加速或长期归档能力。
- 格式错误返回 400；指纹变化、跨账户、未知锚点及锚定末条返回统一
  `409 ESK_PLATFORM_HISTORY_CHANGED`，客户端清空并从第一页重新加载。
- 新增分录（包括时间回拨）也改变指纹；不得拼接不同时间的记录。

## Android 与 Web

- 原正式资产页增加“查看完整审核流水”，打开非导出的独立原生 Activity。
- 新页有全账户总额、总笔数、当前范围、审核分录、“下一页”与“重新加载”。
- 只保留当前一页内存；使用现有严格 JSON 边界、身份世代与一次性请求门。
- 安全来源校验必须先于读取 token；HTTP 私有请求仍关闭，不新增证书或改用 HTTPS。
- 用户切换（含 A→B→A）、退出、后台、保存状态、超时、迟到或重复回调均清空。
- 续页与当前展示会话严格绑定；会话失效或页面展示超过 60 秒则清空。
- 不向日志、磁盘、WebView、Intent、SavedState 写入历史、游标或会话数据。
- 409 明示“账本已更新，请重新加载”；不保留旧页冒充当前账本。
- 主项目 PWA 同步更新正式资产说明及 APK 引导，不伪造 Web 私有流水可用。
  本期原生私有能力边界与既有正式资产页一致，Web 不代持会话。
- 复用主项目颜色/Views 布局；Preview-first，默认不操作物理设备。

## 验收与交付

1. 合成账本空、多页、同时间、不同 limit、最后一页、完整金额及范围守恒。
2. 新入账、跨用户、末条/未知锚点、非法游标、损坏分录、停用/撤销会话失败关闭。
3. 真实路由测试覆盖认证、未知/重复参数、范围限制、409 与每类响应不缓存。
4. Android 严格字段/UTF-8/金额/范围/摘要/游标验证及隐私生命周期回归。
5. 原账户、Paper 与双 APK 协议兼容测试保持通过；正式全历史不可经旧协议泄出。
6. 分别报告代码、测试、推送、服务器/APK 发布、视觉和真实本人验收状态。
7. 没有核对付款、映射和审批，不创建真实用户余额或进行生产资金操作。

## 分工与后续入口

- 后端账本页与指纹：`server/src/store/common/esk_platform_assets/history.rs`。
- 独立 HTTP 合同：`server/src/esk_platform/history_api.rs`。
- Android 解析/传输与原生页面分开；只把无行为变化的严格 JSON 搬迁独立提交。
- 交付证据：`docs/delivery/esk-platform-history-v1.md`；未实际验收不得标为用户可用。
- 正式占用/官方卖回仍待独立状态机与授权切片，本期不从 Paper 偷换为正式余额。
