# 消费者发现请求输入边界 V1 验收

状态：`verified_local`

## 已验证能力

- 服务端规范化搜索词、能力键、调用方 App ID 和返回数量。
- 空可选值不再作为无效精确条件进入目录查询。
- SQL LIKE 特殊字符按普通字符查询。
- 规范化后的返回数量进入结果截断和排序凭证输入指纹。
- `limit` 的 1 至 50 约束位于共享消费者发现服务，HTTP、MCP 和 PC 不需要分别实现夹取规则。

## 本地验收

- `sql_like_metacharacters_are_literal_and_reads_have_no_side_effects`：用真实 SQLite 目录验证 `%`、`_`、反斜杠只匹配其字面字符，空白搜索词被规范化，查询不改商户或审计状态。
- `query_capability_and_app_identifiers_share_one_normalization_boundary`：覆盖 200/201 个 Unicode 字符、控制字符、能力键大小写与空白、空能力键、非法能力键/App ID、缺失 App 和空 App 映射 `pc-web`。
- `result_limit_candidate_cap_and_receipt_fingerprint_use_validated_values`：用 105 个已发布商户覆盖 0/1/50/51/超大 `limit`、固定 100 候选窗口、结果截断和排序凭证中的返回数量及请求指纹。

验证命令：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\validate-rust.ps1 -Domain open-commerce-consumer-inputs -- test --manifest-path server\Cargo.toml open_commerce_consumer::input_tests -- --nocapture
```

验证指纹：`8bb7344c8aa68112092116aa0eacbe70e033914943e97c3f9f6389d0f33465a6`。

发现 MCP 回归指纹：`7275ca6676ce32c5c9f23f00029fc3d7291c4643278ffc9fd2136c3adf7087f4`。

## 未覆盖边界

本批直接调用共享领域服务和真实 SQLite Store，并回归 MCP 路由；未启动 HTTP 服务或 PC 页面，因此不构成实际 HTTP 状态码、前端控件或端到端网络验收。测试不证明目录完整、商户数据真实、全网最优或生产部署。
