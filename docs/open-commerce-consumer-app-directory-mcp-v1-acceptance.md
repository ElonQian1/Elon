# 消费者 AI 本人 App 目录 MCP V1 验收

状态：`verified_local`

## 已验证能力

- 只列出当前项目中当前用户拥有的 App。
- 返回当前 MCP 身份、App 状态和沙箱可用标记；只有 `active + sandbox` 才可用于沙箱 MCP。
- 显式 `x-elon-app-id` 必须属于当前用户和当前项目；同项目其他用户、本人其他项目和未知 App 均失败关闭。
- 不返回测试 Token、Token 提示、生产凭据或其他用户标识。
- MCP 协议初始化说明已从主路由抽到独立模块，源文件规模门禁通过。

## 本地验收

- `default_identity_lists_only_current_users_project_apps_without_secrets_or_writes`：覆盖默认身份、多人和跨项目隔离、活跃/停用状态、秘密键与真实秘密值缺失，以及读取前后 App 和审计快照不变。
- `explicit_owned_active_or_disabled_identity_is_marked_and_routed`：覆盖真实 MCP 路由、本人活跃 App 当前项和本人停用 App 的不可用标记。
- `teammate_cross_project_and_unknown_explicit_identities_fail_closed`：覆盖同项目其他用户、本人其他项目和未知显式身份失败关闭且无写入。
- `empty_directory_and_argument_contract_are_stable`：覆盖空目录、额外参数拒绝、未知工具旁路、只读注解和初始化说明。

验证命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain open-commerce-consumer-apps -- test --manifest-path server\Cargo.toml open_commerce_consumer_app_mcp::tests -- --nocapture
```

验证指纹：`0243f2e8622f7d73ccfd15dc9776439e170bb12f8ad357e9212b31385e7d43cf`。

## 未覆盖边界

本批使用本地 SQLite 假数据，不创建或使用真实生产凭据，不连接外部商业平台，不证明生产部署、跨运营方身份互认或真实交易完成。App 创建、Token 轮换、停用和生产准入仍由现有开发者门户负责。
