---
title: "ESK Sui 六桶分配观察器 V1 交付证据"
version_status: current
reviewed_at: 2026-09-05
owners: [platform-assets, protocol]
---

# ESK Sui 六桶分配观察器 V1 交付证据

本文件只记录代码、测试和交接事实，不批准经济参数、地址、签名、发布或资金操作。
范围见[正式需求](../requirements/esk-sui-allocation-observer-v1.md)，操作见
[使用手册](../esk-sui-allocation-observer.md)。

## 当前状态

| 能力 | implementation_status | verification_status | delivery_status | acceptance_status |
| --- | --- | --- | --- | --- |
| 双公共源创世分配观察 | implemented | 98 项离线测试通过 | pushed | pending：缺真实 ESK 参数 |
| 发布 Cap 唯一性与一次消费 | implemented | synthetic/integration passed | pushed | pending：缺真实发布/分配交易 |
| 六桶历史输出与团队锁仓快照 | implemented | synthetic/integration passed | pushed | pending：缺真实对象/checkpoint |
| 官方 testnet 固定查询兼容 | implemented | environment passed | pushed | accepted：仅无关公开样本 |
| 平台余额或 manifest 晋级 | deliberately disabled | fixed-false tested | 本批不交付 | not authorized |

这里的 `implemented/verified` 只描述本需求限定的只读工具。真实 ESK 正向观察、链上
发行、地址控制、用户余额和终局性仍未验收。本项目只继续新版冻结总回执合同，不读取、
迁移或恢复旧 Paper/原生桥、旧六回执 manifest 或 `vesting_policy_ref` 双轨。

实现提交 `0da99c8b1eed9d95462fdb53babfe6da1671729f` 已推送至远程 `main`。
该提交只交付源码、测试和文档，不是 testnet 发布或用户资产验收。

## 已执行验证

- `node scripts\test-esk-sui-allocation-observer.js`：98/98 通过；覆盖严格输入、BCS、
  固定 GraphQL、双源一致性、CLI、完整对象变化、唯一 Cap、六桶、版本和锁仓守恒。
- 两笔交易都要求未分页的完整 object changes；空节点、不可分类 state、缺 BCS、重复
  object ID、额外或缺失 ESK 目标均失败关闭。
- 发布 Cap 输出绑定发布 Lamport version；回执、四枚新 Coin、团队锁仓和安全储备
  输出绑定 allocation Lamport version；Cap 消费与供应 mutation 版本关系有负例。
- allocation 与 observation 时间使用严格 UTC RFC3339 日历校验并进入规范化证据；
  不存在日期、倒序时间和同 checkpoint 不同毫秒均拒绝。
- 原发布观察器 65/65、Currency 观察器 312/312、六桶静态证据回归均通过。

2026-09-04 19:56:20 UTC，官方 testnet 固定查询实际成功。输出为
`schema_pass=true`、`unrelated_sample_rejected=true`、
`rejection_code=OWNER_MISMATCH`、`source_count=1`、
`NO_BALANCE_OR_CERTIFICATION=true`。这只证明当前公开 GraphQL schema 接受查询且
无关样本未被误认证；它没有运行真实 ESK 正向观察，也不单独证明 ESK 类型门禁。

## Move 回归复跑

本轮用 `sui 1.79.0-46f18562f1f5` 和固定依赖提交
`46f18562f1f5af2438d35828e8b62d5e0b972db7` 重新执行：

- `esk_currency`：3/3 通过；
- `yilong_participation`：13/13 通过；
- 两项均启用 `--warnings-are-errors`，没有 Move compiler warning；
- 当前六份 Move 源码/测试与隔离运行副本逐文件 6/6 SHA-256 相同；
- 没有下载新 CLI、创建钱包、签名、广播或修改链状态。

官方 CLI 可执行文件的本机 SHA-256 为
`D9B7FF7B4BB3CBBF3F327DDF5998B388773956CE30C897798B56A6C0DB9FEE7F`。
离线 RPC 探测回退 protocol version 136 是现有 ADR 允许的测试环境行为，不是编译器
warning，也不构成 testnet 发布证据。

## 跨平台证据修复

Windows `core.autocrlf` 会把已提交的 LF 原始测试回执检出成 CRLF，导致字节摘要在新
worktree 中漂移。本批把 `contracts/sui/**/evidence/*.txt` 固定为 LF，并在静态门禁读取
时先规范化换行再计算仓库定义的回执摘要。原证据文本和声明的 Move 结果没有改写；
修复只使同一 Git blob 在 Windows/Linux 上得到同一校验结果。

## 未执行与下一步

1. 未取得经批准的真实 testnet package、发布/分配摘要、Cap、供应 Coin、回执、锁仓、
   两个 checkpoint、六桶金额、五个职责地址和第二公共 GraphQL 来源，所以
   `ESK_SUI_ALLOCATION_REAL_ESK_ACCEPTANCE=not_performed`。
2. 未读取钱包或交易所凭据，未签名、广播、移动 ESK/SUI/USDT，未发布服务器或 APK，
   未改变用户余额或生产卖回策略。
3. 获得明确 testnet 授权和完整审核参数后，依次发布/注册/分配，再运行 publication、
   currency、allocation 三个双源观察器；随后另立 Evidence/Manifest V2 终局投影。
4. 地址控制、源码对应性和委员会终局性仍需独立证据；观察器报告的八个认证/余额/
   迁移标志继续固定为 `false`。

功能注册表在本记录更新后重新绑定全部当前证据；最终文档/注册表提交和工作树状态由
本批统一 Git/finish 回执报告。
