---
title: "ESK Android 本机只读快照协议 V1"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, quant-android]
---

# ESK Android 本机只读快照协议 V1

本协议只描述主 APK 与量化 APK 的本机只读余额快照，不是网络 API、链上凭证或资产发行证明。
当前 Android [合同源码](../../android/app/src/main/kotlin/com/elon/eskcontract/EskSnapshotContract.kt)
在两仓库保存**字节一致的共享副本**，对应测试也保持相同。后续协议变化须两端一起升级；
未知版本、未知字段和未知状态失败关闭，不靠字段猜测保持兼容。

当前主 API 的受保护 HTTPS 通路仍未部署验收，本协议及测试不能证明真实用户资产可用。
公共 HTTP 行情与原生私有快照隔离；私有请求不得回退 HTTP。

## 请求

显式 Activity action 为 `com.elon.app.action.READ_ESK_SNAPSHOT`。
请求 Bundle **恰好两个 String 字段**：

| 字段 | 值 |
|---|---|
| `protocol` | `yilong.esk.android_snapshot.v1` |
| `nonce` | 随机 256 位值编码为 64 个小写十六进制字符，无 `0x` |

调用包、正式签名、最低版本、Activity 目标、当前会话与用户确认由 Android 外层验证。
nonce 只保留在当前待处理实例内，响应必须匹配并由外层消费一次；纯 Map 校验器不保存重放状态。

## 响应

只允许以下 **17 个字段，全部为 String，每个最多 128 字符**，不允许缺项或扩展项。
Bundle 的实际类型须在转成 `Map<String, String>` **之前**检查，不能用 `toString()` 强转数字、
布尔或 Parcelable。纯 Kotlin Map 单元测试不证明 Android 解包、类型检查或恶意 extras 安全。

<!-- SNAPSHOT_FIELDS_START -->
| 字段 | 精确定义 |
|---|---|
| `protocol` | 固定 `yilong.esk.android_snapshot.v1` |
| `nonce` | 与本次有效请求相同的 64 个小写十六进制字符 |
| `asset_id` | 固定 `esk` |
| `symbol` | 固定 `ESK` |
| `mode` | 仅 `paper` 或 `disabled` |
| `issuance_mode` | 固定 `paper_recorded` |
| `chain_status` | 固定 `not_deployed` |
| `simulated` | 固定字符串 `true`，不是 Boolean |
| `funds_moved` | 固定字符串 `false`，不是 Boolean |
| `total` | ESK 总量，规范六位小数 |
| `available` | 可用 ESK，规范六位小数 |
| `reserved_for_sellback` | 卖回申请占用 ESK，非已兑付现金 |
| `reserved_for_quant` | 量化申请占用 ESK，非 QSHARE、仓位或投资收益 |
| `reserved_total` | 两类占用合计 ESK，规范六位小数 |
| `revision` | 非负规范十进制 Long 字符串，来源修订，不是链上高度 |
| `observed_elapsed_ms` | 本机单调时钟采集毫秒，非负规范十进制 Long 字符串 |
| `expires_elapsed_ms` | 同一单调时钟的到期毫秒，非负规范十进制 Long 字符串 |
<!-- SNAPSHOT_FIELDS_END -->

金额精确为 `(0|[1-9][0-9]{0,12})\.[0-9]{6}`，允许零，不允许正负号、前导零、
指数、逗号、空白、少位或超位后舍入。换算微单位后最大为
`9223372036854775807`，对应 `9223372036854.775807 ESK`。
整数只允许 `0` 或非零开头的最多 19 位数字，并须不超过 Long.MAX_VALUE。

必须同时满足：`total = available + reserved_total`；
`reserved_total = reserved_for_sellback + reserved_for_quant`。
使用 BigInteger 精确校验，不用浮点数，不将占用额再加到总量上。

## 有效期与生命周期

调用者在发起请求时保存 `requestedAt`；接收时取同机单调时钟 `now`，两者不是响应字段。
纯合同要求：

- `0 <= requestedAt <= observed_elapsed_ms <= now < expires_elapsed_ms`。
- `now - requestedAt < 120000`；恰好 120 秒即无效。
- `expires_elapsed_ms - observed_elapsed_ms == 60000`；V1 是精确 60 秒，不接受更长或更短 TTL。
- 负值、时钟逆行、溢出和无法解析的时间全部拒绝。

外层在取消、后台、离开、刷新或实例重建时始终清空已展示快照，显示期结束也立即清空。
仅本实例等待显式主授权 Activity result 时，可保留单次待处理 nonce 至 120 秒 deadline；
正常唤起主授权页引起的量化页后台切换不因此取消该请求。取消、刷新或实例重建必须清空 nonce，
主确认页离开即取消授权，不后台签发。
快照不进入磁盘、日志、剪贴板、WebView、saved state 或凭据传输；
调用者签名认证、一次性消费、生命周期和 FLAG_SECURE 由各端独立测试，不能由本文件推导已验收。

## 合成线格式向量与验证

以下全部是假数据，仅用于字段与字符串编码一致性；`requestedAt=1000`、`now=3000`。
不是用户持币、付款、批准、链上发行或可兑付证明。

<!-- SYNTHETIC_WIRE_FIXTURE_START -->
```json
{
  "protocol": "yilong.esk.android_snapshot.v1",
  "nonce": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "asset_id": "esk",
  "symbol": "ESK",
  "mode": "paper",
  "issuance_mode": "paper_recorded",
  "chain_status": "not_deployed",
  "simulated": "true",
  "funds_moved": "false",
  "total": "1250.000000",
  "available": "900.000000",
  "reserved_for_sellback": "100.000000",
  "reserved_for_quant": "250.000000",
  "reserved_total": "350.000000",
  "revision": "7",
  "observed_elapsed_ms": "2000",
  "expires_elapsed_ms": "62000"
}
```
<!-- SYNTHETIC_WIRE_FIXTURE_END -->

[纯 Kotlin 测试](../../android/app/src/test/kotlin/com/elon/eskcontract/EskSnapshotContractTest.kt)
实际执行 Map、数值和时间边界校验；应分别由两仓库的受控 Gradle 验证入口运行。
主仓 `scripts/test-esk-native-snapshot-contract.js` 仅静态检查字段、常量及本合成向量。
传入 `--quant-root <量化仓库>` 才增加两仓源码、测试和本文的原始字节一致性校验；
不传时必须报告跨仓检查未执行。静态脚本不会执行 Kotlin、设备、网络或业务操作。
