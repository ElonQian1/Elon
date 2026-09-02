---
version_status: current
reviewed_at: 2026-09-02
release_status: verified_pending_production_publish
---

# ESK Paper 量化分配申请 V2 验收

状态：主项目源码与隔离运行验收通过，生产发布待本轮发布回执确认；量化子仓库兼容实现已推送。真实发行、资金、仓位、交易与收益仍关闭。

对应需求：[`requirements/esk-paper-quant-allocation-request-v2.md`](requirements/esk-paper-quant-allocation-request-v2.md)

## 用户结果

- 主项目账号页按服务端精确字符串显示 ESK 总额、可用额、卖回占用、量化申请占用和总占用。
- 登录用户可以提交本人 Paper 量化分配申请；`submitted` 立即占用可用额，但明确显示“尚未形成量化份额”。
- 用户可以取消尚未被接收的申请并释放占用；历史请求和事件保持追加式，不通过改写或删除伪造状态。
- 卖回与量化申请共享同一可用余额真源，不能通过并发或分接口合计超额。
- 量化子项目只验签并显示同一份短期 V2 投影；投影不保存、不移动资金，也不创建模拟或真实仓位。

## 实现范围

- SQLite 迁移版本 `284` 增加量化分配请求、追加式事件、索引和禁止更新/删除触发器。
- `GET/POST /api/me/assets/esk/quant-allocation-requests` 提供本人列表与创建；`POST /api/me/assets/esk/quant-allocation-requests/:request_id/cancel` 提供本人取消。
- `GET /api/me/assets/esk` 返回 `yilong.esk.asset_account.v2`，并满足 `available + reserved_total = total` 与 `reserved_total = reserved_for_sellback + reserved_for_quant`。
- 主项目优先签发 `yilong.esk.asset_projection.v2` / `yep2`；有量化占用时拒绝不含该字段的 V1，避免误报可用余额。
- PC 资产功能位于 `pc-frontend/src/features/assets/`，没有把新界面写回旧 PC 资源。

## 跨仓库证据

- 主项目和量化子项目的 `contracts/quant/esk-paper-asset-projection-v2.schema.json` 逐字节一致，SHA-256 为 `ba3748fe22122e99271b5b6a0aeaa7fd61206557f22e67c807c26a0a97036c57`。
- 量化子仓库实现提交 `1210e8b`、Feature Registry 发布提交 `bc9d3de` 已推送到 `yilong-quant/main`。
- 量化仓库定向 verifier/API 测试、前端 11 项测试与生产构建、全仓 `scripts/validate.ps1` 和离线 Paper E2E 均通过。
- 量化公开 HTTPS origin 尚未获批和部署，所以当前不能把本地/离线验收写成公网可用。

## 主项目运行证据

2026-09-02 在隔离 worktree、临时 SQLite 和临时本地服务完成：

- 生产 `elon-server` 二进制 `cargo check` 与 `cargo build` 通过。
- HTTP E2E 通过匿名 `401`、本人/他人隔离、创建、列表、超额拒绝、跨用户取消 `404`、取消恢复和分项余额关系。
- 测试用户登记 `12.500000 ESK` 后申请 `4.250000 ESK`，可用额变为 `8.250000`；取消后恢复 `12.500000`，状态从 `submitted` 变为 `canceled`。
- E2E 确认响应固定 `simulated=true`、`funds_moved=false`、`position_created=false`，未使用生产用户、生产密钥或外部网络。
- PC 严格 TypeScript/Vite 生产构建、ESK API/UI 静态合同及量化投影合同测试通过。
- 完整 Rust test target 仍被仓库既有、与 ESK 无关的 SQLite VFS 测试编译错误阻断；本功能生产二进制编译、领域源码审查和真实 HTTP 路径均已单独通过，不能把该既有阻断隐去或误报为全仓测试通过。

## 安全边界

本版本中的 ESK 仍为主项目 Paper 账本记录，不是链上代币证明。`submitted` 只表示余额被申请占用，不表示已经入金、被量化项目接收、形成份额、成交或开始产生收益。卖回仍是向官方提交的 Paper 申请，不承诺价格、时限、流动性或固定回购；真实资产阶段必须另行完成法域、托管、安全和合规门禁。
