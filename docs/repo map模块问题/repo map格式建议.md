我的问题是，对于ai理解来说，我们到底给什么格式的文档给ai模型，更方便它理解和使用，

对 **AI 理解和使用** 来说，最好的默认答案不是 HTML，而是：

> **干净、结构稳定的 Markdown，必要时外面套 XML 风格的边界标签。**

也就是：

```text
Markdown 负责正文可读性；
XML 标签负责区分不同内容块；
JSON/YAML 负责严格结构化数据；
HTML 只在必须保留网页语义或表格结构时使用，而且要清洗掉噪音。
```

## 核心原则：AI 吃的是 token，不是视觉排版

大模型并不是像人一样“看”页面排版。大多数情况下，它实际处理的是一串文本 token。Anthropic 在谈 context engineering 时也强调，context 本质上就是模型推理时可见的一组 token，而且这是有限资源，需要优化这些 token 的信息价值。([Anthropic][1])

所以，对 AI 来说，文档格式好不好，主要看这几个标准：

```text
1. 边界是否清楚：哪里是标题？哪里是正文？哪里是代码？哪里是引用资料？
2. 层级是否清楚：一级标题、二级标题、步骤、子项有没有明确结构？
3. 噪音是否少：有没有 CSS、导航栏、广告、脚本、重复页眉页脚？
4. token 是否省：同样的信息，是否用了更少的 token 表达？
5. 是否方便模型引用：每段是否有 ID、标题、来源、日期？
6. 是否方便程序处理：需要机器读取时，字段是否稳定？
```

从这个角度看，**原始 HTML 经常不是最佳选择**，因为它有大量对模型理解帮助不大的东西，比如 class、style、div、span、脚本、导航、隐藏元素、广告区域。

---

# 我建议的优先级

## 第一选择：Markdown

给 AI 阅读的普通文档，首选 Markdown。

适合：

```text
README
技术文档
需求文档
架构说明
API 说明
设计文档
会议纪要
问题分析
代码讲解
操作步骤
```

原因是 Markdown 同时满足三个条件：

```text
人容易写；
AI 容易理解；
token 开销相对低。
```

OpenAI 的 GPT-4.1 Prompting Guide 里也建议，选择分隔符时可以从 Markdown 开始，用 Markdown 标题表示主要章节和子章节，用反引号包代码，用列表表达步骤。([OpenAI开发者][2])

例如，比起这样给 AI：

```html
<div class="section">
  <h2>Build</h2>
  <p>Run the following command:</p>
  <pre><code>cargo build --release</code></pre>
</div>
```

更适合这样：

````md
## Build

Run:

```bash
cargo build --release
```
````

模型理解上没有损失，但 token 更少，噪音更少。

---

## 第二选择：Markdown + XML 标签

如果你的文档要喂给 AI 做比较复杂的任务，我最推荐这种格式：

````md
<task>
请根据下面的项目文档，找出 unsafe 代码的风险点，并给出修改建议。
</task>

<context>
# 项目背景

这是一个 Rust 网络库，目标是提供低延迟 TCP 转发能力。

## 关键约束

- 必须支持 Linux
- 尽量避免额外内存分配
- unsafe 只能用于明确性能收益的地方
</context>

<source_doc id="doc-001" title="buffer design">

# Buffer Design

## Current implementation

```rust
// ...
```

## Known issues

- buffer reuse may cause stale data exposure
- lifetime boundary is unclear

</source_doc>

<output_format>
请按照下面格式输出：

## 风险点

## 原因分析

## 建议修改

## 修改后的代码
</output_format>
````

这种格式的好处是：

```text
Markdown 让正文自然可读；
XML 标签让模型知道不同块的角色；
source_doc 的 id 和 title 方便模型引用；
output_format 明确告诉模型该怎么输出。
```

Anthropic 官方文档明确建议用 XML 标签组织复杂 prompt，尤其是当 prompt 混合了 instructions、context、examples、variable inputs 时，把每类内容放进自己的标签可以减少误解。([Claude API Docs][3])

OpenAI 的 GPT-4.1 指南也说，XML 在长上下文测试里表现不错，因为它可以精确包住一个 section、添加 metadata，并支持嵌套。([OpenAI开发者][2])

所以我会把它总结成一句话：

> **Markdown 适合正文，XML 适合边界。**

---

## 第三选择：JSON / YAML

JSON 和 YAML 不是给“长文档阅读”最舒服的格式，但非常适合 **结构化数据**。

适合：

```text
配置
API schema
数据库记录
实体抽取结果
测试用例
任务列表
工具调用参数
规则表
```

例如：

```json
{
  "project": "rust-tcp-proxy",
  "language": "Rust",
  "constraints": [
    "Linux only",
    "low latency",
    "minimize allocation"
  ],
  "review_focus": [
    "unsafe usage",
    "buffer lifetime",
    "error handling"
  ]
}
```

但是不要把大段自然语言强行塞进 JSON：

```json
{
  "content": "第一段很长很长很长...\n第二段很长很长很长...\n第三段..."
}
```

这样反而不舒服，因为：

```text
换行要转义；
引号要转义；
嵌套太深；
token 变多；
人类也难维护。
```

OpenAI 的 GPT-4.1 指南提到，JSON 对代码场景很容易被模型理解，但更冗长，也会因为字符转义增加开销；在大量文档放入长上下文时，JSON 表现反而不佳。([OpenAI开发者][2])

