结论：**三种模式都可以做 token 消耗统计，但可信度不同。最推荐的架构是：Rust 服务器做“统一统计中心”，APK 只负责展示；只有“用户自己 API Key 且 APK 直连 OpenAI”这种情况，才需要 APK 本地统计并同步到服务器，而且这部分要标记为“客户端上报，非强可信”。**

---

## 1. 三种模式是否支持 token 统计？

| 模式                      |   能不能统计 | 推荐统计位置        |   可信度 | 说明                                                                                                                                                                           |
| ----------------------- | ------: | ------------- | ----: | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. 用户使用你们服务器提供的 API Key |      可以 | **Rust 服务器**  |     高 | 所有请求都经过你们服务器，直接从 OpenAI 返回的 `usage` 字段记录即可。Responses API 返回里有 `input_tokens`、`cached_tokens`、`output_tokens`、`reasoning_tokens`、`total_tokens` 等用量信息。([开放AI开发者][1])          |
| 2. 用户自己输入自己的 API Key    | 可以，但分两种 | APK 或 Rust 代理 | 中 / 高 | 如果 APK 直接拿用户 key 请求 OpenAI，APK 可以从返回体读取 `usage`，但服务器无法完全验证；如果用户 key 也通过你们 Rust 服务器代理请求，则服务器统计可信。                                                                             |
| 3. 用户使用你们服务器的 Codex CLI |      可以 | **Rust 服务器**  |     高 | Codex CLI 的 `codex exec --json` 会输出 JSONL 事件，`turn.completed` 事件里包含 `usage`，例如 `input_tokens`、`cached_input_tokens`、`output_tokens`、`reasoning_output_tokens`。([开放AI开发者][2]) |

---

## 2. 不建议把“服务器 API Key”放在 APK 里

如果是“你们服务器提供的 API Key”，**不要把真正的 OpenAI API Key 下发到 APK**。OpenAI 官方也明确建议不要把 API Key 放在浏览器或移动 App 这种客户端环境，因为别人可以提取 key 后替你们发请求，造成费用和安全风险；请求应该走你们自己的后端服务器。([OpenAI Help Center][3])

所以模式 1 应该是：

```text
APK
  -> 你们 Rust 服务器
      -> OpenAI API
      <- OpenAI 返回 usage
  <- Rust 服务器返回模型结果 + 本次消耗
```

这样你们可以做到：

用户级统计、功能级统计、模型级统计、每日/月度统计、额度控制、异常用量告警、账单对账。

---

## 3. 我的推荐架构

你可以把它设计成一个独立的 **Token Usage Meter 模块**，后续所有功能都接这个模块。

整体结构建议这样：

```text
APK / Web / 其他客户端
        |
        v
Rust Server API Gateway
        |
        +--> OpenAI Responses API Adapter
        |
        +--> Codex CLI Adapter
        |
        +--> Client Report Adapter  只给“用户自己的 key 且 APK 直连”使用
        |
        v
Token Usage Meter
        |
        +--> usage_events 原始流水表
        +--> usage_daily_aggregates 每日聚合表
        +--> quota / balance 用户额度表
        +--> pricing_snapshot 价格快照表
```

核心思想是：**所有来源最后都转成统一的 usage event。**

---

## 3.1 Codex CLI 的资源来源与扣费口径

PC 项目会话里的 Codex CLI 需要同时记录 token 和资源来源，不能只看 `usage_mode=pc_agent_cli`。当前统一字段是：

| `billing_source` | 含义 | 是否扣平台额度 | 经验条展示 |
|---|---|---:|---|
| `own_codex` | 用户使用自己 PC 节点上登录的 Codex 账号 | 否 | 自用 Codex |
| `shared_codex` | 用户借用其他用户 PC 节点上登录的 Codex 账号 | 是，按平台策略结算 | 借用 Codex；节点 owner 侧显示分享给别人 |
| `platform` | 平台 API / 平台模型 / 未能归类的可信服务端调用 | 是 | 平台/其他 |
| `user_api_key` | 用户自己的 API key 经服务端代理 | 否 | 不计平台额度，仍记录 token |
| `client_reported` | 客户端参考上报 | 否 | 仅参考统计 |

