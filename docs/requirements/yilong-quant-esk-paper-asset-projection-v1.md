---
version_status: current
reviewed_at: 2026-09-02
implementation_status: accepted
---

# 一龙量化 ESK Paper 资产投影 V1

## 目标

登录用户从主项目安全进入“一龙量化交易”后，量化页面应直接显示同一账号在主项目登记的“一龙 ESK”总额、可用额、卖回申请占用额、账本修订和同步时间。主项目继续是 ESK 唯一余额真源；量化项目只消费最长五分钟的签名只读投影，不创建第二份余额。

## 用户体验

- 主项目账号页与量化页面使用相同名称“一龙 ESK”和符号 `ESK`。
- 用户即使没有量化模拟仓位，只要启动消息携带有效投影，仍能看到自己的 ESK 资产摘要。
- 页面持续显示 `PAPER / SIMULATED`、尚未上链、不移动真实资金和来源为一龙主项目。
- 量化页面显示主项目账本修订和本次投影时间，让用户可以判断数据的新鲜度；关闭或刷新后需重新从主项目进入。
- 量化页面不提供增加、锁定、申购、转让或卖回结算动作；卖回申请继续只在主项目操作。

## 版本化签名投影

1. 主项目在签发 `yilong.quant.paper_access_grant.v1` 的同一次一键启动中读取本人 ESK 账户，并签发独立 `yilong.esk.asset_projection.v1`。
2. 投影固定绑定 `issuer=yilong-main`、`audience=yilong-quant`、同一 `participant_ref`、ESK 固定身份、精确余额字符串、源账本修订、观察时间、到期时间、`simulated=true` 与 `funds_moved=false`。
3. 投影使用与 Paper grant 相同的当前 Ed25519 key ID 和轮换信任集合，但使用独立 token 前缀及 Schema；签名私钥和用户脱敏密钥不离开主项目。
4. 投影到期不得晚于同次 grant，最长五分钟。量化服务必须同时验证 grant 和投影，并拒绝篡改、过期、未知 key、错误资产、错误阶段或 `participant_ref` 不一致。
5. 投影只描述读取时的主项目账户事实，不授权量化项目修改 ESK、创建量化份额、提交交易或结算卖回。

双方仓库各保存一份完全一致的 `contracts/quant/esk-paper-asset-projection-v1.schema.json`，合同同步由跨仓库测试固定。

## 启动协议兼容

- 量化页面的 `ready` 消息只有在支持本能力时才声明 `yilong.esk.asset_projection.v1` capability。
- 主项目只有看到该精确 capability 才在既有 exact-origin、window、nonce、attempt 绑定消息中附带投影 token。
- 旧量化页面不声明 capability，主项目继续只发送原 grant；新量化页面在旧主项目下也继续接受不含投影的原协议，并明确显示投影未提供。
- grant 与投影只保存在当前 React 内存和单次 API 请求中，不进入 URL、浏览器历史、DOM 文本、日志、剪贴板、localStorage、sessionStorage、IndexedDB 或量化 SQLite。

## 安全与产品边界

- 本功能只读取现有 ESK Paper 账本，不新增管理员登记、用户自增、锁定、兑换、支付、链上发行或真实卖回能力。
- ESK 余额与量化模拟 NET/QSHARE、NAV、收益和退出金额保持独立；量化页面不得把持有 ESK 描述为已经申购或自动产生收益。
- `ESK_ASSET_MODE=disabled` 仍可投影已经登记的只读事实；未知/非法模式失败关闭。投影不改变写入门禁。
- 未登录、非 active 账号、ESK 读取失败、签名配置无效或量化地址无效时不得返回启动票据。
- 本功能不证明链上 Coin、付款、钱包、KYC/AML、地区或投资适当性，也不接收、托管或移动用户资金。

## 验收标准

1. 主项目合同和单元测试证明投影与同次 grant 绑定同一脱敏用户、同一签名 key、最长五分钟有效期，并精确携带 ESK 总额、可用额、占用额、修订和 Paper 边界。
2. 主项目一键启动在量化页声明 capability 时只通过既有 exact-origin 内存通道发送 grant 与投影；未声明时保持 V1 兼容；源码检查证明两类 token 不进入 URL 或浏览器持久化。
3. 量化 API 同时验签 grant 与投影，覆盖合法、篡改、过期、错误 key、错误资产、金额不一致和跨用户投影，失败时不返回任何余额。
4. 量化 PWA 在有/无模拟仓位、零余额和卖回占用场景都正确显示独立 ESK 卡片、源修订、同步时间、尚未上链和不移动资金；不渲染资产写操作。
5. 双仓库合同文件字节一致，Rust/TypeScript/生产构建、Paper 端到端验收和文档门禁通过；公开部署未就绪时继续标记为未部署。

## 预计实现范围

- 主项目：`contracts/quant/`、`server/src/quant_*`、`server/src/esk_asset/`、`pc-frontend/src/features/conversation/` 及定向测试。
- 量化项目：相同合同、`crates/paper-access/`、`apps/api/`、`frontend/src/features/position/` 及 Paper E2E。
- 双方：当前事实、能力基线、关系/集成文档和项目广场净化能力摘要。

## 回滚

先从量化 `ready` 消息移除 ESK capability，即可让主项目停止发送投影而保留原 Paper grant 和仓位入口。随后可分别回滚两侧代码；本功能无数据库迁移、无余额写入、无资金或链上副作用。