所以我的建议是：

```text
结构化元数据：JSON/YAML
长篇正文：Markdown
多个文档边界：XML 标签
```

---

## 第四选择：清洗后的语义 HTML

HTML 不是完全不好。它在下面这些情况有价值：

```text
原文就是网页；
表格结构很复杂；
需要保留超链接；
需要保留标题层级；
需要保留列表、表格、引用、代码块等语义；
你正在做网页抽取、网页理解、网页测试。
```

但不要把完整网页 HTML 直接丢给 AI。

糟糕示例：

```html
<html>
<head>
<style>...</style>
<script>...</script>
</head>
<body>
<nav>...</nav>
<div class="sidebar">...</div>
<div class="cookie-banner">...</div>
<article>
  ...
</article>
<footer>...</footer>
</body>
</html>
```

更好的做法是只保留正文语义：

```html
<article>
  <h1>Buffer Design</h1>

  <p>This document explains the buffer reuse strategy.</p>

  <h2>Known risks</h2>

  <ul>
    <li>Stale data exposure</li>
    <li>Incorrect lifetime boundary</li>
  </ul>

  <pre><code class="language-rust">
  // code here
  </code></pre>
</article>
```

也就是说：

> **HTML 可以给 AI，但应该是清洗后的 semantic HTML，不是浏览器页面源码。**

如果 HTML 里面充满 `div class="container mx-auto flex..."`，对模型大多是噪音。

---

# 不同格式的实际排名

我会这样排序：

| 场景              | 最推荐格式                           |
| --------------- | ------------------------------- |
| 普通技术文档          | Markdown                        |
| 长文档、多来源资料       | Markdown + XML 标签               |
| 多篇文档喂给 AI 检索/总结 | XML-wrapped Markdown            |
| 严格字段数据          | JSON                            |
| 配置类数据           | YAML / TOML / JSON              |
| 大表格             | CSV / TSV                       |
| 小表格             | Markdown table                  |
| 源代码             | 原始代码 + 文件路径 + fenced code block |
| 网页内容            | 清洗后的 Markdown 或 semantic HTML   |
| PDF             | 先转 Markdown，再补表格和图片说明           |
| Word 文档         | 先转 Markdown                     |
| 幻灯片             | 按页转成 Markdown，每页带标题和备注          |
| 图片/图表           | OCR/图表描述 + 原图，最好加人工说明           |

---

# 给 AI 最舒服的文档格式模板

如果你要把一个项目文档交给 AI，我建议整理成下面这样：

````md
<document id="doc-001" type="design-doc" title="Buffer Design" date="2026-06-10">

# Buffer Design

## Summary

这个模块负责管理 TCP 转发时的读写 buffer。

## Goals

- 降低内存分配次数
- 避免复制
- 保证数据生命周期安全

## Non-goals

- 不支持 Windows
- 不处理 TLS

## Current implementation

```rust
// src/buffer.rs

pub struct Buffer {
    data: Vec<u8>,
}
```

## Important constraints

- 不能暴露未初始化内存
- unsafe 块必须有明确注释
- buffer 复用时必须清空旧数据

## Open questions

1. 是否需要使用 bytes::BytesMut？
2. 是否应该引入 slab allocator？

</document>
````

如果有多份文档：

```md
<documents>

<document id="doc-001" title="Architecture Overview">
# Architecture Overview
...
</document>

<document id="doc-002" title="Buffer Design">
# Buffer Design
...
</document>

<document id="doc-003" title="Unsafe Audit">
# Unsafe Audit
...
</document>

</documents>
```

这比单纯把几万字拼在一起好很多。

---

# 对 Rust 项目，怎么喂给 AI 最好？

你做 Rust 底层开发的话，我建议这样组织：

````md
<project_context>
# Project Context

项目：低延迟 TCP proxy  
语言：Rust  
目标平台：Linux  
重点关注：unsafe、内存生命周期、并发、错误处理、性能
</project_context>

<file path="src/buffer.rs">
```rust
// src/buffer.rs

use std::ptr;

pub struct Buffer {
    ptr: *mut u8,
    len: usize,
    cap: usize,
}
```
</file>

<file path="src/connection.rs">
```rust
// src/connection.rs

pub struct Connection {
    // ...
}
```
</file>

<review_request>
请重点检查：

1. unsafe 是否有 UB 风险
2. 生命周期是否可能悬垂
3. 并发访问是否可能 data race
4. 错误处理是否会导致资源泄漏
5. 有没有更符合 Rust 习惯的安全写法
</review_request>

<output_format>
请输出：

## 总体结论

## 高风险问题

## 中风险问题

## 建议修改

## 修改后的完整代码
</output_format>
````

这种格式比直接丢一堆代码强很多，因为 AI 明确知道：

```text
哪些是项目背景；
哪些是文件；
每个文件路径是什么；
用户要审查什么；
输出格式是什么。
```

OpenAI 的帮助文档也建议，把指令放在 prompt 开头，并用清晰分隔符把 instruction 和 context 分开；同时要具体说明期望的 context、outcome、length、format、style 等。([OpenAI Help Center][4])

---

# 不推荐直接给 AI 的格式

## 1. 原始 PDF

PDF 对人类看起来像文档，但对 AI/程序来说经常很糟糕：

```text
段落顺序可能乱；
页眉页脚混入正文；
两栏排版会错位；
表格可能变成碎片；
代码缩进可能丢失；
图片里的文字可能读不到。
```