硬规则：

1. 用户自己的 Codex 账号不消耗一龙平台余额；即使用户平台额度为 0，也允许继续使用自己的 Codex。
2. 自有 Codex 仍必须写入 token 用量，便于用户看到自己实际用了多少上下文和输出。
3. 借用别人 Codex / 远程节点时，consumer 侧记录 `shared_codex`，provider 侧通过节点结算流水累计“分享给别人”的 token 和收益。
4. 自用自己的节点不能生成 provider 分享流水，避免经验条把“自己用自己”误算成贡献。
5. 月度额度、余额预检和可用性判断必须排除 `own_codex`；共享或平台来源仍按平台策略检查额度。

---

## 4. 三种模式分别怎么接入

### 模式 1：用户使用你们服务器 API Key

这是最好做、最可靠的模式。

Rust 服务器每次请求 OpenAI 后，从返回 JSON 里取：

```text
usage.input_tokens
usage.input_tokens_details.cached_tokens
usage.output_tokens
usage.output_tokens_details.reasoning_tokens
usage.total_tokens
```

OpenAI Responses API 的返回结构里明确有这些字段。([开放AI开发者][1])

然后你们写入数据库：

```text
user_id = 当前登录用户
mode = server_api_key
feature_code = 当前功能，例如 chat / translate / code_review
model = gpt-xxx
input_tokens = ...
cached_input_tokens = ...
output_tokens = ...
reasoning_tokens = ...
total_tokens = ...
```

这部分可以作为你们系统的“强可信账本”。

---

### 模式 2：用户自己输入自己的 API Key

这里有两种实现路线。

第一种是 **APK 直连 OpenAI**：

```text
APK
  -> OpenAI API，使用用户自己的 key
  <- OpenAI 返回 usage
APK 本地记录 usage
APK 可选同步 usage 到你们服务器
```

优点是用户 key 不经过你们服务器，隐私和责任边界清楚。缺点是服务器无法完全验证 APK 上报的数据，用户也可能在你们 APK 之外使用同一个 key，这部分你们当然统计不到。

这种模式适合做：

```text
“本 App 内使用统计”
“仅供用户参考”
“非你们平台扣费依据”
```

第二种是 **用户 key 也走你们 Rust 服务器代理**：

```text
APK
  -> Rust 服务器，带用户自己的 key 或 key_id
      -> OpenAI API
      <- OpenAI 返回 usage
  <- Rust 服务器返回结果
```

这样统计可信度最高，但你们会接触用户的 API Key，要处理加密存储、脱敏、权限、删除、泄露风险。除非你们明确需要“统一可信统计”或“统一代理能力”，否则我不建议默认这么做。

我的建议是：

```text
默认：用户自己的 key 在 APK 本地使用，本地统计 + 可选同步。
高级模式：用户授权托管 key，走 Rust 服务器代理，服务器强统计。
```

---

### 模式 3：你们服务器的 Codex CLI

如果 Codex CLI 是你们服务器启动的，比如：

```text
APK
  -> Rust 服务器发起 Codex 任务
      -> Rust 服务器运行 codex exec --json ...
      <- 读取 JSONL usage
  <- 返回任务结果
```

这就很适合在 Rust 服务器统计。

Codex CLI 官方文档说明，`codex exec --json` 会把输出变成 JSON Lines 事件流，并且 `turn.completed` 里会包含 usage，例如：

```json
{
  "type": "turn.completed",
  "usage": {
    "input_tokens": 24763,
    "cached_input_tokens": 24448,
    "output_tokens": 122,
    "reasoning_output_tokens": 0
  }
}
```

