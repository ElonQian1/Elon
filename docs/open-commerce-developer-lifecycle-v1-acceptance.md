# 开放商业开发者生命周期 V1 验收

## 验收范围

本轮覆盖沙盒 App 停用与重新启用、旧 Token 永久失效、待处理申请自动取消、申请方列表和主动撤回，以及商户审批前的 App 状态复核。它不覆盖生产身份、公开注册、限流、支付或链上资产。

## 已验证行为

1. 项目编辑者可以停用 App，停用后登录身份和测试 Token 都不能继续代表该 App。
2. 停用会用不可恢复的新摘要替换旧 Token，并自动取消该 App 的待处理申请。
3. 重新启用会生成新的仅显示一次 Token；旧 Token 仍然无效。
4. 申请方项目可以读取其 App 发出的申请，并撤回 `pending` 申请。
5. 同一申请在开发者侧和商户侧共享 `canceled` 状态，不维护两份副本。
6. 商户不能批准已停用 App 的申请。
7. PC 门户提供状态、停用、重新启用、Token 轮换、发出申请列表和撤回入口。

## 验证结果

```text
Rust check --tests: passed
Rust open_commerce_ tests: 20 passed, 0 failed
PC TypeScript/Vite production build: passed
Open commerce PC workspace contracts: passed
Source-size guard: passed
Document modularity guard: passed
```

Rust 通过 `scripts/validate-rust.ps1` 在共享验证队列中运行，测试证据状态为 `success`；PC 通过全新依赖安装后的 `npm run build` 验证。首次 PC 日志包装命令因组合命令被误解析为 `npm ci;` 而失败，拆分为独立 `npm ci` 与 `npm run build` 后均通过，该次失败不属于产品代码失败。

## 仍未完成

- 生产 App 审核、跨运营方身份互认、密钥轮换策略和开发者组织管理。
- 公共 API 配额、限流、异常调用检测、封禁、申诉与争议处理。
- 真实扣费、支付、退款、结算和 Sui 网络提交。