最佳做法：

```text
PDF → 提取正文 → 修正顺序 → 转 Markdown → 表格单独转 CSV/Markdown table → 图片单独加说明
```

---

## 2. 原始 HTML 网页源码

尤其是现代前端页面，HTML 里可能有大量：

```text
CSS class
script
tracking code
广告
导航栏
隐藏元素
重复链接
响应式布局容器
```

这些东西会浪费上下文窗口。

最佳做法：

```text
HTML → 提取 article/main → 删除 nav/footer/script/style → 转 Markdown 或保留 semantic HTML
```

---

## 3. 没有标题的纯文本

纯文本不是不能用，但如果很长，就容易让模型迷路。

不好：

```text
这个模块负责buffer。然后还有connection。然后unsafe在这里。然后错误处理是这样。然后...
```

更好：

```md
# Buffer 模块

## 职责

## 数据结构

## unsafe 使用点

## 已知问题

# Connection 模块

## 职责

## 状态机

## 错误处理
```

---

# 最推荐的实际方案

如果你是在做自己的知识库、项目文档、AI coding agent 上下文，我建议采用这个规范：

```text
正文文档：Markdown
多文档封装：XML 标签
元数据：YAML front matter
表格数据：CSV / Markdown table
代码：带文件路径的 fenced code block
机器输出：JSON schema
网页资料：清洗后转 Markdown
PDF/Word：转 Markdown 后再喂
```

一个完整格式可以长这样：

````md
---
id: rust-buffer-design-001
title: Buffer Design
type: design-doc
project: tcp-proxy
language: Rust
updated: 2026-06-10
---

<document>

# Buffer Design

## Purpose

这个文档说明 buffer 模块的设计。

## Constraints

- 必须避免未初始化内存暴露
- 尽量减少复制
- unsafe 必须有 safety comment

## Source Code

<file path="src/buffer.rs">

```rust
// src/buffer.rs

pub struct Buffer {
    data: Vec<u8>,
}
```

</file>

## Questions for AI

1. 这个设计有没有内存安全风险？
2. 是否可以改成更安全的 Rust 写法？
3. 是否有性能损失？

</document>
````

---

# 最简结论

对于 AI 理解，格式优先级大概是：

```text
1. Markdown：默认最佳
2. Markdown + XML 标签：复杂任务最佳
3. JSON/YAML：结构化数据最佳
4. CSV/TSV：大表格最佳
5. 清洗后的 semantic HTML：网页语义保留时可用
6. 原始 HTML：通常不推荐
7. PDF/DOCX：最好先转换
```

所以，“用 HTML 取代 Markdown”这个说法对 **AI 理解** 来说并不准确。

更好的说法是：

> **不要给 AI 看“漂亮文档”，要给 AI 看“结构清楚、噪音少、边界明确、token 高效”的文档。**

我自己的首选是：

```text
Markdown 写内容；
XML 包边界；
YAML/JSON 放元数据；
CSV 放大表；
HTML 只用于保留网页语义，不作为默认输入格式。
```

[1]: https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents "Effective context engineering for AI agents \ Anthropic"
[2]: https://developers.openai.com/cookbook/examples/gpt4-1_prompting_guide "GPT-4.1 Prompting Guide"
[3]: https://docs.anthropic.com/en/docs/build-with-claude/prompt-engineering/multishot-prompting?utm_source=chatgpt.com "Prompting best practices - Claude API Docs"
[4]: https://help.openai.com/en/articles/6654000-best-practices-for-prompt-engineering-with-openai-api "Best practices for prompt engineering with the OpenAI API | OpenAI Help Center"


对于让ai理解我的大型项目，我做了repo map以及符号索引，以及向量数据库等一大堆辅助工具，我到底应该输出什么格式给ai，让它能够更快的理解我的项目，然后帮我重构项目相关的功能，


我建议你把最终喂给 AI 的东西定义成一种 **Task Context Pack（任务上下文包）**，格式用：

> **XML 风格标签包住 Markdown，代码用 fenced code block，索引用紧凑表格/行记录，源码片段必须带 path、line range、symbol id、hash。**

不要把 repo map、符号索引、向量库结果原封不动塞给模型。
正确做法是：**索引和向量库留给你的工具系统使用，最后只投影出一个“针对当前重构任务裁剪过的上下文包”。**

OpenAI 官方也建议用 Markdown 标题/列表表达层级，用 XML 标签划分上下文边界，并用 XML 属性承载可引用的元数据；在长上下文中，XML-wrapped document 的表现也被明确提到比较好，而大批文档用 JSON 包起来反而可能更差，因为 JSON 会更啰嗦、转义更多。([OpenAI开发者][1]) Anthropic 的上下文窗口文档也强调，context 更大不等于更好；随着 token 增加，准确性和召回可能下降，所以“选择什么放进上下文”跟“窗口有多大”一样重要。([Claude API Docs][2])

---

# 直接结论

对于你的场景，我会这样分层：

```text
长期存储格式：
  symbols.jsonl / edges.tsv / chunks.jsonl / summaries.md / repo_map.md

最终喂给 AI 的格式：
  task_context_pack.md

task_context_pack.md 内部结构：
  XML-like tags + Markdown + fenced code blocks

不要作为最终输入的格式：
  原始 HTML
  巨大的 JSON
  整个 repo 拼接
  没有 path/line/symbol 的散乱代码片段
  向量数据库原始 embedding
```

