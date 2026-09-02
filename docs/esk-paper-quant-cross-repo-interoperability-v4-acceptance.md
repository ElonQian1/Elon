---
version_status: current
reviewed_at: 2026-09-02
release_status: released
---

# ESK Paper 双仓签名互操作 V4 验收记录

## 交付结论

一龙主项目与 `yilong-quant` 已使用同一份固定、公开、仅供测试的 ESK Paper 向量完成字节级互操作验收。主项目真实签名代码生成的 `ypg1` grant 与 `yeqa1` 单申请授权可被量化端真实 verifier 接受；量化端真实 signer/store 生成的 accepted/released `yqar1` 回执可被主项目真实 verifier 接受。

这证明两仓当前代码对签名域、序列化、授权绑定、回执链和密钥撤销规则的理解一致，不代表链上发币、真实申购、收益产品或公网量化环境已经启用。

## 固定身份与摘要

- 主项目提交：`701457943501f270983410daed4382688387200a`。
- 量化项目提交：`c08b5af571a20ee84472779ad8ede17a6a942095`。
- 共享 fixture 原始 SHA-256：`9499dcb52d911a0b999568c02d5ae1b8a13e9ca791481746abb6507550d82c97`。
- 授权 Schema 规范换行后 SHA-256：`b63033629337cd8db05693507f8265201b2f9a02b2140eafe7034a786f48f0d7`。
- 回执 Schema 规范换行后 SHA-256：`67eac3c1051c83e78220a3aa80a0f14edc739e95f062f7e4132ef94a1f841687`。

## 验证结果

- 双仓离线 runner：通过；主项目 4 项互操作测试、量化项目 1 项端到端互操作测试全部通过。
- runner 在两个已推送且干净的工作树执行，回执记录 `main_worktree_dirty=false`、`quant_worktree_dirty=false`。
- 主项目官方定向 Rust 验证：4 项通过，验证指纹 `079635ffc20f0ee29ca3d38c0c6d9f979fe59ea613d2f6ef4f13061c7bcc33b2`。
- 量化项目完整 `scripts/validate.ps1`：通过；覆盖 Rust fmt、Workspace clippy/test、TypeScript 与生产前端构建。
- 量化 Paper E2E：8 项检查通过，包含 `esk_cross_repo_signed_vectors`。
- 篡改 token、错误前序摘要和 revoked key 均按预期拒绝。

## 安全边界

本次只使用合成标识、固定测试时间和测试密钥种子；输出不包含完整 token 或私钥。验收回执明确记录 `trading_mode=paper`、`live_trading_enabled=false`、`funds_moved=false`、`external_network_used=false`、`production_secrets_used=false`。

没有发行链上 ESK，没有导入真实用户或付款，没有创建 NET/QSHARE/NAV/交易/收益，也没有配置量化 HTTPS origin、生产 keyring 或服务器。因此当前仍不能宣称量化环境已公开部署，用户界面只能继续展示主项目已有的 ESK Paper 资产与申请状态。

## 复验入口

在两个仓库均已存在且无需联网的机器上运行 `scripts/test-esk-paper-cross-repo-interoperability.ps1 -QuantProjectPath <量化仓库路径>`。脚本先核对共享文件摘要，再分别调用两仓自己的 Rust 缓存与定向测试入口。
