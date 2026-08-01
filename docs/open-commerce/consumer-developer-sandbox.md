# 消费者发现与第三方应用沙盒

本文说明已实现的消费者发现、应用注册、授权审批和开发者调用闭环。沙盒边界由 `docs/decisions/open-commerce-consumer-developer-sandbox-v1.md` 决定，跨项目目录发布由 `docs/decisions/open-commerce-directory-publication-v1.md` 决定，应用与申请生命周期由 `docs/decisions/open-commerce-developer-lifecycle-v1.md` 决定。

## 使用入口

项目详情的“开放商业”区域包含：

- **消费者沙盒**：选择请求 App，按搜索词、能力、城市、标签和价格上限发现商户能力。
- **开发者**：注册沙盒 App、轮换一次性测试 Token、处理授权申请并调试能力调用。

发现结果必须展示是否存在付费排序。当前实现固定采用非付费排序；分数来自公开资料、能力匹配和显式偏好，不能通过购买排名改变。

## 最小闭环

```text
商户编辑者主动发布脱敏目录
  -> 项目编辑者注册测试 App
  -> Token 只显示一次
  -> 消费者沙盒以该 App 发现能力
  -> 对 authorized 能力提交授权申请
  -> 商户项目批准或拒绝
  -> 使用 App 身份和 Grant 调用
  -> 复用幂等、计量与审计
```

公开能力可由 `pc-web` 直接调用。受限能力必须使用独立 App；`pc-web`、未知 App 和 `owner_only` 能力都不能绕过授权。非系统 App 还必须证明 App 归当前用户所有，不能通过伪造请求头借用其他 App 的 Grant。

## HTTP 接口

| 用途 | 方法与路径 |
|---|---|
| 注册或列出项目测试 App | `GET/POST /api/projects/{project_id}/open-commerce/developer-apps` |
| 轮换一次性测试 Token | `POST /api/projects/{project_id}/open-commerce/developer-apps/{record_id}/rotate-token` |
| 停用或重新启用 App | `POST .../developer-apps/{record_id}/disable` 或 `reactivate` |
| 查看项目授权收件箱 | `GET /api/projects/{project_id}/open-commerce/authorization-requests` |
| 批准或拒绝申请 | `POST .../authorization-requests/{request_id}/approve` 或 `reject` |
| 查看或撤回本项目发出的申请 | `GET .../outbound-authorization-requests`、`POST .../{request_id}/cancel` |
| 发布或撤回商户目录 | `PUT /api/projects/{project_id}/open-commerce/merchants/{merchant_id}/directory-publication` |
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

当前已实现商户主动选择的跨项目基础目录，以及沙盒 App 停用、旧 Token 永久失效、重新启用生成新 Token、申请方查看和撤回的生命周期闭环。通过该验收仍不代表生产公共网络已经完成；生产应用审核、跨运营方身份互认、限流、滥用处置、支付和真实平台适配器仍是后续模块。
