# 消费者 AI 本人 App 目录 MCP V1

状态：已接受；代码已形成，尚未编译或测试。

## 决策

新增只读工具 `open_commerce_list_my_consumer_apps`，让消费者 AI 查看当前项目中由当前用户拥有的开发者 App，并选择后续 MCP 连接应固定的 `x-elon-app-id`。

响应只包含 App 记录 ID、App ID、显示名、启停状态、沙箱/生产环境、资料状态、已声明范围和更新时间，并标记当前 MCP 身份及 App 是否可用于沙箱 MCP。其他项目成员拥有的 App 被过滤。

## 秘密边界

响应固定 `test_tokens_included=false`、`production_credentials_included=false`，不返回测试 Token、Token 提示、生产凭据、凭据摘要或所有者内部 ID。App 创建、Token 轮换、停用和生产准入继续由现有开发者门户负责。

## 模块边界

工具位于独立 `open_commerce_consumer_app_mcp.rs`。本批同时把 MCP 初始化协议元数据抽到 `open_commerce_mcp_protocol.rs`，使主路由保持在源文件规模门禁以内，不改变协议语义。

## 实现入口

- `server/src/open_commerce_consumer_app_mcp.rs`
- `server/src/open_commerce_mcp_protocol.rs`
- `server/src/open_commerce_mcp.rs`
- `docs/open-commerce-consumer-app-directory-mcp-v1-acceptance.md`