一句话：

> **工具系统用 JSONL/TSV/SQLite/向量库；模型最终吃 XML-wrapped Markdown Context Pack。**

---

# 你真正需要输出的不是 repo map，而是“任务切片”

大型项目重构时，AI 最需要的不是整个项目，而是这 8 类信息：

| 内容         | 是否必须 | 作用               |
| ---------- | ---: | ---------------- |
| 当前任务目标     |   必须 | 防止模型泛泛理解项目       |
| 项目全局简述     |   必须 | 让模型知道系统边界        |
| 架构/模块地图    |   必须 | 让模型知道大方向         |
| 相关符号图      |   必须 | 让模型知道调用关系        |
| 需要修改的完整代码  |   必须 | 让模型能真正改代码        |
| 调用方/被调用方摘要 |   必须 | 防止改坏接口           |
| 相关测试和构建命令  |   必须 | 让模型知道怎么验证        |
| 不变量/约束/历史坑 | 强烈建议 | 防止模型做“看似正确”的危险重构 |

研究上也有类似结论：直接拼接整个仓库会超出上下文并导致性能下降，而保留文件拓扑依赖、函数级建模、裁剪无关实现，可以减少输入长度并提高 repo-level code completion 的效果。([arXiv][3]) 最近关于代码库可视化的 context engineering 研究也采用了“把完整 IR 压缩成任务相关视图”的思路，而不是把全部中间表示直接交给模型。([arXiv][4])

---

# 推荐格式：Task Context Pack v1

下面是我建议你最终输出给 AI 的主格式。

````md
<!-- task_context_pack.md -->

<task_context_pack version="1.0" repo="your-repo-name" commit="abc123" generated_at="2026-06-10">

<instructions>
你是一个资深 Rust 重构助手。

请严格遵守：
1. 优先基于本上下文中的源码、符号图、测试和约束进行判断。
2. 所有关于项目现状的判断都必须引用 path:line 或 symbol id。
3. 不要修改没有提供源码的文件；如果必须修改，请放入 missing_context。
4. 重构必须保持现有公共 API 行为，除非 task 明确要求破坏性变更。
5. 输出修改方案前，先说明影响范围。
6. 最终输出 unified diff，或者按文件输出完整代码。
</instructions>

<task>
## 当前任务

重构 buffer/session 相关功能，把旧的同步 buffer 管理逻辑迁移为新的 async-safe buffer pool。

## 成功标准

- 现有 public API 尽量不变
- 不能引入 unsafe UB
- 不能增加明显的内存拷贝
- cargo test 通过
- 相关 benchmark 不应明显退化
</task>

<project_brief>
## 项目简述

这是一个 Rust 网络服务项目，核心目标是低延迟 TCP/UDP 转发。

## 关键模块

- `crates/core`: 协议无关的 buffer、session、runtime 抽象
- `crates/proxy`: 代理入口、连接生命周期管理
- `crates/protocol`: 协议解析和编码
- `crates/tests`: 集成测试

## 全局约束

- Linux 是主要目标平台
- async runtime 使用 tokio
- buffer 相关代码对性能敏感
- unsafe 必须有 safety comment
</project_brief>

<architecture_map>
## 架构地图

```text
accept loop
  -> connection manager
    -> session
      -> protocol decoder
      -> buffer pool
      -> transport writer
```

## 数据流

```text
socket read
  -> BufferPool::acquire
  -> Session::on_read
  -> Decoder::decode
  -> Handler::handle_frame
  -> Writer::flush
  -> BufferPool::release
```

## 重构敏感点

- buffer 生命周期
- session 并发访问
- backpressure
- 错误路径上的资源释放
- unsafe 内存初始化边界
</architecture_map>

<repo_map>
## 相关目录

```text
crates/core/src/buffer/
  mod.rs              buffer 模块入口
  pool.rs             BufferPool 实现
  chunk.rs            BufferChunk 数据结构
  metrics.rs          buffer 指标

crates/core/src/session/
  mod.rs              Session 类型入口
  state.rs            SessionState 状态机
  read.rs             读路径
  write.rs            写路径

crates/proxy/src/
  server.rs           accept loop
  connection.rs       连接生命周期
```

## 相关 crate 依赖

```text
proxy -> core
core -> protocol
core -> tokio
protocol -> bytes
```
</repo_map>

<symbol_graph format="compact-adjacency-list">
## 关键符号

```text
S1 struct core::buffer::BufferPool
  path=crates/core/src/buffer/pool.rs
  lines=12-118
  role=edit-target
  summary=管理可复用 BufferChunk，当前基于 VecDeque，没有 async-aware lease 语义

S2 struct core::buffer::BufferChunk
  path=crates/core/src/buffer/chunk.rs
  lines=8-96
  role=edit-target
  summary=持有 Vec<u8>，包含 len/capacity/reset 逻辑

S3 fn core::session::Session::on_read
  path=crates/core/src/session/read.rs
  lines=33-112
  role=caller
  summary=读 socket 后获取 buffer 并送入 decoder

S4 fn core::session::Session::close
  path=crates/core/src/session/state.rs
  lines=120-170
  role=resource-release
  summary=关闭 session 并释放 pending buffers
```

## 调用关系

```text
S3 -> S1.acquire        calls
S3 -> Decoder::decode   calls
S3 -> S1.release        calls-on-success
S4 -> S1.release_all    calls-on-close
S1.acquire -> S2.reset  mutates
S1.release -> S2.clear  mutates
```

