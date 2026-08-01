# 开放商业商户目录 V1 验收

## 验收范围

本轮验收覆盖商户主动发布、跨项目脱敏发现、目录撤回、授权 App 身份绑定和 PC 管理入口。它不覆盖生产 App 审核、限流、滥用治理、真实支付、链上提交或大厂生产适配器。

## 已验证行为

1. 新商户默认不进入目录，至少发布一项 `public` 或 `authorized` 能力后才允许显式公开。
2. 跨项目发现只返回 `open_commerce.directory_merchant.v1`，不包含项目 ID、所有者 ID、节点模式、处理器类型、处理器配置、运行地址或密钥引用。
3. `owner_only` 能力不会进入公开目录。
4. 非系统 App 必须属于当前用户；伪造其他 App ID 不能借用其 Grant。
5. 商户撤回目录后，发现结果消失，外部调用和新的授权申请同时关闭。
6. 商户项目编辑者可以在 PC 工作区发布或撤回目录，并看到修订号和脱敏边界。
7. HTTP、MCP 和 PC 工作区复用同一目录发布状态与领域服务。

## 验证命令与结果

```text
Rust check --tests: passed
Rust open_commerce_ tests: 20 passed, 0 failed
Rust consumer authorization follow-up: 1 passed, 0 failed
PC TypeScript/Vite production build: passed
Open commerce PC workspace contracts: passed
Source-size guard: passed
Document modularity guard: passed
```

Rust 验证通过 `scripts/validate-rust.ps1` 和共享验证证据执行；PC 构建通过 `npm run build` 执行。首次验证曾被已退出进程遗留的共享租约阻塞，确认租约所有者已死亡后按验证调度器的租约机制安全回收，随后检查与测试均通过。

## 仍未完成

- 生产第三方 App 的发布审核、密钥轮换策略和跨运营方身份互认。
- 公共 API 限流、配额、滥用检测、封禁与争议处理。
- 真实扣费、支付、退款、跨主体结算和 Sui 网络提交。
- 美团、抖音、京东、淘宝闪购等生产数据适配器。
