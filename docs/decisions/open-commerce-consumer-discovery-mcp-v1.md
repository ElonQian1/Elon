# 消费者 AI 完整发现 MCP V1

状态：已接受；代码已形成，尚未编译或测试。

## 决策

新增只读工具 `open_commerce_discover_for_consumer`，让 MCP 消费者 AI 复用 PC 沙盒同一套消费者发现领域服务，而不是只调用基础商户目录。

工具可表达搜索词、能力键、五种透明非付费排序器、消费者偏好、城市/类别/标签硬约束、能力类型、访问级别、币种价格、商户声明期、内部同步回执来源及最大年龄，并返回候选范围、匹配原因、授权状态和可选排序凭证。

## 身份边界

- 未设置 `x-elon-app-id` 的默认 MCP 身份映射为只读 `pc-web` 发现身份；受限能力只显示需要注册 App。
- 显式设置 App ID 时，服务端仍要求该开发者 App 属于当前用户，并可返回其现有 Grant 或待审批状态。
- 工具参数不能覆盖调用身份。

## 非目标

- 不自动注册 App、申请授权、创建 Grant、准备动作确认、调用能力、下单或结算。
- 不扩大当前运营方固定 100 个候选窗口，也不构成全网目录。
- 不证明来源字段由美团等外部平台签发或实时回读。

## 实现入口

- `server/src/open_commerce_consumer_discovery_mcp.rs`
- `server/src/open_commerce_consumer.rs`
- `server/src/open_commerce_mcp.rs`
- `docs/open-commerce-consumer-discovery-mcp-v1-acceptance.md`
