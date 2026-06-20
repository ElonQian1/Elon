是的，**真的能省 token**，但要分清楚两种“省”：

```text
1. 真的少发 token：
   不把整个项目代码发给模型，只发 repo map + 相关文件片段。

2. token 数可能没少很多，但费用/延迟下降：
   静态前缀被 API prompt cache 命中，cached input tokens 更便宜、更快。
```

最关键的是：**repo map、符号索引、rg 搜索不是为了让模型“知道一切”，而是为了让模型“少读无关内容”。**

---

## 1. 做了这些以后，API 调用真的能省 token 吗？

**能，但不是自动省，取决于你怎么用。**

假设你的项目有：

```text
总代码：300,000 行
粗略 token：2,000,000+ tokens
```

如果你每次让 AI 改功能，都把大量代码塞进去，那肯定非常贵，而且很多模型也放不下。

更合理的做法是：

```text
每次请求只发：
  - 项目规则：1,000 tokens
  - repo map 相关部分：1,000 ~ 5,000 tokens
  - rg / 符号索引搜索结果：500 ~ 3,000 tokens
  - 真正相关文件：5,000 ~ 30,000 tokens
```

这样就从：

```text
几十万 / 几百万 tokens
```

降到：

```text
几千 / 几万 tokens
```

这就是实实在在的节省。

Aider 的 repo map 就是这个思路：它会把整个仓库压缩成一个简洁地图，包含文件列表、关键符号、类/函数签名，并且只把最相关、能放进 token budget 的部分发给 LLM。Aider 文档里还提到 repo map 默认受 `--map-tokens` 控制，默认大约是 1k tokens。([Aider][1])

---

## 2. 但 repo map 不是越大越好

很多人会误解：

```text
我把完整 repo map 每次都发给 AI，AI 就会更懂项目。
```

这不一定对。

如果 repo map 很大，比如：

```text
AI_PROJECT.md      3,000 tokens
AI_RULES.md        2,000 tokens
repo_map.md       30,000 tokens
symbols summary   20,000 tokens
```

那你每次请求一开始就 50,000+ tokens，反而可能浪费。

正确方式是：

```text
先发小地图
  ↓
模型判断可能相关的模块
  ↓
再用 rg / 符号索引查更具体的位置
  ↓
只读取真正相关文件
```

所以 repo map 最好有两个版本：

```text
target/ai/repo_map_compact.md    # 每次都可以发，控制在 1k~5k tokens
target/ai/repo_map_full.json     # 不直接发给 AI，给工具查询
```

---

## 3. 符号索引本身通常不直接发给 AI

符号索引最大的价值不是“把索引全文发给模型”。

而是让 AI 能查询：

```text
SessionManager::refresh_token 定义在哪里？
TokenStore 这个 trait 有哪些实现？
AuthError::TokenExpired 在哪里被引用？
这个函数有哪些调用方？
```

然后工具只返回很小的结果：

```text
SessionManager::refresh_token
  defined at crates/server/src/auth/session.rs:88
  referenced by:
    crates/server/src/routes/auth.rs:122
    crates/server/tests/auth_refresh.rs:45
```

这个结果可能只有几百 tokens。

如果没有符号索引，AI 可能要读很多文件才能猜到这些关系。

所以：

```text
符号索引省 token 的方式：
  不是“压缩代码”
  而是“减少 AI 乱读文件”
```

---

## 4. rg 搜索也能省 token

`rg` 的作用很直接。

比如用户说：

```text
帮我修改 refresh token 过期逻辑。
```

不好的做法：

```text
把 auth 目录全发给模型。
```

好的做法：

```bash
rg "refresh_token|expires_at|TokenExpired|session" --glob "*.rs"
```

然后只把命中的关键文件发给模型。

这可能把上下文从：

```text
40 个文件
```

减少到：

```text
3 ~ 6 个文件
```

这就是 token 节省。

---

## 5. prompt caching 也很重要

