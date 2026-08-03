# 一龙开放商业决策登记

本文集中记录跨文档的状态，不复制详细论证。新讨论先进入讨论图或草稿，形成明确取舍后再更新本表及对应 ADR。

## 已接受

| 决定 | 含义 | 依据 |
|---|---|---|
| 先从商户 AI 经营与应用开发切入 | 先解决商户愿意付费的真实问题，再形成协议网络 | `docs/decisions/open-commerce-network-principles.md` |
| 商户数据默认归商户控制 | 平台提供存储、授权、调用和审计能力，不把数据控制权作为流量壁垒 | `docs/decisions/open-commerce-network-principles.md` |
| V1 先做商户节点和能力调用主干 | 商户、能力、授权、调用、计量、审计是最小闭环 | `docs/decisions/open-commerce-network-v1-architecture.md` |
| HTTP 与 MCP 共用同一领域服务 | 避免网页、AI 和第三方应用形成不同业务规则 | `docs/open-commerce-network-v1-api.md` |
| 能力声明必须在调用时强制执行 | 发布时只接受平台真正支持的有限 Schema；无效输入不创建调用，无效输出零金额失败并释放预算，审计不记录业务值 | `docs/decisions/open-commerce-capability-contract-enforcement-v1.md` |
| 消费者应持有本人可复核的调用凭证 | 从现有 Invocation 派生账户级终态只读投影；列表隐藏结果，详情仅本人读取，摘要绑定规范负载且不冒充支付或链上证明 | `docs/decisions/open-commerce-consumer-invocation-receipts-v1.md` |
| 消费者 PC 应按能力契约填写调用输入 | 从商户发布的有限 Schema 生成非技术表单；未声明默认值的可选字段默认省略，无法呈现时失败关闭，动作能力对当前输入再次确认且不提供原始 JSON 绕过 | `docs/decisions/open-commerce-schema-driven-invocation-form-v1.md` |
| 动作能力确认必须由服务端绑定并一次性消费 | `action` 使用 5 分钟两阶段确认，绑定用户、App、商户、能力、Grant、幂等键和输入摘要；调用创建与确认消费原子完成，所有 HTTP、MCP 和 PC 入口共用规则 | `docs/decisions/open-commerce-server-action-confirmation-v1.md` |
| 真实商户 ERP 通过受控运行时接入 | 运行地址和密钥与能力契约分离，经过 HTTPS 白名单、HMAC 和 Manifest 健康验证后才可调用 | `docs/decisions/open-commerce-merchant-runtime-v1.md` |
| 当前调用只计量、不做真实收费 | 先验证行为、权限和审计，再验证经济层 | `docs/decisions/open-commerce-network-v1-architecture.md` |
| 先建数据接入控制面，再逐个平台实现适配器 | 统一记录来源、范围、健康度和同步证据，不用“已接入”掩盖真实权限差异 | `docs/decisions/open-commerce-integration-control-plane.md` |
| 先用项目内消费者与开发者沙盒验证开放网络 | 采用非付费透明排序、显式授权和一次性测试凭据；不把沙盒宣传为公共网络 | `docs/decisions/open-commerce-consumer-developer-sandbox-v1.md` |
| 商户主动发布脱敏跨项目目录 | 商户默认私有；只有显式发布后才返回专用脱敏契约，撤回后阻断外部调用，App 身份必须绑定所有者 | `docs/decisions/open-commerce-directory-publication-v1.md` |
| 沙盒 App 与授权申请必须有完整生命周期 | 停用永久废弃旧 Token 并取消待处理申请；重新启用生成新 Token；申请方和商户共享同一申请状态 | `docs/decisions/open-commerce-developer-lifecycle-v1.md` |
| 商户必须能终止失信 App 的访问 | 商户可手动封禁具体 App，并原子撤销 Grant、取消待审批申请；解除不恢复旧信任 | `docs/decisions/open-commerce-app-blocks-v1.md` |
| 第三方 App 授权应默认有期限 | PC 新授权默认 30 天，长期授权需显式选择；到期不改历史、不自动续期，发现和调用失败关闭 | `docs/decisions/open-commerce-grant-expiration-v1.md` |
| 消费者关系必须由消费者持有和撤销 | 关系只包含匿名标识、固定范围和最长 366 天期限；商户看不到消费者账号或项目，重新建立关系会更换匿名标识 | `docs/decisions/open-commerce-consumer-relationships-v1.md` |
| 消费者关系续期必须轮换匿名身份并保持重试幂等 | 续期撤销旧关系、继承原范围和用途并生成新别名；同一来源只产生一个后继，内部续期链不向商户公开 | `docs/decisions/open-commerce-consumer-relationship-renewal-v1.md` |
| 删除请求必须停止关系且不能伪装外部履约 | 消费者发起请求时原子撤销关系；商户可处理匿名工单，但完成只表示商户声明，不是平台验证的删除证明 | `docs/decisions/open-commerce-consumer-data-erasure-requests-v1.md` |
| 消费者关系与删除请求应支持本人可验证导出 | 同一幂等键保存不可变快照，每次读取复核 SHA-256；V1 只导出关系、续期链和请求回执，不冒充完整数据迁移 | `docs/decisions/open-commerce-consumer-portability-exports-v1.md` |
| 消费者偏好必须由本人保存并按关系选择字段披露 | 档案默认私有；商户只读取有效 `preference.remember` 关系上的匿名快照，关系失效后失败关闭，档案更新不自动扩张披露 | `docs/decisions/open-commerce-consumer-preference-disclosures-v1.md` |
| 消费者低敏偏好应进入本人可验证数据包 | V2 在同一读事务中加入当前偏好档案和历史披露，旧 V1 包保持原字节摘要；仍不导出订单、联系方式、支付或账号身份 | `docs/decisions/open-commerce-consumer-portability-exports-v2.md` |
| 消费者本人调用凭证应进入可携带数据包 | V3 在同一读事务中加入账户级终态调用，每条凭证和总包双层复核；保留 V1/V2 原摘要，不导出原始输入，也不冒充完整订单或支付证明 | `docs/decisions/open-commerce-consumer-portability-exports-v3.md` |
| 每个 Grant 可限制完整授权期风险 | 商户可选设置总调用次数和总计量金额；调用前原子预留、失败退回，预算用尽后重新授权 | `docs/decisions/open-commerce-grant-budgets-v1.md` |
| 孤儿商业调用必须失败关闭并释放 Grant 预留 | 启动时关闭遗留调用，运行期回收超过 120 秒的调用；失败、预算退回、预留释放和脱敏审计保持同一事务 | `docs/decisions/open-commerce-invocation-recovery-v1.md` |
| App 异常先形成商户可解释证据 | 按商户和外部 App 派生近 24 小时失败、限流、Grant 预算拒绝和中断恢复计数；只提醒人工处置，不自动评分、封禁或赔付 | `docs/decisions/open-commerce-app-activity-health-v1.md` |
| 开发者 App 必须能可靠续读自己的调用结果 | Invocation 进入终态时原子追加稳定序号；测试 Token 只读取本 App 摘要和详情，游标跨 App 失败关闭；当前不冒充 Webhook | `docs/decisions/open-commerce-developer-terminal-events-v1.md` |
| 商户业务结果先形成证据层再进入 ERP/CRM | 从终态 Invocation 派生项目内证据、结果摘要、可选商户标准回执和 ERP 实例关联；不复制订单库，不把调用成功冒充为支付或履约 | `docs/decisions/open-commerce-merchant-business-evidence-v1.md` |
| ERP/CRM 衔接状态必须来自证据和显式回执 | 人工回执绑定结果摘要、接入器和明确确认；待办与需重试状态由最新回执派生，不建立第二套可漂移状态 | `docs/decisions/open-commerce-business-handoff-receipts-v1.md`、`docs/decisions/open-commerce-business-handoff-queue-v1.md` |
| 接入器机器身份必须最小授权且可失效 | 每个接入器使用只写衔接回执的一次性 Token；服务端只保存摘要，轮换、撤销和接入停用失败关闭，机器回执固化凭据版本 | `docs/decisions/open-commerce-adapter-machine-credentials-v1.md` |
| 接入器机器凭据必须限时有效 | 签发时明确选择 1–366 天，服务端按数据库时间拒绝过期 Token；历史不改写且不自动续期 | `docs/decisions/open-commerce-adapter-credential-expiration-v1.md` |
| AI 资源 V1 先做控制面和路由预演 | 统一盘点现有真实资源并保存项目策略，但不复制执行器、不泄露凭据、不假装已核验外部额度 | `docs/decisions/ai-resource-control-v1.md` |
| 节点模型共享必须由所有者显式开放 | 在线和模型上报不代表同意出租；按模型、并发和每日 Token 预算授权，调度前原子预留，所有者自用不受影响 | `docs/decisions/node-compute-sharing-supply-v1.md` |
| 共享节点每日 Token 预算必须覆盖在途任务 | 派发前原子检查今日实耗、活动预留和本次保守预留；终态按实际用量记账，过期租约不能复活 | `docs/decisions/node-compute-sharing-token-reservation-v1.md` |
| 共享节点运行偏差只先形成所有者健康证据 | 从持久化记录派生失败、Token 预留超出和过期租约，仅在所有者控制面告警，不自动处罚、赔付或上链 | `docs/decisions/node-compute-sharing-runtime-health-v1.md` |
| 过期节点推理必须失败关闭并回收预授权 | 收到终态后先冻结实际用量为非执行状态；只有未收到可信终态且租约过期的运行才原子失败关闭，迟到结果不得改写终态 | `docs/decisions/node-compute-sharing-expired-run-reconciliation-v1.md` |
| 先实现链外影子结算，再评估 Sui 网络适配器 | 复用真实成本和人工验收事实，保持双分录、幂等和默认关闭，不移动真实资金 | `docs/decisions/task-shadow-settlement-v1.md` |
| Sui 适配器先消费可复核的链下投影包 | 投影包绑定目标网络、来源摘要与投影摘要；当前只持久化和复核，固定未提交 | `docs/decisions/sui-offchain-projection-packages-v1.md` |
| 争议只追加证据并阻断投影，不改写历史账本 | 待审核或已接受争议阻断原凭证投影；纠正必须使用新的 Matter 和凭证 | `docs/decisions/task-shadow-settlement-disputes-v1.md` |
| 影子结算纠正采用独立 Matter 和追加式双腿过账 | 人工验收后在同一事务内追加冲销与替换凭证，原凭证不改写；单张纠正凭证不进入普通 Sui 投影 | `docs/decisions/task-shadow-settlement-corrections-v1.md` |
| Sui 纠正投影必须把冲销与替换绑定为一个链下原子包 | 包绑定两条腿、来源摘要和目标网络；可复核但固定未提交，替换凭证的新争议会阻断就绪 | `docs/decisions/sui-correction-projection-packages-v1.md` |
| 纠正后的经营读取必须解析当前有效凭证 | 从任意凭证回溯根并沿已过账纠正前进；待验收计划不改金额，循环或分叉失败关闭 | `docs/decisions/task-shadow-settlement-lineage-v1.md` |
| 区块链只作为多方可信、授权与结算候选层 | 链上只承载跨主体所有权、授权、关键回执和结算；订单、日志、媒体、实时数据库、AI 推理、复杂查询和匿名化继续留在链外 | `docs/open-commerce/integration-architecture.md` |
| 文档按状态与职责模块化 | 事实、决策、草稿和讨论来源分开，避免巨型愿景文档误导 AI | `docs/open-commerce/README.md` |