## 风险边

```text
S3 -> S1.release
  risk=release may be skipped on decode error
  evidence=crates/core/src/session/read.rs:L80-L95

S1.acquire -> S2.reset
  risk=old bytes may remain visible if len/cap invariant is broken
  evidence=crates/core/src/buffer/pool.rs:L44-L59
```
</symbol_graph>

<invariants>
## 必须保持的不变量

1. `BufferChunk::len` 不能超过 initialized length。
2. `BufferPool::release` 之后，外部不能再持有可写引用。
3. decode error 路径必须释放 buffer。
4. session close 必须释放所有 pending buffer。
5. public API 中 `Session::on_read` 的错误语义不能改变。
</invariants>

<relevant_files>

<file path="crates/core/src/buffer/pool.rs" role="edit-target" symbols="S1" lines="1-150" sha256="...">
```rust
// crates/core/src/buffer/pool.rs

// 这里放完整源码，不要省略
```
</file>

<file path="crates/core/src/buffer/chunk.rs" role="edit-target" symbols="S2" lines="1-130" sha256="...">
```rust
// crates/core/src/buffer/chunk.rs

// 这里放完整源码，不要省略
```
</file>

<file path="crates/core/src/session/read.rs" role="caller" symbols="S3" lines="1-150" sha256="...">
```rust
// crates/core/src/session/read.rs

// 这里放完整源码，不要省略
```
</file>

</relevant_files>

<neighbor_summaries>
## 没有全文展开，但可能受影响的邻居

```text
N1 crates/core/src/session/write.rs
  reason=写路径也可能持有 BufferChunk
  important_symbols=Session::flush, PendingWrite
  needed_if=修改 BufferChunk ownership 或 lifetime

N2 crates/proxy/src/connection.rs
  reason=connection close 时触发 Session::close
  important_symbols=Connection::run, Connection::shutdown
  needed_if=修改 session close/error semantics
```
</neighbor_summaries>

<tests>
## 相关测试

```text
cargo test -p core buffer
cargo test -p core session
cargo test -p proxy connection_lifecycle
cargo test --workspace
```

## 相关测试文件

<file path="crates/core/tests/buffer_pool.rs" role="test" lines="1-220" sha256="...">
```rust
// crates/core/tests/buffer_pool.rs

// 这里放完整测试源码
```
</file>

<file path="crates/core/tests/session_read.rs" role="test" lines="1-180" sha256="...">
```rust
// crates/core/tests/session_read.rs

// 这里放完整测试源码
```
</file>
</tests>

<retrieval_evidence>
## 向量库/符号检索结果

<chunk id="C1" source="vector+symbol" score="0.91" path="crates/core/src/buffer/pool.rs" lines="12-118" reason="BufferPool 是本次重构核心类型">
摘要：当前 BufferPool 使用 VecDeque 复用 BufferChunk，release 依赖调用方手动归还。
</chunk>

<chunk id="C2" source="graph" score="0.86" path="crates/core/src/session/read.rs" lines="33-112" reason="Session::on_read 是 BufferPool 的主要调用方">
摘要：on_read 在 decode 成功路径释放 buffer，但错误路径需要检查。
</chunk>
</retrieval_evidence>

<output_contract>
请输出：

1. 影响范围分析
2. 需要修改的符号列表
3. 重构方案
4. 风险点
5. 具体 patch
6. 需要新增或修改的测试
7. 如果上下文不足，列出 missing_context，不要猜
</output_contract>

</task_context_pack>
````

这个格式的核心不是“好看”，而是让模型能快速回答几个问题：

```text
我要做什么？
这个项目大概怎么工作？
哪些文件能改？
哪些符号相关？
哪些调用方会受影响？
哪些不变量不能破坏？
怎么验证？
如果上下文不够，应该要什么？
```

---

# 你的 repo map 应该长什么样？

不要输出完整树形目录。大型项目里目录树太长，很多信息没价值。repo map 应该是 **语义地图**，不是 `tree -a`。

推荐 repo map 包含这些字段：

```text
1. crate/package/module 的职责
2. 入口点
3. 数据流
4. 控制流
5. 关键 trait/interface
6. 状态机
7. 资源生命周期
8. 错误处理边界
9. unsafe/FFI 边界
10. 测试入口
```

一个更紧凑的 repo map 可以这样写：

````md
<!-- repo_map.md -->

# Repo Map

## Workspace

| crate | role | key entrypoints | depends on |
|---|---|---|---|
| core | buffer/session/runtime 抽象 | Session, BufferPool | protocol, tokio |
| proxy | 网络入口和连接生命周期 | Server, Connection | core |
| protocol | 协议解析 | Decoder, Encoder | bytes |

## Main Runtime Flow

```text
proxy::Server::run
  -> proxy::Connection::run
    -> core::Session::on_read
      -> core::buffer::BufferPool::acquire
      -> protocol::Decoder::decode
      -> core::buffer::BufferPool::release
````

## Critical Invariants

* BufferChunk 不能暴露未初始化内存
* Session close 必须释放所有 pending resources
* Decoder 不能持有超过当前 read cycle 的 borrowed buffer

````

这比几千行目录树更有价值。

---

# 你的符号索引应该怎么投影给 AI？

底层你可以用 JSONL 存，但最终喂给模型时，不要给它巨大的 JSON 数组。  
OpenAI 的长上下文建议里明确提到，JSON 对代码场景能理解，但在大量文档输入时可能因为冗长和转义带来额外负担；Markdown 和 XML 更适合作为 prompt 分隔和长上下文组织方式。:contentReference[oaicite:4]{index=4}

底层存储可以是这样：

```text
repo_index.symbols.jsonl
repo_index.edges.tsv
repo_index.chunks.jsonl
repo_index.tests.jsonl
````

