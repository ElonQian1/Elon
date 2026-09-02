---
status: accepted
owner: platform-assets, quant-integration
reviewed_at: 2026-09-02
---

# ESK Paper 双仓签名互操作验收 V4

## 目标

主项目与独立 `yilong-quant` 仓库必须针对同一份固定、公开、仅供测试的签名向量完成字节级互认，消除“两边各自测试通过，但序列化字段顺序、签名域或摘要算法已经漂移”的风险。

本需求只增强离线 Paper 验收证据，不新增生产 API、用户入口、代币发行、资金托管、量化交易、收益计算或公网部署。

## 共享向量

两仓各保存一份同路径、同字节的 `contracts/quant/esk-paper-cross-repo-interoperability-v1.fixture.json`，包含：

- 固定测试时间、固定业务标识和明确的 `test_only` 标记；
- 主项目测试公钥、`ypg1` Paper grant 和 `yeqa1` 单申请授权；
- 量化项目测试公钥、accepted/released 两个 `yqar1` 回执及其 SHA-256 摘要；
- `simulated=true`、`funds_moved=false`、`quant_units_issued=false`、`nav_participation=false`、`trading_started=false` 等安全断言。

共享文件不得包含生产私钥、测试私钥种子、真实用户标识、付款数据、交易所凭据或 bearer 会话。固定私钥种子只能存在于测试源码中，并必须用明显的测试名称与注释标识。

## 主项目验收责任

1. 使用主项目真实 Ed25519 签名代码和结构体序列化固定 grant 与 allocation authorization，并与共享向量逐字节相等。
2. 使用主项目真实量化回执 keyring/verifier 验证 accepted/released 向量，校验事件、revision、前序摘要、金额、申请、参与者和授权关联。
3. 篡改任一 token 后必须验签失败；把测试 key 标记为 revoked 后必须拒绝。
4. 提供一个显式接收量化仓库路径的离线脚本，比较共享 fixture 以及既有 authorization/receipt schema 的 SHA-256，并运行两仓定向验收。

## 量化项目验收责任

1. 使用真实 `PaperAccessVerifier` 在固定时间验证主项目 grant 与 allocation authorization，确保 key、grant、participant、request、金额、风险版本和 TTL 精确绑定。
2. 使用真实 `EskAllocationBindingStore` 与 `EskAllocationReceiptSigner` 接收固定授权、幂等重放、生成 accepted 回执、释放并生成 released 回执。
3. 两个量化回执 token、摘要、binding ID、receipt ID 和前序摘要必须与共享向量逐字节相等。
4. token 篡改、跨 grant/participant 漂移或 fixture 安全边界变为真实资金/交易时必须失败关闭。

## 跨仓 runner 合同

主项目脚本只允许读取两个已存在的本地 Git 工作区，不 clone、fetch、push、启动服务或访问外网。它必须：

- 验证两个路径不是同一目录，且关键共享文件都存在；
- 对共享 fixture 做原始字节 SHA-256 比较；既有 authorization/receipt schema 只规范 CRLF/LF 后比较 UTF-8 字节，不能让 Windows checkout 换行差异形成假漂移；
- 调用每个仓库自己的共享 Rust 缓存入口运行对应定向测试；
- 输出不含 token 正文的结构化 Paper 验收回执；
- 任一步失败即返回非零，不以其中一仓的成功替代双仓成功。

## 验收标准

- 两仓 fixture 原始字节哈希一致，两份既有 schema 的换行规范化字节哈希一致。
- 主项目生成的 `ypg1`、`yeqa1` 与 fixture 完全一致。
- 量化项目生成的 accepted/released `yqar1` 与 fixture 完全一致。
- 主项目能验签量化回执，量化项目能验签主项目授权。
- 篡改与 revoked key 回归失败关闭。
- 量化 `scripts/accept-paper-e2e.ps1` 纳入互操作测试并输出 `status=passed`。
- 两仓常规验证、源码体积与文档门禁通过。

## 明确不包含

- 真实 ESK 链上发行、转账、销毁或做市；
- 真实用户付款导入、申购、赎回、卖回结算或托管；
- sandbox/testnet/live 交易、交易所密钥或订单；
- 基金份额、NAV、收益分配、固定或预计年化收益；
- KYC、适当性、法域准入或生产密钥轮换的替代品；
- 量化公网域名、服务器部署或项目广场“已上线”状态。

## 回滚

删除新增 fixture、测试与 runner，并恢复验收入口和状态文档即可。不得修改既有 V3 运行时协议或历史数据库来完成回滚。