## 当前实施重点

1. 维持 AI 应用开发、Matter/Assignment、Git 执行和发布主链稳定。
2. 以 `cofficethinking` 参考节点验证真实商品、服务端报价、用户确认下单、订单查询、调用审计与失败降级，保持公共网络边界清晰。
3. 在已实现的节点模型供给控制和流租约上继续验证成本、异常赔付、隐私与运行告警，再扩展异构任务市场。
4. 基于数据接入控制面逐个平台实现、审核并验收真实行业适配器。
5. 为商户 ERP、营销内容、小游戏和经营分析建立可验证的行业模板。
6. 基于已实现的链外影子回执、争议案件、纠正 Matter 和 Sui 原子纠正包验证节点调用、商业调用、人工验收与净额复核，再评估真实网络适配器的租约、密钥和最终性边界。
7. 用项目文档 MCP 保持实现状态、决策和讨论来源一致。

## 提案与试验

| 提案 | 当前状态 | 进入下一阶段的条件 |
|---|---|---|
| 消费者 AI 生产公共网络 | 主动发布的跨项目目录和授权沙盒已实现；生产网络仍是提案 | 商户节点数量、生产身份互认、限流和滥用防护经过试点 |
| 闲置设备算力市场 | 模型推理供给控制与流租约已实现；完整市场仍是技术与经济提案 | 异构任务调度、可信回执、失败赔付、运行告警和单位成本优于替代方案 |
| Sui 测试网结算适配器 | 设计提案 | 链外回执和投影包通过试点，并完成 Move 对象、适配器租约、密钥、Gas、重放和争议安全评审 |
| NET 网络资产 | 经济提案 | 网络已有真实使用和可治理参数，且不承担服务计价或合同收益分配 |
| CREDIT 服务计量单位 | 经济提案 | 形成稳定的服务目录、报价和退款规则 |
| RevenuePosition 合同收入权益 | 融资提案 | 每份收入来源可独立核验、封顶、到期并披露风险 |
| 公司融资与链上治理桥接 | 长期提案 | 公司权利、协议治理和具体收入权益已完成结构分层 |