这正好可以被你的 Rust 服务器解析并转成统一 usage event。([开放AI开发者][2])

注意：不要只靠 `/status` 文本解析。Codex CLI 的 `/status` 确实可以显示当前 token usage，但它更适合人看，不如 `codex exec --json` 稳定。官方文档里 `/status` 的用途是显示 session 配置和当前 token usage。([开放AI开发者][4])

---

## 5. Rust 服务器做统计，还是 APK 做统计？

我的建议非常明确：

```text
Rust 服务器做主统计。
APK 只做展示和少量本地辅助统计。
```

原因不是单纯“性能”，而是这几个更关键：

第一，**安全性更好**。服务器 API Key 不会泄露到 APK。

第二，**统计更可信**。服务器记录的是实际请求返回的 usage，不是客户端自己报的数字。

第三，**更容易做额度控制**。比如用户每天最多 100 万 token，服务器可以请求前预扣、请求后结算。

第四，**更容易复用**。以后你们 APK 新增翻译、总结、代码审查、图片理解、Codex 自动修复等功能，都只要调用同一个 usage 模块。

第五，**更容易对账**。OpenAI 还有组织级 Usage API，可以按时间、project、api_key、model、user 等维度查聚合用量，用来和你们自己的 usage_events 做每日对账。([开放AI开发者][5])

性能方面，Rust 服务器统计这点数据开销很小。一次模型调用可能几百毫秒到几十秒，插入一条 usage event 通常是毫秒级，不会成为瓶颈。真正要注意的是数据库写入和聚合方式，而不是 Rust 本身性能。

---

## 6. 你这个模块建议统计哪些字段？

建议不要只存 `total_tokens`，否则后面算成本、做分析会很痛苦。

最少建议存这些：

```text
id
request_id                  // 一次请求唯一 ID，防止重试重复计费
user_id
workspace_id / team_id       // 如果你们以后有团队功能
feature_code                // 哪个功能消耗的，例如 chat、translate、codex_fix
usage_mode                  // server_api_key / user_api_key_client / user_api_key_proxy / server_codex_cli
provider                    // openai
model
endpoint                    // responses / chat_completions / codex_cli / embeddings 等
input_tokens
cached_input_tokens
output_tokens
reasoning_tokens
total_tokens
tool_calls_count            // web_search、file_search 等工具调用次数可另算
estimated_cost_usd
pricing_version
openai_response_id
openai_request_id
status                      // success / error / cancelled / timeout
created_at
completed_at
```

为什么要拆这么细？

因为 OpenAI 计费不是简单看 total tokens。不同模型、输入、缓存输入、输出价格可能不同；官方价格页也是按 Input、Cached input、Output 分开列的。([开放AI开发者][6])

计算成本时应该类似这样：

```text
cost =
  non_cached_input_tokens * input_price
+ cached_input_tokens * cached_input_price
+ output_tokens * output_price
+ 其他工具调用费用
```

不要只用：

```text
total_tokens * 单价
```

这个会不准。

---

## 7. 推荐的数据流

### 请求前

Rust 服务器生成一个 `request_id`，记录请求属于哪个用户、哪个功能、哪个模式。

可选：如果你们要做额度控制，可以先调用 token count 接口估算输入 tokens。OpenAI 有专门的 input token count API，可以在真正请求前计算输入 token，用于预估成本和避免超上下文。([开放AI开发者][7])

```text
1. 生成 request_id
2. 估算 input_tokens
3. 检查用户额度
4. 预冻结额度
5. 发起模型请求
```

### 请求后

```text
1. 解析 OpenAI / Codex 返回的 usage
2. 写入 usage_events
3. 更新用户余额 / quota
4. 更新每日聚合表
5. 返回给 APK：本次消耗 + 当前剩余额度
```

### 定时对账

每天或每小时跑一次：