但是投影给 AI 时建议变成这种紧凑文本：

```text
S1 struct core::buffer::BufferPool
  path=crates/core/src/buffer/pool.rs
  lines=12-118
  visibility=pub(crate)
  role=edit-target
  summary=管理 BufferChunk 复用，当前没有 lease 语义
  methods=acquire, release, release_all
  invariants=release 后不能再被外部持有

S2 fn core::session::Session::on_read
  path=crates/core/src/session/read.rs
  lines=33-112
  role=caller
  calls=S1.acquire, protocol::Decoder::decode, S1.release
  error_paths=decode_error, socket_closed
```

为什么不用完整 JSON？

因为模型看这个：

```json
{
  "id": "S1",
  "kind": "struct",
  "qualified_name": "core::buffer::BufferPool",
  "path": "crates/core/src/buffer/pool.rs",
  "range": {
    "start": 12,
    "end": 118
  },
  "visibility": "pub(crate)",
  "role": "edit-target",
  "summary": "..."
}
```

能理解，但 token 成本更高。少量 JSON 没问题，大规模上下文里就不划算。

---

# 向量数据库结果应该怎么给 AI？

不要给 embedding，不要只给 top-k chunk 文本。
你应该给 **带解释的检索证据**：

```md
<retrieval_evidence>

<chunk id="C17" source="vector" score="0.89" path="crates/core/src/buffer/pool.rs" lines="12-118" symbols="S1" reason="query 命中 buffer pool acquire/release 语义">
## 摘要

BufferPool 当前通过 VecDeque 保存 BufferChunk，acquire 弹出一个 chunk，release 推回池子。

## 相关风险

release 是手动调用；如果 decode error 提前 return，可能跳过 release。
</chunk>

<chunk id="C24" source="symbol_graph" score="0.84" path="crates/core/src/session/read.rs" lines="33-112" symbols="S3" reason="S3 是 S1.acquire 的主要调用方">
## 摘要

Session::on_read 在读取 socket 后调用 BufferPool::acquire，然后交给 Decoder。
</chunk>

</retrieval_evidence>
```

重点是 `reason`。
因为模型不只需要知道“这段相关”，还需要知道“为什么相关”。

你的检索系统最好混合四种信号：

```text
1. vector similarity：语义相似
2. exact symbol match：精确符号命中
3. graph distance：调用图/依赖图距离
4. test linkage：哪些测试覆盖这个符号
```

一个简单排序公式可以是：

```text
score =
  0.35 * vector_score
+ 0.25 * symbol_match_score
+ 0.20 * graph_proximity_score
+ 0.10 * test_relevance_score
+ 0.10 * recent_change_score
```

向量检索适合“找到语义相关”，但重构更依赖 **调用关系、所有权关系、测试关系、错误路径关系**。所以不要只靠 vector DB。

---

# 源码应该怎么放？

源码片段必须满足 5 个条件：

```text
1. 有 path
2. 有 line range
3. 有 symbol id
4. 有 role
5. 有 hash 或 commit
```

例如：

````md
<file path="crates/core/src/buffer/pool.rs" role="edit-target" symbols="S1" lines="1-150" sha256="abc...">
```rust
// crates/core/src/buffer/pool.rs

use std::collections::VecDeque;

pub struct BufferPool {
    // ...
}
```
</file>
````

其中 `role` 很重要：

```text
edit-target：模型可以修改
caller：调用方，主要用于判断影响
callee：被调用方，主要用于理解约束
interface：公共 trait/API，不要随便破坏
test：相关测试
config：构建/feature/config
example：用法示例
generated：生成代码，默认不要改
vendor：第三方代码，默认不要改
```

这样模型不会把所有文件当成同等重要。

---

# 全量代码、摘要、签名分别什么时候放？

不要全部都放完整代码。推荐规则：

| 类型                           | 给 AI 的内容       |
| ---------------------------- | -------------- |
| 本次要修改的文件                     | 完整源码           |
| 直接调用方                        | 完整函数，或完整文件     |
| 直接被调用方                       | 签名 + 关键实现      |
| public trait / public struct | 完整定义           |
| 错误类型                         | 完整 enum / impl |
| 状态机                          | 完整状态定义 + 转移函数  |
| 测试                           | 相关测试完整给        |
| 无关工具函数                       | 只给摘要           |
| vendor/generated             | 默认不给           |
| Cargo.toml / features        | 和任务相关就给        |

对于 Rust 项目，我会特别强制完整给这些东西：

```text
Cargo.toml
feature flags
public trait
unsafe 代码块
生命周期相关 struct/impl
Send/Sync 相关 impl
Drop impl
Pin/Unpin 相关代码
FFI 边界
tokio task spawn 边界
channel/resource close 边界
错误类型和 From impl
相关测试
```

Rust 重构里，很多 bug 不是函数内部错，而是边界错：ownership、lifetime、Drop、Send/Sync、feature flag、error conversion、async cancellation。

---