## 明确不采用

- 重新建设一个把所有消费者和商户锁在同一 App 的中心化平台。
- 把全部商户和消费者数据默认公开给所有参与者。
- 用一个代币同时代表公司股权、网络治理、服务额度、算力奖励和合同收入权。
- 在没有真实回执、对账和收入来源时，先用代币价格替代产品验证。
- 把 Sui、AWS、Filecoin 或任一基础设施绑定成不可替换的业务核心。
- 宣称分散设备已经具备企业级 GPU 集群的实时同步训练能力。

## 未决问题

1. 第一个可规模化行业应选择餐饮、便利零售还是已有付费客户所在行业。
2. 外部平台数据连接应优先采用官方 API、商户授权导出，还是本地自动化适配器。
3. 免费发现、按调用收费和按成交收费的边界如何设置。
4. 共享节点首批适合运行哪些可切分、可验证、低数据敏感度的任务。
5. 首个 Sui 试点应只做节点任务结算，还是同时验证 RevenuePosition。
6. 当前长期合同收入中，可用于产品运营、节点奖励和融资分配的比例如何隔离。

## 决策晋升流程

```text
讨论来源
  -> 讨论知识图中的候选结论
  -> 草稿与可验证假设
  -> 原型、成本或用户证据
  -> 正式 ADR
  -> 能力基线和实现引用更新
```

任何提案只有在正式 ADR 接受并出现实现证据后，才能从“提案”升级为“已实现”。
