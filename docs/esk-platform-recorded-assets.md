---
title: "ESK 平台登记服务接入与操作边界"
version_status: current
reviewed_at: 2026-09-04
owners: [platform-assets]
---

# ESK 平台登记服务接入与操作边界

需求真源：[平台登记 V1](requirements/esk-platform-recorded-assets-v1.md)。
阶段目标：[首批用户路线图](requirements/esk-first-user-delivery-roadmap-v1.md)。
本文件解释代码合同；部署与用户验收以交付记录为准，不因存在接口即视为已上线。

## 与旧资产的关系

ESK 仍是主项目同一种服务与权益参与资产。新服务仅保存经人工审核的正式平台登记，
不改变代币经济设计，也不把旧模拟分录升级成真实付款。主项目持有唯一正式账本；
量化端只应通过经验证的本人授权读取，不复制一个可自行增发的正式余额。

| 数据源 | 接口 | 含义 |
| --- | --- | --- |
| 原有 Paper V2 | `GET /api/me/assets/esk` | 保持现有模拟与占用合同，不混入正式记录 |
| 正式平台登记 V1 | `GET /api/me/assets/esk/platform` | 已明确批准的 ESK 登记数量；不是链上余额 |
| 完整审核流水 | `GET /api/me/assets/esk/platform/history` | 同快照逐页查账；摘要变化重新读取 |
| 正式卖回申请 V1 | 独立 `platform/sellback-requests` 合同 | 开发中的申请与占用切片，生产默认关闭；不是成交或付款 |
| Sui 观察 | 独立观察器合同 | 发行证据观察，不直接产生本接口分录 |

双 APK 本人资产授权仍使用既有合同。主 APK 独立个人入口的开发与验收见
[正式登记个人入口 V1](requirements/esk-platform-profile-v1.md)，不原地扩展 Paper 合同；
禁止在客户端把两者直接相加、把 `platform_recorded` 显示成 `onchain`，或把此数量
标为可立即提现。真实发行和迁移未来应通过有审计来源的结转降低平台数，避免双计。

## 配置与人工核对

默认没有写入能力。生产启用必须另行确认付款网络/资产、总额度、销售条款、历史完整性
与审批责任人；本批开发和测试不会替用户填写这些实际数据。

- `ESK_PLATFORM_ASSET_MODE`：缺省或 `disabled` 为关闭；唯一启用值 `platform_recorded`。
- `ESK_PLATFORM_ASSET_POLICY`：严格 JSON，包含 `source` 与 `issuance_limit_base_units`。
  source 字段为 `namespace`、`network`、`asset_symbol`（仅 USDT）、`asset_reference`、
  `decimals`（0–18）和 `reference_format`（`hex32` 或 `opaque`）。
  额度为正整数字符串，单位是 ESK 六位最小单位，而不是 USDT 数量。
- 第一次成功准备会固定该政策。此后换网络、namespace、资产或额度均拒绝；
  没有在线修改政策接口。关闭写入不隐藏已存在的本人正式记录。

不要将这里的字段说明当生产参数。先按既有只读付款预演核对用户、付款事件、用途、
销售比例和历史，但预演报告本身没有入账权。审核材料在受控位置保存，服务只保存摘要；
摘要不能证明材料真实，也不替代逐笔外部付款核对。

## 准备、确认与纠错

本节付款登记端点只接受真实数据库用户会话；普通用户只能读本人登记。正式入账需 active
全局 admin/owner，项目编辑者、静态管理员 token、虚拟 `OWNER_TOKEN` 均不够。
确认事务内重新检查权限、会话有效期和用户状态。不要通过创建虚拟用户或修改角色绕过。

1. `POST /api/admin/assets/esk/platform-allocations/prepare`：提交
   `yilong.esk.platform_allocation_input.v1`。确切字段以 `model.rs::PrepareBody` 为准；
   金额必须是字符串。必须说明 `esk_purchase` 用途、完整历史、实际核对的付款/同意/
   历史材料摘要、销售条款和比例。服务不猜 1:1、不接受浮点比例、不创建用户。
2. 准备回执返回 `allocation_id` 和 `request_digest`，此时余额不增加。
   操作者须重新核对该申请与实际资料，不能把自动脚本预演当人工确认。