# 最适合你的整体输出体系

我建议你的工具链最后生成 5 类文件。

## 1. `repo_brief.md`

长期稳定，短小，几百行以内。

```text
项目是什么
架构是什么
关键约束是什么
怎么 build/test
哪些目录最重要
哪些规则不能破坏
```

类似 Claude Code 的 `CLAUDE.md` 也强调项目级记忆应该放构建命令、测试说明、项目约定、架构决策等，并且应该简洁、结构化；它还建议大项目用 path-scoped rules，而不是把所有规则塞进一个巨大文件。([Claude API Docs][5])

## 2. `repo_map.md`

项目语义地图。

```text
crate/module 职责
入口点
主要数据流
主要控制流
核心 trait
状态机
资源生命周期
```

## 3. `symbols.jsonl`

给工具用，不直接全部给模型。

每行一个符号：

```json
{"id":"S1","kind":"struct","qname":"core::buffer::BufferPool","path":"crates/core/src/buffer/pool.rs","range":[12,118],"summary":"管理 BufferChunk 复用","methods":["acquire","release"]}
```

## 4. `edges.tsv`

给工具用，做调用图/依赖图。

```text
src	dst	kind	weight	evidence
S3	S1.acquire	calls	0.95	crates/core/src/session/read.rs:L52
S3	S1.release	calls-on-success	0.90	crates/core/src/session/read.rs:L88
S4	S1.release_all	calls-on-close	0.85	crates/core/src/session/state.rs:L144
```

## 5. `task_context_pack.md`

每次用户提出具体重构任务时动态生成。
这是唯一应该直接喂给模型的大上下文文件。

---

# 最终喂给模型的顺序

建议顺序是：

```text
1. Instructions
2. Task
3. Output Contract
4. Project Brief
5. Architecture Map
6. Repo Map
7. Symbol Graph
8. Invariants
9. Relevant Full Source Files
10. Neighbor Summaries
11. Tests
12. Retrieval Evidence
13. Final Instructions
```

OpenAI 的长上下文建议里提到，长上下文场景下，指令位置会影响表现；如果上下文很长，最好在上下文前后都放关键指令。([OpenAI开发者][6])

所以你的 context pack 最前面放一次规则，最后再重复一次核心要求：

```md
<final_instructions>
请基于上面的 context 完成重构。
不要修改没有给出源码的文件。
所有判断引用 path:line 或 symbol id。
如果上下文不足，先列 missing_context。
输出 patch 前先给影响范围。
</final_instructions>
```

---

# 重构任务的上下文选择算法

你的工具可以按下面逻辑生成 `task_context_pack.md`：

```text
输入：
  用户任务描述

步骤 1：任务解析
  提取关键词、模块名、功能名、错误信息、测试名、符号名

步骤 2：召回候选
  vector search top 30
  symbol exact search top 30
  grep/ripgrep top 30
  test linkage top 20
  graph neighbors top 50

步骤 3：图扩展
  对 edit-target 做 caller/callee 扩展
  深度 1：完整代码
  深度 2：签名 + 摘要
  深度 3：只列 symbol 名和 reason

步骤 4：重构风险补全
  自动加入：
    public API
    trait impl
    error type
    Drop impl
    unsafe block
    feature flags
    tests
    benchmarks

步骤 5：裁剪
  去重
  删除 generated/vendor
  删除低分 chunk
  合并同文件相邻 line range
  优先保留完整函数，不要截断函数体

步骤 6：生成上下文包
  输出 XML-wrapped Markdown
  每个片段带 path/line/symbol/reason/hash
```

对于重构，**完整函数比随机 chunk 更重要**。
不要给模型这种片段：

```rust
if let Some(buf) = self.pool.pop_front() {
    ...
}
```

要给完整函数：

```rust
// crates/core/src/buffer/pool.rs

impl BufferPool {
    pub fn acquire(&mut self) -> BufferChunk {
        ...
    }

    pub fn release(&mut self, chunk: BufferChunk) {
        ...
    }
}
```

因为模型需要看到完整控制流和错误路径。

---

# 推荐 token 预算

如果你的模型上下文比较大，比如 128k 或以上，可以这样分配：

| 区域        |      建议预算 |
| --------- | --------: |
| 指令 + 输出格式 |   1k - 3k |
| 项目简述      |   1k - 3k |
| repo map  |  3k - 10k |
| 符号图/调用图切片 |  5k - 20k |
| 不变量/约束    |   1k - 5k |
| 需要修改的完整代码 | 30k - 80k |
| 调用方/被调用方  | 10k - 40k |
| 测试        | 10k - 40k |
| 检索证据摘要    |  3k - 10k |

我的经验判断：

```text
重构类任务：
  完整源码 + 测试 > repo map > 向量摘要

理解类任务：
  repo map + 符号图 > 完整源码

排 bug：
  错误日志 + 测试 + 相关调用链 > repo map

设计新功能：
  public API + 架构约束 + 相似实现 > 全量源码
```

---

# 不要这么做

## 1. 不要把整个项目拼成一个巨大文本

这会让模型“看过”，但不一定“用对”。长上下文会出现注意力稀释和 recall 退化，Anthropic 文档称更多 context 并不自动更好，并强调上下文筛选的重要性。([Claude API Docs][2])

## 2. 不要输出原始 HTML

HTML 对代码重构没有优势。大量 `<div>`、class、style、导航结构只会浪费 token。
除非你分析的是前端 DOM 或网页语义，否则不要用 HTML。

