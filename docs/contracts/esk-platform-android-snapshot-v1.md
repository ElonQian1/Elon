---
title: "ESK Android 正式平台登记摘要协议 V1"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, android, quant-integration]
---

# ESK Android 正式平台登记摘要协议 V1

本协议只投影主项目经审核的平台登记总量，不是发行证明、链上凭证、付款自动核验、
投资份额、即时余额或兑付授权。依据是[提供端需求](../requirements/esk-platform-native-provider-v1.md)。
[纯 Kotlin 合同](../../android/app/src/main/kotlin/com/elon/eskcontract/EskPlatformSnapshotContract.kt)
与[测试](../../android/app/src/test/kotlin/com/elon/eskcontract/EskPlatformSnapshotContractTest.kt)
是当前主仓实现；量化接收端尚待独立接入和验收，届时合同副本必须字节一致。

此协议独立于旧 `yilong.esk.android_snapshot.v1` Paper 17 字段协议：互相拒绝，不升级、
重命名或放宽旧协议。当前主私有 HTTPS 和首批真实账户仍待环境与入账验收；
合同、合成测试或提供端构建不能证明双 APK 或真实用户数量已经可用。

## 请求与外层认证

action 固定为 `com.elon.app.action.READ_ESK_PLATFORM_SNAPSHOT`，必须显式指定主提供端组件。
请求 Bundle 恰好包含两个 String：

| 字段 | 值 |
|---|---|
| `protocol` | `yilong.esk.platform_android_snapshot.v1` |
| `nonce` | 每次随机 256 位值，64 个小写十六进制字符，无 `0x` |

外层依据操作系统提供的调用方，验证官方量化包、当前固定签名、至少版本 4 和非别名组件
`com.elon.quant.assets.platform.EskPlatformAssetsActivity`，并要求当前主账户显式同意。
纯 Map 合同不验证 Android 身份、Bundle 实际类型、用户同意或会话；这些必须另行测试。
HTTP origin 在读取会话/凭据前拒绝，主端只使用固定 HTTPS 本人接口及既有严格来源解析器。

## 响应

恰好 **21 个字段，全部为 String，每值最多 128 字符**。拒绝缺失和额外字段；
Bundle 必须先验证实际 String 类型，不能对 Boolean、Number、Parcelable 使用 `toString()`。
未知协议、来源或能力失败关闭，不回退到 Paper，也不把不可用响应转换为零。

<!-- PLATFORM_SNAPSHOT_FIELDS_START -->
| 字段 | 精确定义 |
|---|---|
| `protocol` | 固定 `yilong.esk.platform_android_snapshot.v1` |
| `nonce` | 与本次待处理请求相同的 64 个小写十六进制字符 |
| `asset_id` | 固定 `esk` |
| `symbol` | 固定 `ESK` |
| `decimals` | 固定字符串 `6` |
| `source` | 固定 `platform_recorded` |
| `chain_status` | 固定 `not_deployed` |
| `simulated` | 固定字符串 `false` |
| `funds_moved` | 固定字符串 `false`，本次读取不移动资金 |
| `verification_basis` | 固定 `authenticated_operator_review` |
| `external_payment_verified` | 固定字符串 `false` |
| `total` | 主项目正式账本总量，规范六位小数 |
| `total_base_units` | 同一总量的非负规范十进制微单位字符串 |
| `entry_count` | 全部正式审核分录笔数，非负规范十进制字符串 |
| `observed_elapsed_ms` | 本机单调时钟采集毫秒，非负规范十进制字符串 |
| `expires_elapsed_ms` | 同一单调时钟的到期毫秒，非负规范十进制字符串 |
| `service_spending` | 固定字符串 `false` |
| `quant_subscription` | 固定字符串 `false` |
| `sellback_settlement` | 固定字符串 `false` |
| `onchain_transfer` | 固定字符串 `false` |
| `chain_migration` | 固定字符串 `false` |
<!-- PLATFORM_SNAPSHOT_FIELDS_END -->

金额符合 `(0|[1-9][0-9]{0,12})\.[0-9]{6}`，换算微单位后不得超过
`9223372036854775807`，即 `9223372036854.775807 ESK`。整数只允许 `0` 或
非零开头的最多 19 位数字，且不超过 Long.MAX_VALUE；不允许符号、空白、指数或舍入。
必须精确满足 `units(total) = total_base_units`；`entry_count = 0` 当且仅当总量为零；
正数笔数不得超过总微单位数，因为每条正式分录至少为一个正微单位。

不包含账户 ID、昵称、token、付款证据、分录、可用额、占用、收益、汇率或修订号。
量化端不得从这个摘要推导 QSHARE、量化网页登录身份、可消费余额或可兑付金额。

## 时效与一次性处理

接收端自行保存请求单调时间 `requestedAt`，验证时读取同机单调时间 `now`：

- `0 <= requestedAt <= observed_elapsed_ms <= now < expires_elapsed_ms`。
- `now - requestedAt < 120000`，恰好 120 秒拒绝。
- `1 <= expires_elapsed_ms - observed_elapsed_ms <= 60000`；允许因主会话临近到期缩短。
- 主提供端须另将到期值限制在已知会话剩余有效期内；纯合同没有 epoch 会话信息，不能代做此检查。
- 先检查非负和时间顺序再减法；负值、逆行、非规范整数、溢出和过期全部拒绝。

外层必须一次性消费 nonce。纯验证函数无重放状态，重复传入同一 Map 仍可能通过。
等待显式主授权结果时，本实例仅保留待处理 nonce 至 120 秒期限；后台、离页、刷新、取消
和重建始终清空已展示摘要，取消/刷新/重建也清空待处理请求。主确认页退后台即取消。
量化页面必须标明“本次主项目确认账户的临时摘要”，不能声称实时余额；
主端返回后无法远程撤回摘要，接收端必须落实显示期限和生命周期清空。
摘要不进入磁盘、日志、剪贴板、WebView 或 saved state；外层须启用 FLAG_SECURE。

## 合成线格式向量

仅用于合同测试：`requestedAt=1000`、`now=3000`，不是任何真实用户或付款记录。

<!-- SYNTHETIC_WIRE_FIXTURE_START -->
```json
{
  "protocol": "yilong.esk.platform_android_snapshot.v1",
  "nonce": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "asset_id": "esk",
  "symbol": "ESK",
  "decimals": "6",
  "source": "platform_recorded",
  "chain_status": "not_deployed",
  "simulated": "false",
  "funds_moved": "false",
  "verification_basis": "authenticated_operator_review",
  "external_payment_verified": "false",
  "total": "1250.000000",
  "total_base_units": "1250000000",
  "entry_count": "7",
  "observed_elapsed_ms": "2000",
  "expires_elapsed_ms": "62000",
  "service_spending": "false",
  "quant_subscription": "false",
  "sellback_settlement": "false",
  "onchain_transfer": "false",
  "chain_migration": "false"
}
```
<!-- SYNTHETIC_WIRE_FIXTURE_END -->

测试覆盖形状、数值、来源、Nonce、缩短 TTL 与旧 Paper 互拒；
不替代实际 Bundle 解包、双 APK 身份和一次性往返、设备生命周期、安全传输或真实入账验收。
