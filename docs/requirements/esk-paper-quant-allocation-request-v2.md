---
title: "ESK Paper 量化分配申请与只读占用投影 V2"
status: accepted
implementation_status: verified
owner: platform-assets
priority: p0
reviewed_at: 2026-09-02
---

# ESK Paper 量化分配申请与只读占用投影 V2

## 用户结果

登录用户可以在一龙主项目把本人一部分可用 ESK 提交为“量化 Paper 分配申请”。申请成功后，该数量与卖回申请分别占用，不再出现在可用余额中；在量化项目尚未接收并形成独立模拟仓位前，用户可以取消申请并立即释放占用。

量化 PWA 通过最长五分钟的签名 V2 投影，只读显示同一账号的 ESK 总额、可用额、卖回占用、量化分配申请占用和总占用。它不得据此自动创建份额、交易或收益记录。

## 范围与非目标

本需求只建立 Paper 申请、余额占用、取消和跨项目只读投影：

- 所有响应固定 `simulated=true`、`funds_moved=false`、`chain_status=not_deployed`；
- 不接收、托管或移动用户资金，不发行链上 ESK，不连接 sandbox/testnet/live；
- `submitted` 只表示用户提交了模拟分配申请，不表示量化项目已接收、已经建仓、已经成交或开始产生收益；
- 不把旧 Paper `NET` 仓位改名、折算或并入 ESK；
- 不实现量化端消费、份额生成、NAV 结算后的主项目释放、真实申购赎回或收益分配；这些属于后续版本化合同。

## 主项目状态机

```text
submitted -> canceled
```

1. 创建请求需要本人登录、`ESK_ASSET_MODE=paper`、正数六位精确金额、全局本人幂等键、风险披露版本 `esk-quant-paper-allocation-v2` 和确认文本 `REQUEST PAPER ESK QUANT ALLOCATION`。
2. `submitted` 立即计入量化分配申请占用；卖回占用与量化占用之和不得超过总 ESK。
3. 同一幂等键的相同请求返回原请求；金额、披露版本或确认语义漂移必须冲突。
4. 用户只能取消本人的 `submitted` 请求，确认文本为 `CANCEL PAPER ESK QUANT ALLOCATION`；重复取消返回原终态。
5. 请求和事件都追加写入 SQLite，禁止更新或删除。并发卖回与量化申请必须在同一余额真源和立即事务下竞争，至多一个超额候选成功。

## 本人 API

| 方法 | 路径 | 结果 |
|---|---|---|
| `GET` | `/api/me/assets/esk/quant-allocation-requests?limit=20` | 本人申请列表 |
| `POST` | `/api/me/assets/esk/quant-allocation-requests` | 创建并占用本人可用 ESK |
| `POST` | `/api/me/assets/esk/quant-allocation-requests/:request_id/cancel` | 取消未接收申请并释放占用 |

响应不得返回邮箱、付款资料、钱包、KYC、管理员身份或其他用户 ID。错误必须区分未登录、模式关闭、超额、幂等冲突、不存在和不可取消。

## 账户视图 V2

`GET /api/me/assets/esk` 升级为 `yilong.esk.asset_account.v2`，余额同时返回：

- `total` / `total_base_units`；
- `available` / `available_base_units`；
- `reserved_for_sellback` / `sellback_reserved_base_units`；
- `reserved_for_quant` / `quant_reserved_base_units`；
- `reserved_total` / `reserved_base_units`。

服务端必须满足 `available + reserved_total = total` 以及 `reserved_total = reserved_for_sellback + reserved_for_quant`。修订号和更新时间覆盖资产登记、卖回事件和量化分配申请事件，不能只跟随初始登记。

## 跨仓库签名投影 V2

1. 新合同为 `yilong.esk.asset_projection.v2`，token 前缀为 `yep2`，并增加量化占用、总占用及对应基础单位字段。
2. 投影仍与同次 `yilong.quant.paper_access_grant.v1` 精确绑定 grant ID、脱敏参与者、key、签发窗口和到期时间。
3. 新量化页面在 ready capability 中声明 V2；主项目优先签发 V2。旧页面只声明 V1 时，仅在量化占用为零的情况下签发 V1，避免把已占用数量误报为可用。
4. 双方 `contracts/quant/esk-paper-asset-projection-v2.schema.json` 必须逐字节一致。V1 合同和旧客户端继续可用，不修改其字段语义。
5. 量化项目只验证和显示投影，不保存投影 token 或余额，不提供申请、取消、建仓、卖回或交易写操作。

## 用户界面

主项目 PC 资产卡片显示总额、可用、卖回占用、量化申请占用和总占用，并提供创建、列表和取消入口。文案必须使用“申请占用”“尚未形成量化份额”，不得使用“已投资”“已入金”或“开始收益”。

量化 PWA 的 ESK 卡片独立显示量化申请占用，并说明需要后续接收合同才能形成模拟仓位；旧 V1 投影继续显示原三项，不伪造缺失字段。

加载、零余额、空列表、错误、模式关闭、重复提交和窄屏均需有明确状态；金额只使用服务端精确字符串，不用 JavaScript 浮点数计算账本。

## 验收标准

1. 领域/存储测试覆盖创建、精确幂等、漂移、超额、取消、重复取消、追加式触发器和重启恢复。
2. 并发测试证明卖回与量化申请共享可用额，不能合计超额。
3. HTTP 测试覆盖本人绑定、跨用户隔离、模式失败关闭、列表和错误状态，并确认响应无用户标识泄露。
4. 主项目 V2 投影测试覆盖金额关系、V2 capability 优先、V1 安全回退及有量化占用时拒绝 V1。
5. PC 类型检查、单元/合同测试和生产构建通过；量化仓库的 verifier、API、PWA 和 Paper E2E 通过，并保持 V1 兼容。
6. 主项目和量化项目分别更新当前事实、能力说明和交付证据，分别提交推送；生产只允许 Paper 模式，公网量化部署仍按独立部署门禁报告。

## 回滚

主项目先停止量化 PWA 的 V2 capability 协商，再隐藏 PC 创建入口；已有 `submitted` 请求仍可由本人取消并释放，追加式历史不得删除。回滚不得把量化占用重新解释为卖回占用，也不得用 V1 投影报告错误可用额。