如果你用的是 OpenAI API，静态内容还可能享受 prompt caching。

OpenAI 官方文档说，Prompt Caching 会自动用于较长 prompt，能降低延迟和输入 token 成本；缓存命中要求 prompt 开头部分完全匹配，因此应该把静态内容放在前面，把每次变化的用户问题、搜索结果、文件片段放在后面。([OpenAI开发者][2])

所以你的请求结构最好是：

```text
固定前缀：
  system prompt
  AI_RULES.md
  工具说明
  小型 repo map
  项目固定约定

动态部分：
  本次任务
  rg 搜索结果
  符号索引查询结果
  相关代码片段
```

不要这样：

```text
本次任务
当前时间
随机生成的 session 信息
system prompt
repo map
代码片段
```

因为前面一变，缓存命中率就下降。

OpenAI 文档也说明，缓存适用于 1024 tokens 以上的 prompt，并且 usage 里会显示 `cached_tokens`；不过 cached tokens 仍然会计入 TPM rate limits。([OpenAI开发者][2])

也就是说：

```text
cached tokens：
  费用可能更低
  延迟可能更低
  但不是“完全不存在”
  也不是“不占 rate limit”
```

---

## 6. 一个很现实的 token 对比

假设你现在改一个功能。

### 不做这些工具

```text
AI_PROJECT.md      3,000 tokens
整个 auth 目录      80,000 tokens
整个 core 目录      120,000 tokens
测试目录            60,000 tokens
聊天历史            20,000 tokens

总计：约 283,000 input tokens
```

### 做了 repo map + rg + 符号索引

```text
固定规则             2,000 tokens
compact repo map     3,000 tokens
rg 搜索结果           1,000 tokens
符号索引结果           800 tokens
相关代码文件          18,000 tokens
相关测试文件           6,000 tokens

总计：约 30,800 input tokens
```

这不是小优化，是一个数量级的差别。

---

## 7. 所有好的 AI 代理都是这样做的吗？

严格说：**不是所有都用同一种实现，但成熟的 coding agent 基本都会做“上下文管理”。**

它们不一定都叫：

```text
repo map
符号索引
RAG
context pack
memory
codebase index
```

但本质上都在解决同一个问题：

```text
大项目太大，不能全部塞进上下文；
所以必须筛选、压缩、索引、按需读取。
```

例如：

Aider 明确使用 repo map，把文件、关键符号、函数签名等压缩后发给 LLM，并且会根据 token budget 选择最相关部分。([Aider][1])

Claude Code 有 `CLAUDE.md`、auto memory、context window 管理、compaction、subagent 等机制。它的文档说明，每个会话从新的 context window 开始，`CLAUDE.md` 和 auto memory 会作为上下文加载；文档也建议保持说明文件简洁，因为它们会消耗上下文。([Claude Code][3])

`AGENTS.md` 则是一个开放格式，目标是给 coding agents 一个固定位置读取项目说明、构建命令、测试命令、代码风格、安全注意事项等。它的网站也列出多个 coding agent / 工具生态正在兼容这种项目级说明文件。([代理商助手][4])

所以更准确的说法是：

```text
好的代码代理几乎都不会“盲目把整个项目发给模型”。

它们通常会做：
  - 项目说明文件
  - 文件搜索
  - 代码索引
  - 上下文压缩
  - 相关文件选择
  - 工具调用
  - 测试/编译验证
```

但它们未必都使用完全一样的 repo map 或符号索引格式。

---

## 8. 最好的 AI 代理通常不是“总结工具”强，而是“上下文调度”强

你可以把一个 coding agent 想象成这样：

```text
LLM 本身：
  会思考、会写代码、会解释。

上下文系统：
  决定给 LLM 看什么。

工具系统：
  帮 LLM 搜索、读取、修改、测试。

验证系统：
  cargo check / cargo test / git diff。
```

真正的差距往往在这里：

