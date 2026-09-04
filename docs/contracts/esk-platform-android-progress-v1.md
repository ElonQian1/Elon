---
title: "ESK Android 正式进度协议 V1"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets, android, quant-integration]
---

# ESK Android 正式进度协议 V1

本协议是新版唯一原生 ESK 资产入口。按用户 2026-09-05 要求，旧 Paper 17 字段和
正式总量 21 字段原生接口退役，不保留组件别名或兼容回退；固定旧载荷仅用于拒绝测试。
依据：[正式进度需求](../requirements/esk-platform-native-progress-v1.md)。

## 请求与身份

Protocol 为 `yilong.esk.platform_android_progress.v1`，action 为
`com.elon.app.action.READ_ESK_PLATFORM_PROGRESS`；显式组件
`com.elon.app.esk.platform.progress.EskPlatformProgressConsentActivity`。
请求恰好三个实际 String：`protocol`、`nonce`、`cursor`。
nonce 为新随机 64 位小写十六进制；cursor 首页恰为 `""`，后续为
`esbr1.<64小写hex摘要>.eskpsr_<32小写hex>`（110 字符）。
不接受任何额外 Intent 数据、flags、URI、类别、selector、ClipData、identifier 或边界。
主端以 OS 身份验证官方量化当前单签、versionCode>=5、精确非别名
`com.elon.quant.assets.progress.EskPlatformProgressActivity`，并逐页确认主账户。
量化必须反向验证当前主官方单签、精确非别名导出组件及实际首发最低主版本。
纯合同不承担 Android 身份验证或用户同意。

## 固定顶层字段

每项均为实际 String；不允许类型转换或嵌套 JSON。响应精确为以下 35 个顶层字段，
加 `page_count * 6` 个逐行字段。每值最多 128 字符，键最多 64 字符，
最多 155 项；全部 key/value UTF-8 字节总和不得超过 32768。
先检查总项数和值预算、再解析 0..20 的 page_count、再生成精确键集合。

| 字段 | 定义 |
|---|---|
| protocol / nonce | 本协议及本次 nonce |
| requested_cursor | 与本次请求 cursor 逐字相同 |
| asset_id / symbol / decimals | esk / ESK / 6 |
| source / chain_status | platform_recorded / not_deployed |
| simulated / funds_moved | false / false |
| verification_basis | authenticated_operator_review |
| external_payment_verified | false |
| total / total_base_units | 总 ESK 六位小数 / 规范微单位整数 |
| reserved / reserved_base_units | 卖回占用，表示未取消申请的数量 |
| available / available_base_units | 可申请量，总量减占用，非可兑付金额 |
| snapshot_digest | 同快照 64 小写 hex 摘要，不是链证明 |
| request_count / open_count | 全量申请数 / 未取消申请数 |
| range_start / range_end | 当前页 1 基范围，空账本均为 0 |
| page_count | 本页条数，0..20 |
| has_more / next_cursor | true/false；后续游标，无下一页恰为 "" |
| observed_elapsed_ms / expires_elapsed_ms | 同机单调采集/失效毫秒 |
| service_spending / quant_subscription / sellback_settlement | 均 false |
| onchain_transfer / chain_migration | 均 false |
| submit_request / cancel_request | 均 false；没有跨 APK 写授权 |

金额只接受 `(0|[1-9][0-9]{0,12})\.[0-9]{6}`，对应微单位不得超过
Long.MAX_VALUE；整数仅 `0` 或非零开头最多 19 位，且不超过 Long.MAX_VALUE。
三个金额对均完全相等；reserved<=total，available=total-reserved；
open_count<=request_count、open_count<=reserved，且 open_count=0 当且仅当 reserved=0。
不推导登记笔数、价格、法定股权、投资份额或量化网页登录身份。

## 行与分页

索引从 0 至 page_count-1；每行恰含 `request_<i>_id`、`request_<i>_amount`、
`request_<i>_amount_base_units`、`request_<i>_status`、`request_<i>_created_at`、
`request_<i>_canceled_at`。不得含政策、幂等键、账号或写入 payload。

- id 符合 `eskpsr_[0-9a-f]{32}`，本页唯一。
- 金额微单位为正，与六位小数完全匹配。
- status 仅 submitted / canceled；submitted 的 canceled_at 恰为 ""；canceled 必须
  有真实可解析 UTC 时间且不早于 created_at。
- 时间严格为 `YYYY-MM-DDTHH:mm:ss[.1至9位小数](Z|+00:00)`，严格有效日期；
  按创建时刻降序、同刻 ID 降序，不以未经解析的日期文本排序。
- request_count=0 必须无行、range=0/0、无下一页/游标、请求 cursor 为空；有请求不能空页。
- 首页 start=1；有 cursor 则 start>1 且当前摘要等于 cursor 摘要，页内不得重复 anchor。
- end=start+page_count-1 且 end<=request_count，运算避免溢出。
- has_more 等价于 end<request_count；next_cursor 必须绑定当前摘要和本页最后一个 ID，
  不能等于请求游标；无更多时恰为 ""。
- 当前页 submitted 数量/金额不能超过总 open_count/reserved；剩余未见开放笔数
  不超过剩余未见总笔数，其金额下界每笔至少 1 微单位。无剩余开放笔数时金额精确相等。
  完整页必须精确满足全量未取消数与金额守恒；求和溢出拒绝。

## 时效、生命周期与拒绝

`0<=requestedAt<=observed<=now<expires`，`now-requestedAt<120000`，
`1<=expires-observed<=60000`；主端还限制在当前登录会话剩余有效期内。
nonce 一次性消费由外层负责，验证函数不能代替重放保护。

每页新 nonce 和主账户明确同意；接收端开始请求时先清空旧页，既不追加也不持久保存。
当前页只有 60 秒以内有效显示，离页/后台/旋转/过期清空。等待主授权返回的生命周期
只允许短暂保留待处理请求，取消/超时/重建清除，绝不保存到磁盘或日志。
游标失效/409 只报错误并让用户明确重新发起首页，不自动去掉 cursor 重试。
本摘要不证明量化网页登录账户，不显示为已上链、已兑付或实时可提现。
受保护连接不可用时拒绝，不把失败变成零、不回退到 Paper 或网页签名结果。

## 验证责任

两仓纯 Kotlin 合同及测试向量必须字节一致；各自验证真实 Bundle 类型和 Intent外层、
官方组件/签名、会话门禁、一次性响应及生命周期。离线测试不等于真实本人跨 APK 验收。
