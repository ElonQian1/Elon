---
title: AI 原生开放商业网络 V1 验收计划
owner: quality
reviewed_at: 2026-07-28
status: active
source: docs/decisions/open-commerce-network-v1-architecture.md
---

# AI 原生开放商业网络 V1 验收计划

## 验收原则

第一版只验证运行闭环和边界，不用虚构商户规模、经营收益或真实资金结算。测试数据必须明确标记为演示。

## 自动化验收

1. 数据库从旧版本迁移后建立商户、能力、授权、调用和审计表。
2. 同一项目不能创建重复 slug；同一商户不能创建重复能力键。
3. 非项目成员不能管理商户节点。
4. viewer 不能创建节点、能力或授权。
5. `public` 能力无需 grant，但仍生成调用和审计记录。
6. `authorized` 能力在缺少、过期、撤销或 scope 不匹配时失败。
7. `owner_only` 能力拒绝项目外调用者。
8. 相同幂等键不会重复计量；不同输入复用同一幂等键返回冲突。
9. 调用记录只保存请求哈希和字段摘要，不默认保存原始输入。
10. 未知处理器和任意 HTTP 处理器被拒绝。
11. HTTP、MCP 和 PC 概览读取同一组统计。

## 真实纵向验收

在测试项目中：

1. 创建演示商户“测试咖啡店”；
2. 发布公开能力 `store.profile.read`；
3. 发布受限能力 `booking.preview`；
4. 调用方 A 通过 PC 工作台读取商户并调用公开能力；
5. 调用方 B 通过 MCP 读取同一商户；
6. B 未授权调用预约能力应失败；
7. 项目编辑者为 B 创建仅含 `booking.preview` 的限时授权；
8. B 再次调用成功；
9. 撤销授权后调用立即失败；
10. PC 工作台显示全部调用、金额为零或演示微单位、结算状态为“仅记录未扣费”。

## 最终统一验证

代码全部完成后再集中运行：

- Rust 格式化；
- 服务端目标测试；
- 服务端 `cargo check`；
- PC 前端项目相关测试；
- PC 前端生产构建；
- HTTP 纵向 smoke；
- MCP `initialize`、`tools/list` 和核心调用 smoke；
- 浏览器确认项目“开放商业”页面与真实 API 数据一致。

只有运行时代码需要在实际网页中验收时才发布一次服务端；不为中间提交重复构建或发布。

## 2026-07-29 实现与验证记录

当前 V1 已形成同一服务层驱动的纵向实现：

- SQLite v108：商户、能力、授权、调用和审计五类事实；
- HTTP：项目管理、公开发现和幂等调用；
- MCP：`yilong-open-commerce` 的 9 个供应商无关工具；
- PC：项目详情页“开放商业”工作台；
- 安全边界：只允许 `merchant_profile` 与 `static_json`，拒绝任意 HTTP 处理器；
- 资金边界：只记录整数微单位计量，固定为 `recorded_not_charged`。

最终专项证据：

- PC `npm run build`：TypeScript 检查与 Vite 生产构建通过；
- Rust `cargo check --manifest-path server/Cargo.toml --bin elon-server`：通过；
- Rust `cargo test --manifest-path server/Cargo.toml --bin elon-server open_commerce`：8 项通过；
- 纵向测试真实执行“创建项目与商户 → 发布公开/授权能力 → 公开调用 → 幂等重放 → 创建授权 → HTTP 共用服务调用 → MCP 调用 → 计量与审计回读”，并验证公开发现不会泄漏处理器配置或调用原始值。

尚不属于 V1 已验证能力：真实第三方平台连接、真实资金扣款/退款/分账、消费者数据保险箱、多排序器、公开评价、配送、联邦治理和闲置算力市场。
