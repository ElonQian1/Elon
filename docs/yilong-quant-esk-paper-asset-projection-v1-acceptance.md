---
version_status: current
document_type: acceptance_evidence
accepted_at: 2026-09-02
requirement_ref: docs/requirements/yilong-quant-esk-paper-asset-projection-v1.md
main_implementation_commit: 00920f77a619eed52744efa854d8d360a5a42641
quant_implementation_commit: 3efcd23cbe8baac370bbc65ba25335763ddd6b1f
quant_documentation_commit: f4c9666a73e32fa2cebdc18878432bc55d45149a
published_server_version: v0.3.1713
---

# 一龙量化 ESK Paper 资产投影 V1 验收

本轮已把主项目唯一 ESK Paper 余额安全投影到“一龙量化交易”。用户在主项目继续查看并办理 ESK Paper 资产；量化页面只显示经签名、与当前用户授权绑定的余额摘要，不复制账本或提供资产写操作。

## 用户结果

- 主项目 PC/Android 已有 ESK 总额、可用额、卖回申请占用和申请/撤销入口。
- 量化页面现在有独立“一龙 ESK”卡片，显示总额、可用额、卖回申请占用、主项目源修订和同步时间；没有量化仓位时仍可显示。
- 量化卡片明确标记 Paper、尚未上链、不移动真实资金，并说明持有 ESK 不等于已经申购量化份额或自动获得收益。
- 卖回仍回到主项目申请。量化端没有购买、卖回、锁定或收益按钮。

## 安全与兼容

- 主项目从 ESK 唯一账本签发 `yep1` Ed25519 投影，最长五分钟，并绑定同次 Paper grant 的 grant ID、脱敏 participant、签发和到期边界。
- 只有量化页声明 `yilong.esk.asset_projection.v1` capability 时主项目才发送投影；旧量化页面继续使用原 grant-only 消息。
- 投影经 existing exact-origin、window、nonce、attempt 通道传递，不进入 URL、localStorage、sessionStorage、IndexedDB 或 React state。
- 量化 API 拒绝篡改、过期、错误 key/资产/金额、跨用户、跨 grant、缺少 read scope 和已撤销授权；失败响应不返回余额、participant、grant 或 token。

## 验证与发布

- 主项目生产 `elon-server` 目标编译通过；PC 严格 TypeScript/Vite 生产构建、相关 ESLint、ESK 账户、项目主页、官方目录与投影静态合同测试通过。
- 主项目源码体积、文档模块化、所有权、pre-commit 和 pre-push 门禁通过。
- 量化仓库 `validate.ps1` 通过 Rust 格式、全 workspace Clippy/测试和前端构建；前端 11 项测试与扩展 Paper E2E 通过。
- 扩展 E2E 覆盖合法 ESK 视图、跨用户/跨 grant 拒绝、scope、撤销和不泄露 secret；回执确认 Paper、未启用 live、未移动资金、未访问外网、未使用生产秘密。
- 两仓 ESK Schema 字节一致，Paper 启动 Schema 语义一致。
- 主项目实现提交已推送并发布为 Server `v0.3.1713`，线上健康检查返回提交 `00920f77a619eed52744efa854d8d360a5a42641`；新版 PC 前端一并发布。

## 已知边界

- 主项目完整 Rust test target 仍受远端基线既有 Windows SQLite VFS 测试源码错误阻断，错误位于 `node_agent_compute_plugin_host/.../sqlite_vfs_policy`，不在 ESK 变更范围；生产 server 目标已实际编译发布。
- 量化公开 HTTPS origin 和服务器仍未配置，项目广场 Web/Windows/Android 状态继续为 `planned`。主项目上线不代表量化 PWA 已公网可用。
- 当前 ESK 是 Paper 资产登记，不是已部署链上合约、自由转让代币、稳定币、存款、基金份额或真实量化申购凭证。
- 不存在固定价格、保证卖回、保本、固定 6% 或自动收益；真实发行、交易、托管、结算和量化申购必须另立需求并通过法域与安全门禁。

## 回滚

回退主项目实现提交可移除投影签发与 capability 投递；旧 grant-only 量化入口保持兼容。回退量化实现提交可移除 verifier、只读 API 和 ESK 卡片。两边都没有新增数据库迁移、真实订单、链上交易或资金状态。
