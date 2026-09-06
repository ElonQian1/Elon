---
version_status: current
reviewed_at: 2026-09-07
evidence_status: static_candidate
---

# 币安 U 本位合约网格源码线索（2026-09-07）

真实 Win 浏览器研究 MCP 已返回币安页面脚本的搜索命中与资源分片。本报告整理其接口线索，**全部为 `static_candidate`**：尚无本报告对应的认证后业务请求/响应证据，没有执行创建、关闭或修改网格。接口字符串和封装函数存在，不等于当前页面实际调用、参数合法或业务成功。

## 来源与定位

本轮直接核对的本机回执为 `.ai-tmp/binance-grid-source-hits.json` 和 `.ai-tmp/binance-grid-page-definitions.json`；后者只读取资源的一个分片（`offset=2700`、`next_offset=8200`、`complete=false`）。这些临时文件不是完整脚本正文的跨机器存档。

| 标记 | MCP 资源身份与来源 |
|---|---|
| P | [页面脚本 page-ee55.21718cb8.js](https://bin.bnbstatic.com/static/chunks/page-ee55.21718cb8.js)；resource ID `bd22529c388df481c26a74a5de0196a424817e185f5e5cf88fba26cf84735ad8` |
| P 元数据 | SHA-256 `de6da80226b765083ad5bfa49ed741e24ec4148587673d88bf14412c786cc446`；1,884,546 bytes；generation 3；`redacted=false`、`truncated=false` |
| C | [脚本 4818.ab7ba930.js](https://bin.bnbstatic.com/static/chunks/4818.ab7ba930.js)；resource ID `e90566f83d5dee76ccfb96cf463a75d016df998ecc85b6e080b7c594aa209159` |
| C 元数据 | 另一份真实 `read_resource` 回执：SHA-256 `24765919f67706cdb4e894bb491a4904e9000fa64efa5814ee808fd40fc0dcf2`；161,121 bytes；generation 3；`redacted=false`；读取 `offset=103700`、limit 1,300 |

以下 `offset` 是回执记录的命中或读取位置，用于返回该资源查阅附近代码，不宣称是整个函数的起点。资源哈希和字节位置对应凭据处理后的本机 UTF-8 文本；CDP initiator 的原始脚本行列不能直接与其等同。本轮两份回执没有建立坐标换算。

## 静态候选表

方法栏只记录片段明确出现的具名依赖 `get` / `post` 调用；只有裸函数名、未核实绑定的地方保留“待读”。即使已填 GET/POST，也尚未得到网络层方法证据。表内路径均为源码中的相对路径，不能把脚本 CDN origin 当成 API origin。

| 用途线索 | 路径 | 方法候选 | 定位与片段能证明的范围 |
|---|---|---|---|
| 运行中列表 v1 | `/bapi/futures/v1/private/future/grid/query-open-grids` | GET | C 103187 见依赖 `a.get` 与查询串序列化；P 3642 另有裸 `get` 和 `symbol`，不能由后者认定当前调用链 |
| 运行中列表 v2 | `/bapi/futures/v2/private/future/grid/query-open-grids` | POST | P 3857、C 103654；传入对象，字段合同待读。与 v1 GET 分别记录 |
| 历史列表 | `/bapi/futures/v1/private/future/grid/query-grid-history` | 待读（仅见裸 `post`） | C 102920；函数绑定、参数及实际使用位置尚未核实 |
| 网格详情 | `/bapi/futures/v1/private/future/grid/query-grid-detail` | GET | P 5876；对象经过查询串序列化，当前片段未列全字段 |
| 匹配/成交汇总 | `/bapi/futures/v1/private/future/grid/query-matched-info` | GET | P 4054；查询串明确出现 `strategyId`，不推导返回结构 |
| 匹配/成交明细 | `/bapi/futures/v1/private/future/grid/query-grid-matched-items` | POST | P 4283；传入对象，分页与返回字段待读 |
| 运行中订单条目 | `/bapi/futures/v2/private/future/grid/query-grid-open-items` | GET | P 285460；查询串明确出现 `strategyId`，不能当作已成交记录 |
| 创建 v2 | `/bapi/futures/v2/private/future/grid/place-grid` | POST | C 67630、104373；补充读取 103700 附近确认。只得到封装和前端分支线索 |
| 创建 v1（另一版本） | `/bapi/futures/v1/private/future/grid/place-grid` | POST | C 105375；保留版本差异，未确认是否仍由当前流程调用 |
| 关闭网格 | `/bapi/futures/v1/private/future/grid/close-grid` | 待读（仅见裸 `post`） | P 3166；未核实绑定、是否平仓的参数或服务端语义 |
| 修改网格 | `/bapi/futures/v1/private/future/grid/update-grid` | POST | P 6439 的模块 65268 明确调用依赖 `e.post`；P 4484 另一个裸 `post` 封装不作为有效调用链证明 |
| 修改价格区间 | `/bapi/futures/v1/private/future/grid/update-grid-range` | POST | P 搜索回执 391849；主任务补充命中 391861 明确见依赖 `Ae.post`。输入对象字段和范围约束待读 |
| 修改投入 | `/bapi/futures/v1/private/future/grid/update-grid-investment` | 待读（仅见裸 `post`） | P 5283；只记录名称线索，不推导资金划转或保证金行为 |

## 已知限制与后续取证点

- 页面模块 56435 的导出清单只列出 `r`、`a`、`i`。同一片段里的 v1 列表及其他裸 `post` / `get` 封装存在未使用 wrapper 的嫌疑；当前没有完整绑定分析，既不能断言它们失效，也不能将其标成 `active_path`。
- C 的 v2 创建封装前可见按 `symbol` 检查运行中 TWAP、在相应分支返回 `symbolInTwap` 的前端逻辑。这不是服务端互斥规则或创建成功的证据；检查接口、完整输入、校验条件和错误合同仍待读取。
- `update-grid` 与 `update-grid-range` 是两个不同路径；不能合并成已验证的“修改接口”，也不能推导可修改的字段、时机或对运行中仓位的影响。
- 下一步先补函数绑定、导出与调用点以及参数构造；在用户已授权的只读官网动作下，另行获取列表/详情请求及业务响应证据。创建、关闭、修改继续保持未执行，不能通过重放这些候选路径补证。

本报告仅包含源码定位和候选合同，不记录账户标识、持仓值或授权凭据，也不更新需求或功能验收状态。
