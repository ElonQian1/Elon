---
title: "ESK 正式付款占用快照与对账接入 V1"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets]
---

# ESK 正式付款占用快照与对账接入 V1

## 用户目标与现状

首批用户付款复核应直接使用正式账本已经占用的付款事实，避免人工遗漏历史键后，
预演把已登记或待审核付款列为可复核。已有正式登记事务仍能阻止重复记账；本功能
补齐预演阶段的数据来源，不重写登记、取消、用户余额或 Paper 批量接口。

既有 [付款预演](esk-paid-user-reconciliation-preview-v1.md) 只接收操作者填写的历史快照。
本功能连接正式 Store、管理员只读 HTTP、离线合并和既有预演算法，输出逐行结果。
不使用快照代替真实到账、用户映射、用途、条款、同意和人工摘要确认。

## 正式快照合同

`GET /api/admin/assets/esk/platform-reconciliation-snapshot`，不接受查询字段。
只允许 active 真实数据库 admin/owner 会话；拒绝静态管理员、虚拟 owner 和普通用户。
事务内再次验证角色/会话，读取已固定政策，不因暂停新写入隐藏已有历史。
没有固定政策、损坏政策或账本必须明确失败，不能返回貌似完整的空历史。

同一 SQLite 只读事务验证政策、正式审批/分录/取消关系和所有未取消申请，
导出未取消准备与已入账记录的付款键。仅已取消且未重新准备的付款不占用。
每个键一次、按 ASCII 升序，最多 10000；超量明确失败，禁止截断后标记完整。
事务不建立政策、更新会话、写审计或改变业务数据；不存在跨页拼接。

响应严格字段：

| 字段 | 值或含义 |
| --- | --- |
| `schema` | `yilong.esk.platform_payment_snapshot.v1` |
| `scope` | `platform_recorded_allocations_only` |
| `source_fingerprint / policy_digest` | 现有固定政策的 64 位小写 SHA-256 |
| `observed_at` | 服务端采样的 UTC 毫秒时间，`YYYY-MM-DDTHH:mm:ss.sssZ` |
| `used_payment_keys` | 规范付款身份摘要的严格升序数组 |
| `prepared_count / recorded_count / key_count` | 精确整数字符串；前两者之和等于第三者及数组长度 |
| `platform_history_complete` | `true`，仅指本次事务中的正式登记范围 |
| `external_history_complete` | `false`，不覆盖外部收款、旧系统或其他产品历史 |
| `funds_moved / balances_written / external_payment_verified` | 均为 `false` |
| `snapshot_digest` | 把本字段置 `null`，对完整响应做现有 sorted-key canonical JSON SHA-256 |

不返回用户 ID、原始付款引用、来源原文、金额、操作者或审核材料。摘要不是签名，
不能证明下载来源或付款真实性。响应 `no-store/no-cache/no-referrer`；私有凭据仍须
走既有受保护传输，不能由公共 HTTP 下载可用推导为私人读取安全。

## 离线接入现有预演

新增标准输入 CLI `scripts/preview-esk-platform-reconciliation.js`，输入为严格 JSON：
`schema=yilong.esk.platform_reconciliation_input.v1`、`reconciliation`（原 V1 输入）和
`platform_snapshot`（上表完整响应）。复用原严格解析器：最多 1 MiB、30 秒、无未知/
重复键、无凭据参数、不联网、不读环境秘密、不写文件；只提交合成测试材料。

先核对快照形状、固定标志、计数、排序、摘要、来源及相对 `reconciliation.as_of`
的 24 小时时间窗。任一不符整体失败，不把缺少快照降级为原人工模式。
原人工历史仍须独立完整且新鲜；原快照内重复键保持错误，不能用合并悄悄修复。
联合键取两来源并集，最多 10000，使用较早观测时间，再交给既有预演算法。
平台完整性不会把人工的 `history_complete=false` 改成 true。

输出独立 `yilong.esk.platform_reconciliation_preview.v1` 信封，绑定原输入摘要与平台
快照摘要，内含原 V1 逐行预演；不输出登记提交文件。固定 `mode=dry_run`、
`funds_moved=false`、`balances_written=false`、`commit_eligible=false` 和
`platform_snapshot_authenticity_verified=false`，明确只能检查提供材料的一致性。
退出码 0 为无阻塞预演、2 为业务待复核、1 为输入/快照错误；不代表已发币或已入账。

## 验收与编辑边界

1. 合成真实 Store 准备/登记/取消产生快照，再进入实际 JS CLI；已占用被挡，
   未使用可复核，纯取消释放、重新准备再次阻断；Paper 与正式余额保持不变。
2. 同付款换用户/金额仍命中；来源、快照篡改/未来/过期、人工历史不完整、重复键、
   超量和损坏账本有反例；不能以宽松 JSON 或摘要重算宣称真实性验证。
3. HTTP 测试覆盖未登录、静态凭据、普通角色、撤销/过期、写入关闭仍可读取，
   非法查询、输出脱敏和 no-store；真实 TCP 使用合成会话，前后无业务写入。
4. 复用已有付款预演/正式账本测试；新模块单文件目标小于 350 行，入口只做组装。
   root 负责模型、HTTP、JS 和文档；独立 Store/test Owner 不编辑同一文件。
5. 代码、编译/跨语言测试、后端发布、公开未登录检查与真实管理员使用分别记证据。
   本批不读取生产用户或付款，不开启登记政策，不执行正式审批/余额/链操作。

主账本需求、经济参数及首批用户 Goal 保持不变。可信资料导出与传输准备、运营用户
映射、实际款项核对、逐笔审批和双正式 APK 本人验收仍需独立完成。
