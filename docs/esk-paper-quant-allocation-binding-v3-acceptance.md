# ESK Paper 量化申请授权、回执与释放 V3 验收

状态：主项目代码已实现并完成本地专项验证；主项目正式发布与量化独立 HTTPS 环境仍待收尾。真实发行、资金、交易、收益和 KYC/地区准入保持关闭。

## 已实现

- 既有 ESK 请求状态机扩展为 `submitted -> canceled` 或 `submitted -> accepted -> released`；submitted/accepted 继续占用，canceled/released 释放。
- PC 量化入口只为用户明确选择的 submitted 请求签发与同次 Paper grant 精确绑定的 `yeqa1` 授权；accepted 请求重新进入时不签发新业务授权，只用于回执恢复。
- 主项目用独立 `YILONG_QUANT_ESK_RECEIPT_KEYRING_JSON` 验证量化 `yqar1` 回执；验证当前用户脱敏 participant、确定性 authorization ID、精确金额、binding、事件修订和 Paper-only 布尔边界。
- SQLite V285 新增 append-only binding 事件与统一当前状态投影视图，不改写 V2 历史请求/取消表；取消与 accepted 回执在立即事务中串行，只能一个分支成功。
- 主项目只保存回执 SHA-256、key ID、binding ID、量化 binding 修订和发生时间，不保存完整回执 token、grant、授权或 participant。

## 验证结果

- `cargo check --manifest-path server/Cargo.toml --bin elon-server`：通过。
- `cargo test --manifest-path server/Cargo.toml --bin elon-server quant_allocation_tests -- --nocapture`：6/6 通过，覆盖余额占用、取消释放、卖回竞争、append-only/restart、accepted/released 精确重放和取消/接收竞争。
- 单申请授权专项 `allocation_authorization_is_deterministic_and_grant_bound`：1/1 通过。
- 量化回执验签专项 `verifies_receipts_and_rejects_unsafe_or_revoked_claims`：1/1 通过。
- `node scripts/test-esk-asset-contract.js`：通过；覆盖 receipt 路由、PC 四状态文案、capability、无 storage/URL 和无经济效果边界。
- PC `npm run build`：通过。
- 为恢复主仓既有测试目标编译，补回 `server/src/store/store_tests.rs` 缺失的 `rusqlite::params` 导入；不改变生产行为。
- 双仓授权 Schema SHA-256：`b63033629337cd8db05693507f8265201b2f9a02b2140eafe7034a786f48f0d7`；回执 Schema SHA-256：`67eac3c1051c83e78220a3aa80a0f14edc739e95f062f7e4132ef94a1f841687`，逐字节一致。

## 明确边界

accepted 只表示量化端建立模拟 binding；它不证明资金到账、成交、QSHARE、NAV、收益或可提现余额。released 只释放主项目 Paper 占用，不代表官方付款或链上卖回。主项目未配置量化回执 keyring、量化端未配置独立 receipt signer，或量化 HTTPS origin 尚未批准时，对应链路失败关闭。

用户操作与恢复说明见 `docs/yilong-quant-esk-allocation-binding-v1.md`；权威需求见 `docs/requirements/esk-paper-quant-allocation-binding-v3.md`。
