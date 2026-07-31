# 消费者发现与第三方应用沙盒

本文说明已实现的消费者发现、应用注册、授权审批和开发者调用闭环。产品边界由 `docs/decisions/open-commerce-consumer-developer-sandbox-v1.md` 决定。

## 使用入口

项目详情的“开放商业”区域包含：

- **消费者沙盒**：选择请求 App，按搜索词、能力、城市、标签和价格上限发现商户能力。
- **开发者**：注册沙盒 App、轮换一次性测试 Token、处理授权申请并调试能力调用。

发现结果必须展示是否存在付费排序。当前实现固定采用非付费排序；分数来自公开资料、能力匹配和显式偏好，不能通过购买排名改变。

## 最小闭环

```text
项目编辑者注册测试 App
  -> Token 只显示一次
  -> 消费者沙盒以该 App 发现能力
  -> 对 authorized 能力提交授权申请
  -> 商户项目批准或拒绝
  -> 使用 App 身份和 Grant 调用
  -> 复用幂等、计量与审计
```

公开能力可由 `pc-web` 直接调用。受限能力必须使用独立 App；`pc-web`、未知 App 和 `owner_only` 能力都不能绕过授权。

## HTTP 接口

| 用途 | 方法与路径 |
|---|---|
| 注册或列出项目测试 App | `GET/POST /api/projects/{project_id}/open-commerce/developer-apps` |
| 轮换一次性测试 Token | `POST /api/projects/{project_id}/open-commerce/developer-apps/{record_id}/rotate-token` |
| 查看项目授权收件箱 | `GET /api/projects/{project_id}/open-commerce/authorization-requests` |
| 批准或拒绝申请 | `POST .../authorization-requests/{request_id}/approve` 或 `reject` |
| 消费者发现 | `POST /api/open-commerce/sandbox/discover` |
| 提交授权申请 | `POST /api/open-commerce/authorization-requests` |
| 使用测试 Token 调用 | `POST /api/open-commerce/developer/invoke` |

开发者调用使用 `Authorization: Bearer <test-token>` 和 `x-elon-app-id`。Token 不得进入 URL、日志、项目文档、浏览器本地存储或商户能力元数据。

## 验收

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 `
  -TargetDir D:\rust\shared\target -- test `
  --manifest-path server\Cargo.toml `
  open_commerce_client_service_tests::consumer_discovery_request_approval_and_test_token_invocation_form_a_loop

Set-Location pc-frontend
npm run test:open-commerce
```

通过沙盒验收不代表公共网络已经完成。跨项目索引、生产应用审核、限流、滥用处置、支付和真实平台适配器仍是后续模块。
