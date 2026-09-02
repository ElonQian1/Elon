---
status: accepted
owner: platform-assets, quant-integration
reviewed_at: 2026-09-02
---

# ESK Paper 双仓可见余额互操作 V5

## 目标

证明用户从一龙主项目进入量化项目后，量化端真实验证并展示的 ESK Paper 总额、可用额、卖回占用和量化申请占用，与主项目同一账本签出的 `yep2` 投影逐字节、逐字段一致。

本需求补足 V4 未覆盖的资产投影与最终显示链，不改变 ESK Paper 账本、申请绑定或量化仓位语义。

## 共享测试向量

两仓新增同路径、同原始字节的 `contracts/quant/esk-paper-cross-repo-asset-view-v1.fixture.json`。它只包含固定公开测试密钥的公钥、合成身份、固定短期 grant、固定 `yep2` 投影和脱敏预期视图，不含测试私钥种子、真实账号、付款、钱包、KYC 或生产配置。

向量固定表示：

- 总额 `1250.000000 ESK`；
- 可用 `900.000000 ESK`；
- 卖回占用 `100.000000 ESK`；
- 量化申请占用 `250.000000 ESK`；
- 全部占用 `350.000000 ESK`；
- `chain_status=not_deployed`、`simulated=true`、`funds_moved=false`、`position_created=false`。

## 主项目责任

1. 真实 `PaperGrantSigner` 和 V2 资产投影序列化器必须使用固定测试种子与固定 projection ID 复算出 fixture 中的 `ypg1`/`yep2`。
2. 生产签发继续生成随机 projection ID；固定 ID 只允许由模块内测试调用，且必须经过与生产相同的身份、金额、有效期和签名校验。
3. 双仓 runner 必须核对新 fixture 原始 SHA-256、V2 Schema 规范换行后的 SHA-256，并调用两仓定向测试及量化前端测试。
4. runner 回执只输出提交、摘要、测试类别和安全布尔值，不输出 grant、投影 token 或测试种子。
5. Paper 签名器与 ESK 金额规则必须保持独立纯核心模块；主项目用隔离的轻量契约测试包直接包含生产源码，避免验证一条资产协议时链接整个服务器，同时生产 API 仍从原入口复用同一实现。

## 量化项目责任

1. 真实 `PaperAccessVerifier` 在固定时间接受 grant 与 `yep2`，并验证 key、grant、participant、观察时间和到期时间完全绑定。
2. API 投影视图继续只返回脱敏资产身份、精确余额、源修订和时间，不返回 token、grant ID 或 participant。
3. 前端必须对 API 视图执行运行时精确结构、资产身份、六位小数、基础单位和余额等式校验；非法响应不得进入 ESK 卡片。
4. React 实际渲染结果必须包含 fixture 的五组 ESK 数字以及 Paper、未上链、未移动资金和尚未形成量化仓位提示。

## 验收标准

- 两仓 fixture 与 V2 Schema 摘要一致。
- 主项目真实签名代码逐字节复算 fixture 的 grant 与投影。
- 主项目轻量核心套件覆盖金额解析/格式化、签名验真、V1/V2 投影与失败关闭，且受统一格式、规模、锁定依赖和离线门禁管理。
- 量化真实 verifier 接受二者，拒绝篡改、跨 grant、跨 participant 和余额关系错误。
- 前端运行时解析器拒绝未知字段、错误资产身份、错误布尔边界、金额/基础单位不一致和错误总和。
- React 服务端渲染显示 `1250/900/100/250/350` 的六位小数 ESK 值，不显示购买、保证收益或真实上链声明。
- 双仓离线 runner 与两仓风险匹配验证通过，且不访问外网、不使用生产秘密、不移动资金。

## 明确不包含

- 链上 ESK、真实发行、转让、定价、钱包或官方做市；
- 真实买回、兑付、申购、托管、收益分配或固定收益承诺；
- NET/QSHARE/NAV/订单、sandbox/testnet/live 交易；
- 公网域名、服务器安装、生产 keyring、KYC/AML 或地区准入。

## 回滚

删除新 fixture、定向测试和 runner，并恢复投影签名函数的测试缝即可。该切片不写业务数据，不需要账本迁移或用户数据回滚。
