---
title: "ESK Paper 双仓可见余额 V5 验收"
status: verified
requirement_ref: docs/requirements/esk-paper-quant-cross-repo-asset-view-v5.md
verified_at: 2026-09-02
---

# ESK Paper 双仓可见余额 V5 验收

## 交付结果

- 主项目真实 Paper 签名器和 V2 资产投影序列化器复算固定 `yep2`；生产签发仍使用随机 projection ID，固定 ID 只存在于模块测试缝。
- 量化项目真实 verifier 将 `yep2` 与同次 `ypg1` 的 key、grant、脱敏 participant、观察时间和到期时间绑定，篡改失败关闭。
- 量化 API 只生成脱敏 ESK 视图，不返回 grant、投影 token、key 或 participant；前端对精确结构、六位小数、基础单位和余额等式执行运行时校验。
- React 实际渲染固定测试向量的总额 `1250.000000`、可用 `900.000000`、卖回占用 `100.000000`、量化占用 `250.000000`、总占用 `350.000000 ESK`，并显示 Paper、未上链、未移动资金和尚未形成量化仓位。
- 为避免验证单一资产合同却链接整个巨型服务器，签名器与金额规则拆成纯核心模块；隔离契约包直接包含同一生产源码，不复制第二套协议实现。

## 固定身份与摘要

- 主项目实现提交：`73b6f09f1e50307c7e490e768e1206e92c54489b`。
- 量化项目实现提交：`6253f4eaedbf690698e064e5f582a3da5abc6537`。
- fixture 规范 LF SHA-256：`3984e97f16ce83ec401dcf4494f10f11f5f7e8379d26f367fa68a4a8fd9ddf8e`。
- V2 Schema 规范 LF SHA-256：`ba3748fe22122e99271b5b6a0aeaa7fd61206557f22e67c807c26a0a97036c57`。
- 上述余额只属于公开固定测试向量，不代表任何真实用户余额。

## 验证证据

- 主项目统一离线门禁与轻量核心套件通过：9 项测试覆盖 ESK 金额、Paper 签名、V1/V2 投影、余额失败关闭和固定向量；验证指纹 `7926175533f99334a25d4e4602e9603e054a87aeb576976837dbf40af786a865`。
- 量化项目 `scripts/validate.ps1` 通过 Rust fmt、全 Workspace clippy/test、前端 TypeScript 与生产构建；专项验收通过真实 verifier 测试和 15 项前端测试。
- 双仓 runner 在上述两个已推送、干净工作树执行通过，回执记录 `main_worktree_dirty=false`、`quant_worktree_dirty=false`，并核对共享摘要、真实签名/验签、脱敏视图、运行时校验、React 显示和篡改拒绝。
- 回执固定为 `trading_mode=paper`、`live_trading_enabled=false`、`chain_token_issued=false`、`funds_moved=false`、`external_network_used=false`、`production_secrets_used=false`。

## 已知验证边界

当前 Windows 主机在链接巨型 `elon-server` 测试目标以及单并发、无调试符号的全服务器 check 中均由 rustc 以 OOM/`STATUS_STACK_BUFFER_OVERRUN` 终止，没有产生本功能的 Rust 编译诊断；因此这两次尝试不计为通过。现有 pre-push 门禁、纯核心生产源码套件和双仓验收均通过，后续仍应在更高可用内存的 CI/节点补跑完整主服务器 check。

## 未宣称

本交付没有导入真实用户、付款、钱包或 KYC 数据，没有发行链上 ESK、量化份额或 RWA，没有执行真实卖回、交易、收益或公网部署。这里的 `released` 只表示代码、测试和文档进入两仓 `main`，不表示产品已经对真实用户开放。
