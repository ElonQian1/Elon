---
title: 开放商业商户运行时 V1 验收
status: current
owner: backend
reviewed_at: 2026-08-13
---

# 开放商业商户运行时 V1 验收

## 已验证

- 平台数据库迁移保留原能力并新增受控 RuntimeBinding。
- 生产地址强制 HTTPS 与主机白名单；回环 HTTP 仅在测试构建可用。
- 共享密钥只通过服务端环境变量引用解析。
- 平台与商户端对原始请求体执行 HMAC-SHA256 签名和重放窗口校验。
- 健康检查核对商户身份与 Manifest 摘要，未验证绑定不能调用。
- 同一平台幂等键不重复访问商户运行时，不重复计量。
- PC 工作台可配置、验证并查看运行绑定，不收集共享密钥。
- `cofficethinking` 定向编译与 9 项 `commerce_gateway` 测试通过，覆盖签名、整数金额、标准业务回执、Manifest、平台签名动作确认及旧双确认输入拒绝。
- 两仓库的 `contracts/open-commerce/merchant-runtime-v1.json` 已恢复逐字节一致，跨仓库规范化 SHA-256 为 `1FC2C8A8659729957D8E225DAD5F5C7BCA4D72DFF29C373833BEAAC175659E2A`。
- `cofficethinking` 已删除 `order.commit` 业务输入中的 `confirmed_by_user` 和 `confirmation_id` 第二权威，改为只接受 HMAC 信封中的 `action_confirmation_id`；凭据环境、凭据 ID、Grant 和动作确认同时进入商户侧幂等摘要，订单审计快照记录确认权威来源。
- 平台将已服务端确认并一次性消费的动作确认 ID 写入 HMAC 信封；通用 Node 内核的 `order.commit` 不信任业务输入自行声明的确认布尔值。
- Node 运行时专项 18 项和连接器 SDK 全量 55 项已实际通过，覆盖签名、来源身份、Grant、动作确认、不可变 Manifest、结果上限、并发忙碌、幂等冲突、失败释放和成功重放。
- Rust 本地端到端专项已实际通过：登记消费者开发者 App 后，由消费者 MCP 准备并确认动作，再调用临时回环商户节点完成 `order.commit`；测试同时确认 HMAC 信封携带已消费的 `action_confirmation_id`、计量仍为 `recorded_not_charged`，并从同一 Invocation 派生出引用同一商户订单号的有效标准业务回执。最新验证指纹为 `058e16928580a66d1ee5e11007f178a220078969ad9296a96065f4e18923da5e`。
- 双 TCP 纵向专项已实际通过：独立消费者账号使用自己项目内的注册 App 和登录会话，经完整生产 Router 的消费者 HTTP 入口准备、确认并调用 `order.commit`；平台再以 HMAC 调用另一回环端口的商户运行时。测试确认信封中的用户、App 和动作确认身份均来自消费者侧，商户运行时返回有效标准业务回执，随后独立 ERP 机器凭证领取的正是同一个 Invocation。验证指纹为 `1db875411f8c307187426ead6c909dbfa3d09499f1d92757ec1aaaa48eacc87e`，验证收据为 `19dffabb51bbef967cf1c13f61ff0015db97d781d3f604a0d69e0227abe4cf19`。
- 生产凭据双 TCP 纵向专项已实际通过：测试在隔离子进程中显式开启生产开关，由服务层向本地已准入开发者 App 签发一次性 `oc_live_`，经完整生产 Router 的开发者入口准备、确认并调用 `order.commit`，再由 ERP 机器凭据接管同一个 Invocation。商户运行信封中的用户、App、`production` 环境、凭据 ID 和动作确认 ID 均与生产调用一致；验证指纹为 `f4cb3ffc40393de9934df023db15016d6c8368917626e94a79c96bce7641f246`，验证收据为 `8c53c53122bc05740cafe7b6d6f2c31cf893db77b969e295df1c4118c581acd3`。

补充的运行时公网 DNS 拒绝和单次连接地址固定代码已通过 Rust 编译检查，但尚未执行 DNS、TLS 或真实网络回归，见 `docs/open-commerce-merchant-runtime-egress-pinning-v1-acceptance.md`。

## 本地验证边界

Rust 端到端专项只使用全新 SQLite、本机回环 HTTP 商户节点和模拟 ERP 目标，证明平台侧协议、消费者/开发者生产凭据身份和衔接状态链路可执行；生产凭据专项中的 App 准入也是本地领域状态，不是外部组织或域名审核。`cofficethinking` 的 9 项测试也不启动 PostgreSQL 或写真实订单。上述证据不证明公网 DNS、TLS、标准 443 白名单、跨机器网络质量、生产数据库迁移或生产商户后端已经可用。

## 仍需环境配置

生产启用仍需要：商户运行 HTTPS 域名、平台主机白名单、双方一致的随机共享密钥、平台商户 ID、咖啡店门店 ID和至少一个在售商品。没有这些部署参数时，代码存在但运行时保持关闭，不能宣传为公开生产网络。

## 非验收范围

- 美团、抖音、京东、淘宝闪购生产适配器。
- 真实支付扣款、自动分账或 Sui 网络提交。
- 公共跨项目消费者网络和生产第三方 App 审核。
