---
title: "项目 AI 资源控制面 V1"
status: accepted
reviewed_at: 2026-07-31
---

# 项目 AI 资源控制面 V1

## 背景

项目已经存在用户自己的 Codex、本人 PC 节点、显式授权的共享 Codex 以及平台模型配置，但这些资源分散在不同模块。直接建立新的统一执行器会复制现有路由、凭据和计费逻辑，并可能让“可见资源”被误认为“额度已验证且任务已经执行”。

## 决定

1. V1 只建立统一盘点、项目策略和路由预演，不替代现有执行器。
2. 资源清单由现有真实来源投影生成，不复制密钥、Token 或完整第三方账户信息。
3. 项目可配置资源类型开关、优先级、本地倾向、候选回退和已知单位成本上限。
4. 本地执行要求只能选择当前用户自己的在线节点；未经验证或不属于用户的节点不能冒充本地资源。
5. 共享 Codex 只显示当前用户已获得有效授权的资源。
6. 外部额度在没有实时证据时必须标记 `quota_verified: false`，未知成本不能通过成本上限校验。
7. 路由预演固定返回 `execution_started: false`，不创建任务、不消耗额度、不调用节点。

## 当前边界

- 控制面不是新的模型代理、调度器、算力市场或 Token 交易市场。
- 保存策略不会自动改变所有现有聊天和开发任务的执行路线。
- V1 不验证外部供应商余额，也不承诺所有列出的资源当前可用。
- 真实调度接入需要后续复用现有路由和计量边界，并单独建立回归测试。

## 结果

项目先获得一个安全、可解释的资源视图，用户可以看到候选来源和策略结果，而不会误触发计算。后续真实执行只需把已经验证的策略接到现有调度入口，不需要再次发明资源身份和优先级模型。

## 实现证据

- `server/src/ai_resource_control/`
- `server/src/store/ai_resource_control.rs`
- `server/src/ai_resource_control/tests.rs`
- `pc-frontend/src/features/open-commerce/AiResourceControlPanel.tsx`
- `scripts/test-open-commerce-pc-workspace.js`