```text
你们数据库 usage_events 汇总
        vs
OpenAI Organization Usage API 汇总
```

OpenAI 的组织用量接口可以返回聚合的 input_tokens、output_tokens、input_cached_tokens、num_model_requests 等信息，也支持用 start_time、api_key_ids、project_ids 等参数过滤。([开放AI开发者][5])

对账的作用是发现：

```text
服务器漏记
重试重复记账
某个 key 被异常使用
某个功能消耗异常高
```

---

## 8. 对“用户三种模式同时开启”的处理方式

不要把统计逻辑写死成“一个用户只有一种 key 模式”。应该每次请求都带上 `usage_mode`。

例如同一个用户今天可能有三条记录：

```text
user_id = 1001, feature = chat,       mode = server_api_key,      total = 3000
user_id = 1001, feature = translate,  mode = user_api_key_client, total = 8000
user_id = 1001, feature = codex_fix,  mode = server_codex_cli,    total = 120000
```

展示时可以分两层：

```text
总消耗：
  今天共 131000 tokens

按模式：
  服务器 API Key：3000 tokens
  用户自己的 API Key：8000 tokens
  服务器 Codex CLI：120000 tokens

按功能：
  聊天：3000 tokens
  翻译：8000 tokens
  Codex 修复：120000 tokens
```

这样后续任何新功能都能接入。

---

## 9. 哪些统计是“可信”的，哪些只能参考？

建议你在数据库里加一个字段：

```text
trust_level
```

可以是：

```text
verified_server
client_reported
reconciled
estimated
```

对应关系：

```text
服务器 API Key 模式：
  verified_server

用户自己的 key，APK 直连：
  client_reported

用户自己的 key，Rust 代理：
  verified_server

服务器 Codex CLI：
  verified_server

请求前预估：
  estimated

OpenAI Usage API 对账后的修正：
  reconciled
```

这个设计很重要。以后如果你要根据 token 扣费，**只能用 verified_server / reconciled**。如果只是给用户看“你自己 key 在本 App 里的大概消耗”，可以展示 client_reported。

---

## 10. 最终推荐方案

你现在最适合这样做：

```text
第一阶段：
  1. Rust 服务器实现 Token Usage Meter 核心模块
  2. 接入服务器 API Key 模式
  3. 接入服务器 Codex CLI 模式
  4. APK 做用量展示页面

第二阶段：
  5. APK 对“用户自己的 API Key 直连模式”做本地 usage 统计
  6. 同步到服务器，但标记为 client_reported
  7. 如果后续你们需要强可信统计，再增加“用户 key 走服务器代理”模式

第三阶段：
  8. 加 quota、余额、每日限制、异常告警
  9. 加 OpenAI Usage API 对账
  10. 加功能级成本报表
```

一句话总结：

**服务器提供 API Key 和服务器 Codex CLI：一定放 Rust 服务器统计。用户自己的 API Key：APK 直连时只能本地统计；想要强可信，就必须也走 Rust 服务器代理。你的统一 token 统计模块应该放在 Rust 服务器，APK 只负责展示和客户端直连模式的辅助上报。**

[1]: https://developers.openai.com/api/reference/resources/responses/methods/create "Create a model response | OpenAI API Reference"
[2]: https://developers.openai.com/codex/noninteractive "Non-interactive mode – Codex | OpenAI Developers"
[3]: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety?utm_source=chatgpt.com "Best Practices for API Key Safety"
[4]: https://developers.openai.com/codex/cli/slash-commands "Slash commands in Codex CLI | OpenAI Developers"
[5]: https://developers.openai.com/api/reference/resources/admin/subresources/organization/subresources/usage/methods/completions "Completions | OpenAI API Reference"
[6]: https://developers.openai.com/api/docs/pricing "Pricing | OpenAI API"
[7]: https://developers.openai.com/api/docs/guides/token-counting "Counting tokens | OpenAI API"
