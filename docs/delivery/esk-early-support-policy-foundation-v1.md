---
version_status: current
reviewed_at: 2026-09-06
feature_id: esk-early-support-policy-foundation-v1
owner: esk-primary
---

# ESK 两年保障政策草案合同 V1 交付证据

## 本批次范围

对应[工程需求](../requirements/esk-early-support-policy-foundation-v1.md)及
[责任边界](../esk-delivery-ownership.md)。当前主任务直接拥有新政策、后续账本、
统一接口和发行整合工作；旧任务只收尾已开工的发行离线预检。

本批次是本地、无网络的草案检查工具，不是资金产品上线。
基线为 `aebbfc41b910887725179bca46ceb2b0d793458f`。

## 交付矩阵

| 能力 | 实现 | 验证 | 交付 | 实际业务验收 |
|---|---|---|---|---|
| 责任边界与工程需求 | 已编写 | 独立文档审阅通过 | 随本批次提交 | 用户已明确当前任务主导 |
| 版本化输入合同与全空决策 fixture | 已实现 | 独立审查及专项测试通过 | 随本批次提交 | 仅草案 |
| 严格离线检查和 CLI | 已实现 | 34/34 测试通过，零失败、零跳过 | 随本批次提交 | 未配置真实经济条款 |
| 生产保障、发行、投资、兑付 | 未实现 | 未执行 | 未部署 | 未验收 |

## 使用与结果解释

入口为 `scripts/esk-early-support-policy/cli.js`，读取明确提供的本地 JSON 文件。
默认 fixture 仅保留 ESK、募集投资、回报、早期保障与两年政策的已确认意向，
所有待决定参数为 `null`。从仓库根目录运行：

```text
node scripts/esk-early-support-policy/cli.js --input contracts/esk/early-support-policy-draft-v1.fixture.json
```

实际 fixture 结果为 `needs_decisions`，列出 14 个未决定字段；最低收益条款仅在选择
本金与最低收益保障后成为必填。规范内容摘要为
`171d1383f07606af37f152feb1705c21e71a2d50d4cf6c8bd004228f40a50041`。

文件必须为普通本地文件，最多 65,536 字节。Schema 定义字段及枚举；检查器另外执行
严格 JSON、真实日期、Unicode 标量和跨字段检查，因此只用通用 Schema 验证器不等同本工具的完整检查。
合法但未完成的草案退出码为 0，结果仍明确待决定；坏输入退出码为 2，只返回固定错误码。

读取成功不代表保障生效：输出中的政策状态始终为 `draft`，资金证明和生产授权始终为 false。
即使所有必填字段完整，也只能进入政策审阅，不能因此声称已获经济、签名或资金操作授权。
输入文件只应包含非秘密的政策描述；输出只给摘要、状态和固定字段/问题标识，不回显自由文本。

## 尚待完成的业务工作

- 保障范围、计价币种、两年起止及周年规则、责任主体、资金来源与兑现条件。
- 转让、消费、二级买入、已领回报及兑付是否交回代币的权利规则。
- 购币记录、投资账本、保障义务、准备金和分配账本的真实实现与对账。
- 实际资金证明、真实钱包验收、TLS 客户端接入、正式发行和真实资金操作。

这些事项不会由 Schema 完整性检查或现有测试网发行预检自动批准。

## 验证记录

- 2026-09-06，独立测试代理执行完整测试集，主任务复核实际结果文件及 TAP 摘要：
  34 个测试全部通过，失败、取消、跳过均为 0，耗时约 1.7 秒。
- 测试包括全空/完整/矛盾草案、字段与枚举、重复和危险键、非法 UTF-8、BOM、
  深度和大小上限、非法/倒序日期、文本限制、稳定摘要，以及真实 CLI 子进程及错误脱敏。
- 独立代码审查发现 Unicode 行分隔符后的尾随空白和 C1 控制字符可绕过旧文本模式，
  实现已修复，新增回归在本次测试中通过；未声称运行过修复前的失败测试。
- 两份实现脚本的 Node 语法检查通过；第二位独立审查代理复核合同、默认值、
  输入边界和报告权限，未发现需要修改的实质问题。
- 本地运行版本为 Node `v22.14.0`。源码体积门禁覆盖本批次 7 份脚本并通过，
  文档模块化门禁覆盖 3 份文档并通过且无警告；暂存差异格式检查通过。
- 运行证据由项目日志器保存，日志标识为
  `esk-early-support-policy-tests-20260906-105045-234`，对应 result 为 `passed`、exit code 0。

可重复的项目验证入口：

```powershell
powershell -WindowStyle Hidden -NoProfile -ExecutionPolicy Bypass -File scripts/invoke-ai-logged-command.ps1 -LogName esk-early-support-policy-tests -CommandLine "node --test scripts/esk-early-support-policy/tests/*.test.js" -WorkingDirectory . -TimeoutSeconds 180 -StallTimeoutSeconds 90 -RequireOutput
```

本批次没有服务器、APK、交易所或链上部署；也没有验证任何实际资金、责任主体或收益条款。
旧发行预检、Move 合约、资产入口和历史经济决定均不在本批次修改范围。