## 3. 不要把 symbol index 全量 JSON 给模型

JSONL 适合工具，不适合大规模直接喂模型。
最终给模型时，转成 compact symbol records。

## 4. 不要只给向量 top-k

向量 top-k 经常漏掉这些东西：

```text
调用方
测试
错误类型
feature flag
trait impl
Drop impl
unsafe 边界
配置文件
```

这些在重构里非常关键。

## 5. 不要给没有来源的摘要

坏：

```text
BufferPool 可能有释放问题。
```

好：

```text
BufferPool 可能有释放问题。
evidence=crates/core/src/session/read.rs:L80-L95
reason=decode error 分支提前 return，未看到 release 调用
```

---

# 我会给你的最终规范命名为 RCP

你可以把它叫：

```text
RCP: Refactor Context Pack
```

最小字段：

```text
<rcp>
  <instructions>
  <task>
  <project_brief>
  <architecture_map>
  <repo_map>
  <symbol_graph>
  <invariants>
  <relevant_files>
  <tests>
  <output_contract>
</rcp>
```

完整字段：

```text
<rcp>
  <metadata>
  <instructions>
  <task>
  <success_criteria>
  <project_brief>
  <architecture_map>
  <repo_map>
  <symbol_graph>
  <data_flow>
  <control_flow>
  <state_machines>
  <invariants>
  <public_api_contracts>
  <unsafe_boundaries>
  <feature_flags>
  <relevant_files>
  <neighbor_summaries>
  <retrieval_evidence>
  <tests>
  <build_commands>
  <known_failures>
  <missing_context_policy>
  <output_contract>
  <final_instructions>
</rcp>
```

---

# 如果模型可以调用你的工具

如果你是做 agent，而不是一次性 chat，那么更好的方式是：

```text
初始上下文只给：
  repo_brief
  repo_map
  tool schema
  task

让模型按需调用：
  search_symbols
  get_symbol
  get_callers
  get_callees
  get_file
  get_tests_for_symbol
  get_recent_changes
```

工具返回仍然用同一种格式：

````md
<tool_result tool="get_symbol" symbol="S1">
<symbol id="S1" qname="core::buffer::BufferPool" path="crates/core/src/buffer/pool.rs" lines="12-118">
```rust
// crates/core/src/buffer/pool.rs

// 完整符号源码
```
</symbol>
</tool_result>
````

这样你的系统会更像“可交互代码库”，而不是“超长 prompt”。

---

# 对 Rust 重构，我建议额外加入这些 section

Rust 项目尤其需要这些：

````md
<rust_safety_context>
## Unsafe Blocks

```text
U1 crates/core/src/buffer/chunk.rs:L44-L61
  reason=Vec::set_len after reading into spare capacity
  safety_contract=caller must initialize bytes before len update
  related_symbols=S2, S7
````

## Send/Sync Boundaries

```text
T1 BufferChunk
  Send=true
  Sync=false
  reason=contains Vec<u8>, no interior mutability

T2 BufferLease
  Send=?
  risk=if lease crosses await point, pool ownership semantics matter
```

## Drop Semantics

```text
D1 BufferLease::drop
  expected=return chunk to pool
  risk=drop during panic/cancellation path
```

## Feature Flags

```text
default = ["tokio-runtime"]
metrics = ["dep:metrics"]
unsafe-fast-buffer = []
```

</rust_safety_context>

````

对于 Rust async 重构，我还会加入：

```text
哪些对象会跨 await
哪些引用不能跨 await
哪些锁可能被 await 持有
哪些 Drop 路径负责资源释放
哪些错误路径提前 return
哪些类型需要 Send + 'static
````

这比单纯给源码重要得多。

---

# 最终答案

你现在已经有 repo map、符号索引、向量数据库，这很好。
下一步不要再纠结“md、html、json 哪个更高级”，而是做一个统一的 **Context Projection Layer**：

```text
底层：
  repo map / symbols / graph / vector DB / tests / git history

中间：
  根据任务选出 relevant slice

最终：
  输出 task_context_pack.md
  格式 = XML-wrapped Markdown + fenced code + compact symbol graph
```

最推荐的最终输入格式就是：

```text
task_context_pack.md
```

里面用：

```text
XML 标签：划分边界和元数据
Markdown：写说明、表格、列表
fenced code block：放源码
compact adjacency list：放调用图
path:line + symbol id：做引用
hash/commit：防止上下文过期
```

这比 HTML 更适合 AI 理解代码，也比大 JSON 更省 token、可读性更高。

[1]: https://developers.openai.com/api/docs/guides/prompt-engineering "Prompt engineering | OpenAI API"
[2]: https://docs.anthropic.com/en/docs/build-with-claude/context-windows "Context windows - Claude API Docs"
[3]: https://arxiv.org/abs/2406.18294 "[2406.18294] Hierarchical Context Pruning: Optimizing Real-World Code Completion with Repository-Level Pretrained Code LLMs"
[4]: https://arxiv.org/abs/2605.24453 "[2605.24453] Code2UML: Agentic LLMs with context engineering for scalable software visualization"
[5]: https://docs.anthropic.com/en/docs/claude-code/memory "How Claude remembers your project - Claude Code Docs"
[6]: https://developers.openai.com/cookbook/examples/gpt4-1_prompting_guide "GPT-4.1 Prompting Guide"
