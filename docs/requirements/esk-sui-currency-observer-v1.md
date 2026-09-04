---
title: "ESK Sui 规范 Currency 与固定供应只读观察器 V1"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets, protocol]
---

# ESK Sui 规范 Currency 与固定供应只读观察器 V1

## 目标与非目标

在[发布观察器](esk-sui-publication-observer-v1.md)之上，提供独立、无钱包的
测试网 CLI，核对预期 ESK 的规范 Currency 地址、注册创建交易、精度与固定供应。
属于[首批用户路线图](esk-first-user-delivery-roadmap-v1.md)的链证据准备，
代码交付不等于 ESK 实际发行或用户资产验收。

不签名、不广播、不读取钱包/密钥、不变更生产开关或余额，不修改 Move、主项目
HTTP、APK、量化合同及旧创世清单。只有测试网模式。两读取端点不证明运营主体
独立，也不替代源码匹配、逐桶分配、权限、地址归属及委员会终局性验证。

## 输入与可重复身份

严格接收八个字段：`network`、完整 32 字节 Base58 `chain_identifier`、
`package_id`、`publication_digest`、`registration_digest`、
`registration_version`、`expected_supply_base_units`、`endpoints`。
注册版本为正 UInt53 十进制字符串；供应为正 u64 十进制字符串，禁止浮点、
科学计数法、前导零、隐含默认供应或以数字类型传递金额。

币种固定为规范化 package 下 `::esk::ESK`，精度固定为 6。
使用精确锁定的官方 Sui SDK，按 Currency Registry 的派生对象规则计算规范
Currency 地址，不信任调用者随意指定的元数据对象。依赖仅用于离线地址推导，
不初始化网络客户端或钱包。保留锁文件和已知公开链上对象交叉验证用例。

第一个端点固定官方 testnet GraphQL，第二个必须是不同主机的公开 HTTPS
端点；复用现有端点、DNS 防私网、证书校验、禁止重定向、响应大小和整体超时
约束。HTTPS 仅用于外部公开链读取，不变更主项目的服务协议。

## 验收合同

1. 单次固定 GraphQL 查询绑定预期包发布证明和 canonical Currency：历史注册
   版本、注册成功交易及 checkpoint、该交易对该对象的创建输出必须相互一致。
   注册对象必须为规范 Currency 类型，不能用 legacy CoinMetadata 或待注册对象替代。
2. 通过历史对象版本与该注册交易的对象变更核对 `idCreated=true`、未删除、
   无 inputState、输出 address/version/digest 匹配。不得将最新对象版本的
   previousTransaction 当成注册交易；后续元数据/共享对象交易不应误判注册失败。
3. 历史注册状态与当前规范 metadata 都必须是预期 ESK 类型、6 位精度、
   精确预期供应、`FIXED`；`BURN_ONLY`、未知/null、类型不符、超限数值、
   供应不符、历史缺失或版本混用均失败关闭。
4. 仅当两个来源所有归一化证据一致时返回 `observed`。一端失败、部分数据、
   超时或分歧均为 `unverified` 并失败退出；不降级为单端认证。
   日志仅含公开标识和有界错误码，不回显服务端原文、输入凭据或完整端点。
5. 输出固定版本，始终为 `publication_certified=false`、
   `asset_identity_verified=false`、`balance_eligible=false`、
   `manifest_transition_allowed=false`。观察到注册及供应不授权余额入账或状态迁移。
6. 覆盖精确身份、历史与当前版本、创建/修改区分、供应精度、双源分歧、传输
   故障、敏感输入拒绝、CLI 退出码；旧发布观察器 65 项和创世基础回归继续通过。
7. 官方公开非 ESK 对象可用于 schema/地址推导 smoke，但必须明确不算 ESK
   真实发布验收。真实 ESK 验收需真实公开参数与可用双源，不得伪造结果。

## 分工与交付

先独立提交保持行为不变的公共传输抽取及兼容测试；再提交新观察器纵向切片。
入口只组装，地址派生、固定查询、领域校验、双源调度和测试按职责分文件，
每个源文件目标不超过 350 行。不增加已有 599 行创世校验入口。

登记 ID：`esk-sui-currency-observer-v1`。传输、领域测试与官方 API 调研可并行，
同一文件仅一位 Owner。工具绑定源码/测试证据，使用手册记录安装、无密钥参数、
脱敏输出与恢复方法；交付报告分别记录实现、验证、代码推送和 ESK 实际验收。
本轮为 `CodePushed`，不要求重发未改动的服务器或 APK。