```text
差的 agent：
  一上来读一堆文件
  或者只凭文件名猜
  或者改完不测试

好的 agent：
  先看规则
  再看 repo map
  再搜索
  再查符号
  再读少量关键文件
  再修改
  再运行测试
  再根据错误修
```

所以 repo map、符号索引只是基础设施。
更重要的是 agent 的工作流程。

---

## 9. 这些工具什么时候不省 token？

有几种情况可能不省：

### 情况一：项目很小

如果项目只有：

```text
10 个文件
3,000 行代码
```

那 repo map、索引、RAG 的收益不大。

直接让 AI 读相关文件就可以。

---

### 情况二：你每次都发完整 repo map

如果你生成了一个 100,000 tokens 的 repo map，然后每次都发给模型，那当然不省。

应该发：

```text
compact repo map
相关 repo map fragment
```

而不是完整地图。

---

### 情况三：AI 总结 repo map 的成本太高

如果每次任务开始都让 AI 重新总结整个项目，那很浪费。

正确做法：

```text
本地工具生成 repo map
缓存结果
git diff 后增量更新
AI 只读取结果
```

不要每次都让大模型重新扫描总结。

---

### 情况四：工具返回太多内容

比如 `rg "token"` 可能返回几千行。

这时候应该限制：

```bash
rg "token" --glob "*.rs" -n -C 2
```

或者让工具层做截断：

```text
最多返回前 100 条
按文件聚合
只显示命中行上下 2 行
超过阈值让 AI 细化关键词
```

否则搜索结果本身也会吃很多 token。

---

## 10. 我建议你这样设计 API 请求

一个比较好的请求结构：

```text
System:
  你是 Rust 项目代码修改助手。
  必须先定位，再修改，再测试。
  不要猜文件。
  不要大范围重构。

Static project context:
  AI_RULES.md
  AI_PROJECT.md
  compact repo map
  工具使用说明

User task:
  本次要修改的功能

Tool results:
  rg 搜索结果
  符号索引结果
  相关文件片段

Final instruction:
  给出修改计划
  修改代码
  运行 cargo check/test
  输出 diff 摘要
```

如果你的模型 API 支持 prompt caching，就尽量保持前面这部分完全稳定：

```text
System + AI_RULES.md + compact repo map + tools schema
```

把每次变化的东西放后面。

---

## 11. 最推荐的策略

你已经做了这些后，我建议你不要每次都问模型：

```text
请理解整个项目。
```

而是让它执行固定流程：

```text
1. 读取 compact repo map。
2. 根据任务提出关键词。
3. 调用 rg 搜索。
4. 调用符号索引查定义/引用/impl。
5. 选择 3~10 个最可能相关文件。
6. 读取这些文件。
7. 给出修改计划。
8. 修改。
9. 运行测试。
10. 根据错误继续修。
```

这个流程比单纯“给模型很多上下文”更省 token，也更可靠。

---

## 12. 一句话回答

**是的，做好 repo map、符号索引、rg 搜索之后，API 调用通常能明显省 token。**

但前提是：

```text
不要把所有索引全文都发给模型；
只发 compact repo map；
符号索引用工具查询；
rg 结果要截断；
相关文件按需读取；
固定前缀利用 prompt caching；
修改后用 cargo check/test 验证。
```

成熟 coding agent 基本都在做类似的上下文管理，只是名字和实现不同。
它们的共同点不是“用了某个神奇总结工具”，而是：

```text
不让 LLM 盲读整个项目；
让工具先定位；
让模型只读必要上下文；
让测试验证最终修改。
```

[1]: https://aider.chat/docs/repomap.html?utm_source=chatgpt.com "Repository map"
[2]: https://developers.openai.com/api/docs/guides/prompt-caching "Prompt caching | OpenAI API"
[3]: https://code.claude.com/docs/en/memory "How Claude remembers your project - Claude Code Docs"
[4]: https://agents.md/ "AGENTS.md"