3. `POST /api/admin/assets/esk/platform-allocations/{allocation_id}/record`：
   提交准确的 `expected_request_digest` 与
   `confirmation: "APPROVE AND RECORD PLATFORM ESK"`。原子写入审批及正式分录。
   同一申请重试返回原回执，`replayed=true`、`balance_written=false`。
4. 未记账申请填错：对同 ID 的 `/cancel` 提交原摘要与
   `confirmation: "CANCEL PLATFORM ESK PREPARATION"`。取消留下不可变回执，之后
   可重新准备；旧申请不能再确认。已记账不能取消，取消不是退款或冲销功能。

付款键绑定政策来源、规范化交易引用和事件索引，而不是人工批次或用户；换批次、改用户
不能重记。同一交易的多个事件必须使用实际事件索引，不能用虚构索引绕过防重。
完整输入重放复用未取消申请；输入不同返回冲突。审批与分录要么都写入，要么都不写入。

## 本人只读响应与错误

本人 GET 支持 `limit=1..100`，默认 20；不能传入其他 `user_id`。
`total`、`total_base_units`、`entry_count` 均为精确字符串。
返回最近分录及 `history_has_more`，总量来自全部已批准分录，而不是本页相加。
原账户 V1 不新增字段；完整历史使用独立 [分页合同](requirements/esk-platform-history-v1.md)，
前端不能把账户摘要中的最近记录标成完整历史。

固定字段：`source=platform_recorded`、`chain_status=not_deployed`、`simulated=false`、
`funds_moved=false`、`external_payment_verified=false`。已登记表示经已认证操作者人工审核，
不表示程序自动验证了 USDT 付款；空记录也不是链上零余额证明。
服务消费、量化认购、官方回购成交和链迁移 capability 当前均为 false。
用户响应不带原始付款引用、操作者、材料摘要或其他用户身份。

认证失败为 401，角色/事务权限拒绝为 403，输入错误为 400，找不到或停用用户为 404，
重复冲突/政策漂移/超额度为 409，写入关闭或配置无效为 503。
意外存储错误返回固定代码，不回显 SQL 或敏感请求。响应禁止缓存；这不是传输加密。
不因本接口上线改变现有私人凭据传输策略，不能将真实会话放入日志或明文公共请求。

正式卖回的申请、占用和本人取消按 [独立需求](requirements/esk-platform-sellback-requests-v1.md)
实现，不改变上述账户及 Paper 响应。申请只占用经审核的正式量，取消不新增 ESK。
其政策开关与付款入账开关分离；暂停新增不应阻止查询和取消合法的旧申请。
网络结果未知需查回本人原申请，不可自动重新建单；真实上线仍需独立政策批准与本人验收。
用户路径、异常恢复和远程分工见 [卖回申请使用说明](esk-platform-sellback-requests.md)。

## 开发与交接

源码分为 `server/src/esk_platform/` 的模型/校验/HTTP/迁移和
`server/src/store/common/esk_platform_assets/` 的事务/读取/取消。
V287 只新增独立表，不回填旧 Paper。验证入口使用 `scripts/validate-rust.ps1`：

- 独立源码复用测试：`test --manifest-path server/tests/esk-platform-harness/Cargo.toml`。
- 真实 Store/Router：`test --manifest-path server/Cargo.toml --bin elon-server esk_asset::platform::http_tests`。

CI 的 `ESK Platform Ledger Tests` 运行独立测试；原有完整 Cargo Test 仍负责真实后端测试。

独立测试不能替代生产编译、迁移、部署或实际用户验收。后续分工：平台代理维护单账本
及审核操作入口，主 APP 代理接入正式来源，量化代理遵循版本化授权合同；链迁移由独立
切片负责。功能状态以 Feature Registry `esk-platform-recorded-assets-v1` 为准。

主服务账本的首次发布证据为 2026-09-04 `0.3.1724`，来源
`5fc7869b5b2560417af26b33f0b09ca749fc9bb1`；公开健康与四条路由无认证 401/no-store
已检查。这只证明后端发布与访问控制，不代表已配置写入、给真实用户记账或 APP 已联调。
