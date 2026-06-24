是，下一步可以做 **RAG**，但我建议你不要把它理解成“上一个向量数据库就完事”。

你现在有了：

```text
repo map       = 仓库结构和重要代码概览
symbol index   = 函数 / 类型 / trait / method / module 的精确索引
```

下一步真正要做的是：

```text
Code Context Retrieval Engine
也就是：代码场景下的 Hybrid RAG / Context Engine
```

**向量数据库只是其中一层，不是核心本体。**

---

# 结论先说

你的下一步优先级应该是：

```text
1. 做混合检索 Hybrid Retrieval
2. 做上下文排序 Ranking
3. 做上下文打包 Context Packing
4. 做检索效果评测 Evaluation
5. 再加向量数据库 / embedding
6. 最后做 Agent 工具接口，比如 CLI / MCP / IDE plugin
```

不是：

```text
repo map + symbol index
        ↓
直接全量 embedding
        ↓
丢进 vector DB
        ↓
让 AI 自己搜
```

这个路线很容易变成“看起来很 AI，但效果不稳定”。

更好的路线是：

```text
repo map + symbol index
        ↓
符号精确召回
        ↓
全文/BM25 召回
        ↓
调用图/依赖图扩展
        ↓
向量语义召回
        ↓
rerank
        ↓
token budget 打包
        ↓
给 AI
```

OpenAI 的 Retrieval 文档把 vector store 描述为数据索引，用来做 semantic search；这说明向量库适合“语义检索”，但代码理解还需要符号、引用、定义、文件路径这些结构化信息。([OpenAI开发者][1]) Aider 的 repo map 也不是纯向量检索，而是把重要 class/function、类型和调用签名压缩成仓库地图，帮助模型理解代码关系。([Aider][2])

---

# 1. 你下一步真正要做的东西：Context Engine

可以把它叫：

```text
Context Engine
Code RAG Engine
Hybrid Retrieval Engine
AI Codebase Context Layer
```

它的职责是：

> 用户问一个问题或者要求改代码时，系统自动判断应该把哪些文件、哪些符号、哪些代码片段、哪些测试、哪些调用关系交给 AI。

例如用户说：

```text
帮我修改登录逻辑，让密码错误时返回 401，而不是 500。
```

你的系统应该自动召回：

```text
AuthService::login
PasswordHasher::verify
AuthError / AppError
login_handler
LoginRequest / LoginResponse
相关测试 tests/auth_login_test.rs
错误映射层，比如 error_to_response()
```

而不是只靠向量库搜“登录逻辑 密码错误 401”。

---

# 2. 为什么不能只做向量数据库？

因为代码检索和普通文档检索不同。

普通文档问答里，用户问：

```text
公司报销规则是什么？
```

向量检索很有效，因为语义相似就够了。

但代码里，用户可能问：

```text
这个 create_session 为什么没生效？
```

真正关键的东西可能是：

```rust
SessionStore::create
SessionRepository::insert
AuthService::login
set_cookie_header
SESSION_COOKIE_NAME
```

这些信息有很多是 **精确符号关系**，不是自然语言语义关系。

向量库擅长找：

```text
语义相似的代码
注释相似的代码
需求描述相似的模块
README / docs / issue / 测试说明
```

但它不擅长保证：

```text
这个函数真实存在
这个 trait 的实现在哪里
这个函数被谁调用
这个错误类型在哪里转换成 HTTP status
这个 pub use 最终指向哪里
```

这些应该由你的 **symbol index + relation graph + full-text search** 负责。SCIP 这类代码索引协议的目标也是支撑 go to definition、find references、find implementations 这类精确代码导航能力。([GitHub][3])

所以正确答案是：

> **要做 RAG，但要做 Code RAG。Code RAG 的核心不是 vector DB，而是 hybrid retrieval。**

---

# 3. 推荐架构

你的系统可以演进成这样：

```text
                User Query
                    │
                    ▼
            Query Understanding
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
 Symbol Search   Text Search   Vector Search
        │           │           │
        └──────┬────┴────┬──────┘
               ▼         ▼
        Graph Expansion  Recent Context
               │         │
               └────┬────┘
                    ▼
                Reranker
                    │
                    ▼
             Context Packer
                    │
                    ▼
              LLM / Agent
```

其中每一层的职责不同。

---

# 4. 第一优先级：Hybrid Retrieval，而不是 Vector-only RAG

你至少应该有 4 种召回器。

## 1. Symbol Retriever

从你的符号索引里查：

```text
AuthService
AuthService::login
login_handler
UserRepository::find_by_email
AppError::Unauthorized
```

它负责高精度召回。

适合处理：

```text
函数名
类型名
trait 名
错误类型
模块路径
文件路径
```

例如：

```text
query: "修改 AuthService::login"
```

直接命中：

```text
crate::service::auth_service::AuthService::login
src/service/auth_service.rs:42-88
```

这是向量库很难替代的。

---

## 2. Full-text / BM25 Retriever

查源码文本、注释、错误字符串、测试名。

例如用户说：

```text
报错 duplicate key value violates unique constraint
```

这类东西最好用全文搜索，不一定适合向量。

你可以用：

```text
SQLite FTS5
Tantivy
ripgrep
```

SQLite FTS5 是 SQLite 官方的全文搜索虚拟表模块，可以高效查找包含某些搜索词的文档集合。([SQLite官网][4]) Tantivy 是 Rust 写的全文搜索引擎库，定位更像 Lucene 级别的 embedded search library。([GitHub][5])

对你的 Rust 项目来说，MVP 可以先用：

```text
SQLite FTS5
```

后面需要更强 ranking / tokenizer / 大规模索引时再换：

```text
Tantivy
```

---

## 3. Graph Retriever

这个是你已经做了 symbol index 之后最应该加的能力。

它根据关系扩展上下文：

```text
定义 → 调用方
定义 → 被调用方
trait → impl
impl → trait
handler → service
service → repository
service → test
error → mapper
config → usage
```

例如命中：

```text
AuthService::login
```

然后扩展：

```text
Called by:
- login_handler
- cli_login_command

Calls:
- UserRepository::find_by_email
- PasswordHasher::verify
- TokenService::issue

Tested by:
- tests/auth_login_test.rs
```

这比向量检索更稳定。

Tree-sitter 的 code navigation 文档里也提到可以用 query language 标记 definition、reference、name 等可命名实体，用来构建代码导航能力。([Tree-sitter][6]) 你现在的 symbol index 就可以继续往这个方向扩展。

---

## 4. Vector Retriever

最后才是向量检索。

它负责语义模糊召回。

适合处理：

```text
“支付超时重试逻辑在哪里”
“用户注册校验规则”
“缓存失效策略”
“权限检查入口”
“数据库连接池初始化”
“哪个模块负责把领域错误转 HTTP response”
```

这些用户问题不一定包含真实符号名，vector search 可以补上召回率。

Azure AI Search 的 hybrid search 文档也把 hybrid search 描述为同时执行 full-text 和 vector 查询，并用 Reciprocal Rank Fusion 合并结果；它强调 vector 擅长概念相似，full-text 擅长精确匹配。([Microsoft Learn][7]) 这个思路非常适合代码库检索。

---

# 5. 所以，要不要做向量数据库？

**要做，但不要第一个做，也不要只做它。**

比较合理的判断标准是：

## 你暂时不需要独立向量数据库，如果：

```text
单机工具
单个 repo 或少量 repo
代码量 < 几百万行
主要是本地 CLI / IDE plugin
已经有 SQLite 存 symbol index
没有多用户并发
没有复杂权限隔离
```

这种情况下，建议：

```text
SQLite + FTS5 + sqlite-vec
```

或者：

```text
SQLite + FTS5 + LanceDB embedded
```

sqlite-vec 是一个小型 SQLite 向量搜索扩展，官方描述是 “fast enough” 且可嵌入。([GitHub][8]) LanceDB 的开源版可以作为 in-process embedded database 使用，并且有 Rust SDK。([LanceDB][9])

## 你需要独立向量数据库，如果：

```text
很多 repo
团队多人使用
需要服务端部署
需要权限隔离
需要高并发
需要跨项目搜索
需要持续增量索引
需要 metadata filter 很强
```

这时可以考虑：

```text
Qdrant
LanceDB server / cloud
Milvus
Weaviate
Elasticsearch vector
Azure AI Search
```

Qdrant 官方定位是 vector search / semantic search engine，也提供 Rust client；它比较适合你这种 Rust 技术栈。([Qdrant][10])

我的建议：

```text
MVP：SQLite + FTS5 + symbol tables
增强：加 sqlite-vec 或 LanceDB embedded
生产/多用户：Qdrant 或 LanceDB
```

---

# 6. 你应该 embed 什么？

不要把整个文件按固定长度切块后直接 embed。

代码库不适合粗暴这样切：

```text
每 500 tokens 切一块
overlap 100 tokens
全部 embedding
```

更好的 chunk 单位是：

```text
symbol chunk
module chunk
test chunk
doc chunk
error chunk
config chunk
```

## 推荐 chunk 类型

### 1. Symbol chunk

每个重要符号一个 chunk：

```text
AuthService::login
UserRepository::find_by_email
AppError
LoginRequest
```

内容可以是：

```text
file: src/service/auth_service.rs
symbol: crate::service::auth_service::AuthService::login
kind: method
signature:
pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError>

doc:
Authenticates user credentials and returns token.

body_summary:
Finds user by email, verifies password hash, issues JWT token.

related:
- UserRepository::find_by_email
- PasswordHasher::verify
- TokenService::issue
```

embedding 不一定要 embed 完整函数体。
很多时候 embed **签名 + 文档 + summary + 关键调用** 效果更好。

---

### 2. File/module chunk

适合 repo map：

```text
src/service/auth_service.rs
Role: authentication business logic
Defines:
- AuthService
- AuthService::login
- AuthService::refresh_token
Depends on:
- UserRepository
- PasswordHasher
- TokenService
```

这种 chunk 用来回答：

```text
认证逻辑在哪里？
session 相关代码在哪？
哪个模块负责用户注册？
```

---

### 3. Test chunk

测试非常重要。

你应该单独索引：

```text
#[test]
#[tokio::test]
mod tests
integration tests
snapshot tests
```

测试 chunk 内容：

```text
test: rejects_invalid_password
target_symbol: AuthService::login
file: tests/auth_login_test.rs
asserts:
- wrong password returns Unauthorized
- token is not issued
```

很多修改任务里，AI 最需要的不是所有业务代码，而是相关测试。

---

### 4. Error / string chunk

把这些也索引进去：

```text
错误字符串
HTTP route
SQL query
GraphQL query
config key
env var
feature flag
log message
metric name
```

例如：

```rust
"duplicate email"
"JWT_SECRET"
"/api/v1/login"
"password_mismatch"
```

这类内容经常是用户报错时唯一给你的线索。

---

# 7. Chunk metadata 很关键

你的向量库里不要只存：

```text
embedding
content
```

一定要存 metadata：

```json
{
  "chunk_id": "chunk_123",
  "chunk_type": "symbol",
  "symbol_id": "sym_auth_service_login",
  "qualified_name": "crate::service::auth_service::AuthService::login",
  "kind": "method",
  "file_path": "src/service/auth_service.rs",
  "start_line": 42,
  "end_line": 88,
  "language": "rust",
  "crate": "app",
  "module_path": "crate::service::auth_service",
  "visibility": "pub",
  "hash": "abc123",
  "last_modified": 1712345678
}
```

因为最终你不是只要“搜到文本”，而是要继续做：

```text
打开文件
读取函数体
扩展 caller/callee
找到测试
打包上下文
生成 patch
```

metadata 是连接 vector search 和 symbol index 的桥。

---

# 8. 排序层比向量库更重要

召回只是第一步。

你会得到很多候选：

```text
symbol candidates
text candidates
graph candidates
vector candidates
recently edited files
test failure files
```

下一步要做一个 ranker。

可以先用简单打分：

```text
final_score =
  0.35 * symbol_score
+ 0.25 * text_score
+ 0.20 * graph_score
+ 0.15 * vector_score
+ 0.05 * recency_score
```

不同任务可以调权重。

## 解释类任务

例如：

```text
解释这个项目的认证流程
```

权重可以偏向：

```text
repo map
symbol graph
vector semantic
docs
```

## 修改类任务

例如：

```text
修改 AuthService::login
```

权重应该偏向：

```text
exact symbol
callers
callees
tests
error types
```

## 报错类任务

例如：

```text
这个错误 duplicate key 是哪里来的？
```

权重应该偏向：

```text
全文搜索
错误字符串
SQL
日志
tests
```

## 重构类任务

例如：

```text
把 UserRepository::find_by_email 改成 find_by_identifier
```

权重应该偏向：

```text
references
implementations
callers
tests
public API boundary
```

---

# 9. 上下文打包 Context Packing 是下一步的核心

检索到候选以后，不能直接全塞给 AI。

你要做一个 context packer，根据 token budget 组装：

```text
1. task
2. repo map slice
3. relevant symbols
4. exact code snippets
5. dependency/caller/callee relation
6. tests
7. constraints
```

推荐格式：

````md
<code_context>

# Task
用户想修改登录逻辑：密码错误时返回 401，而不是 500。

# Relevant Repo Map
- src/api/auth_handler.rs: HTTP login endpoint
- src/service/auth_service.rs: authentication business logic
- src/error.rs: AppError -> HTTP status mapping
- tests/auth_login_test.rs: login behavior tests

# Symbols

## crate::service::auth_service::AuthService::login
File: src/service/auth_service.rs:42-88
```rust
pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError>
````

Calls:

* UserRepository::find_by_email
* PasswordHasher::verify
* TokenService::issue

## crate::error::AppError

File: src/error.rs:1-80

```rust
pub enum AppError {
    Unauthorized,
    Internal(anyhow::Error),
}
```

# Code Snippets

## src/service/auth_service.rs:42-88

```rust
// actual relevant code here
```

## src/error.rs:34-60

```rust
// error mapping here
```

# Tests

## tests/auth_login_test.rs:15-50

```rust
// related test here
```

</code_context>

````

这一步非常重要。  
很多 AI coding 系统的差异不在于“有没有索引”，而在于 **最后给模型的上下文是否干净、短、准确、可操作**。

---

# 10. 你现在最应该补的数据库表

你已经有 repo map 和 symbol index。下一步加这几张表。

## chunks 表

```sql
CREATE TABLE chunks (
    id TEXT PRIMARY KEY,
    chunk_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    symbol_id TEXT,
    qualified_name TEXT,
    start_line INTEGER,
    end_line INTEGER,
    content TEXT NOT NULL,
    summary TEXT,
    hash TEXT NOT NULL,
    token_count INTEGER,
    updated_at INTEGER
);
````

## full_text_index

SQLite FTS5 版本：

```sql
CREATE VIRTUAL TABLE chunks_fts USING fts5(
    content,
    summary,
    qualified_name,
    file_path,
    content='chunks',
    content_rowid='rowid'
);
```

## embeddings 表

```sql
CREATE TABLE embeddings (
    chunk_id TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    created_at INTEGER NOT NULL
);
```

如果用 LanceDB 或 Qdrant，就把 `chunk_id` 存成外部向量库的 payload key。

## retrieval_runs 表

用来调试和评测：

```sql
CREATE TABLE retrieval_runs (
    id TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    selected_chunks_json TEXT NOT NULL,
    scores_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

这个表很有用。你可以回放一次检索，看看为什么某个错误上下文被选进来了。

---

# 11. Rust 实现建议

你的 Rust 项目可以拆成这些模块：

```text
repo_context/
├── scanner/
├── parser/
├── symbol_index/
├── relation_index/
├── chunker/
├── fts/
├── embedding/
├── vector_store/
├── retriever/
├── ranker/
├── context_packer/
├── eval/
└── server/
```

推荐技术组合：

```text
SQLite: rusqlite / sqlx
全文搜索: SQLite FTS5 / Tantivy
向量搜索: sqlite-vec / LanceDB / Qdrant
embedding: fastembed / ONNX Runtime / 外部 API
token 统计: tokenizers / tiktoken-compatible crate
CLI: clap
服务接口: axum
MCP: rmcp / 官方 SDK 生态
```

FastEmbed 现在有 Rust crate，定位是本地生成 retrieval embeddings 和 reranking。([文档.rs][11]) Hugging Face 的 Candle 是 Rust ML framework，适合更深度地做本地模型推理。([GitHub][12])

---

# 12. 你可以先实现这个 MVP

我建议你下一版不要做太大，先实现：

```text
输入：用户 query
输出：一个 code_context.md
```

流程：

```text
1. query 进入系统
2. symbol index 精确查找
3. SQLite FTS5 查全文
4. relation graph 扩展一跳
5. 可选 vector search
6. 合并候选
7. 排序
8. 按 token budget 打包 Markdown
```

伪代码：

```rust
pub async fn build_context(query: &str, budget_tokens: usize) -> Result<CodeContext> {
    let symbol_hits = symbol_retriever.search(query).await?;
    let text_hits = fts_retriever.search(query).await?;
    let vector_hits = vector_retriever.search(query).await.unwrap_or_default();

    let mut candidates = CandidateSet::new();
    candidates.add(symbol_hits, Source::Symbol);
    candidates.add(text_hits, Source::FullText);
    candidates.add(vector_hits, Source::Vector);

    let expanded = graph_expander.expand(candidates, GraphPolicy {
        include_callers: true,
        include_callees: true,
        include_tests: true,
        max_depth: 1,
    }).await?;

    let ranked = ranker.rank(query, expanded).await?;

    let packed = context_packer.pack(ranked, PackOptions {
        token_budget: budget_tokens,
        include_repo_map_slice: true,
        include_signatures: true,
        include_code_snippets: true,
        include_tests: true,
    }).await?;

    Ok(packed)
}
```

最终输出：

```bash
repoctx search "密码错误为什么返回 500" --budget 12000 > code_context.md
```

然后你把 `code_context.md` 给 AI。

---

# 13. Evaluation 也要尽早做

不要等系统很复杂才评测。

你可以准备一组真实问题：

```json
[
  {
    "query": "登录失败为什么返回 500？",
    "must_include": [
      "src/service/auth_service.rs",
      "src/error.rs",
      "tests/auth_login_test.rs"
    ]
  },
  {
    "query": "用户注册邮箱重复在哪里处理？",
    "must_include": [
      "src/service/user_service.rs",
      "src/repository/user_repository.rs",
      "src/error.rs"
    ]
  },
  {
    "query": "JWT token 是在哪里生成的？",
    "must_include": [
      "src/service/token_service.rs",
      "src/api/auth_handler.rs"
    ]
  }
]
```

然后每次改检索策略，都跑：

```text
Recall@5
Recall@10
MRR
平均 token 数
无关上下文比例
是否包含测试
是否包含错误映射
```

否则你加了 vector DB 以后，可能只是“结果看起来更智能”，但实际改代码成功率没有提升。

---

# 14. 做 Agent 前，先做 Tool API

当你的 retrieval/context engine 稳了以后，就可以把它暴露成工具。

比如：

```text
search_symbols(query)
search_code(query)
get_symbol(qualified_name)
get_references(symbol)
get_implementations(trait_or_method)
get_callers(symbol)
get_callees(symbol)
get_tests(symbol)
build_context(query, budget)
```

这时你可以做：

```text
CLI
HTTP API
IDE plugin
MCP server
```

MCP 是一个连接 AI 应用和外部系统的开放标准，官方介绍里把它描述为让 AI 应用连接本地文件、数据库、搜索工具、工作流等外部能力的协议。([模型上下文协议][13]) 对你的项目来说，MCP 很适合把 repo map、symbol index、retrieval engine 暴露给 Claude Code、Cursor、ChatGPT Desktop 类工具。

但注意顺序：

```text
先有高质量 retrieval
再做 MCP
```

不要反过来。

---

# 15. 最推荐的路线图

你现在的下一步可以这样排：

## 阶段 1：检索闭环

```text
symbol search
+ full-text search
+ graph expansion
+ context packing
```

目标：

```text
不用向量，也能根据 query 生成一份不错的 code_context.md
```

---

## 阶段 2：加 vector search

```text
给 symbol chunk / module chunk / test chunk 做 embedding
接 sqlite-vec / LanceDB / Qdrant
合并 vector hits
```

目标：

```text
用户不知道函数名时，也能找到相关模块
```

---

## 阶段 3：加 reranking

```text
粗召回 top 100
rerank 到 top 20
再打包进上下文
```

目标：

```text
减少无关代码进入上下文
```

---

## 阶段 4：加 evaluation

```text
准备 50-200 个真实 query
记录 must_include files/symbols
比较不同策略
```

目标：

```text
证明 vector / graph / reranker 真的提升效果
```

---

## 阶段 5：Agent 工具化

```text
CLI
HTTP API
MCP server
IDE extension
```

目标：

```text
让 AI 可以按需调用你的 repo context engine
```

---

# 16. 最终答案

**是的，下一步是 RAG，但不是普通文档 RAG，而是 Code RAG。**

更准确地说，你要做：

```text
Hybrid Code RAG = Symbol Index + Full-text Search + Graph Expansion + Vector Search + Reranking + Context Packing
```

向量数据库的位置是这里：

```text
repo map
symbol index
relation graph
full-text search
        ↓
vector search 作为语义召回补充
        ↓
rerank
        ↓
context packer
        ↓
AI
```

我会按这个顺序做：

```text
第一步：SQLite FTS5 / Tantivy 全文搜索
第二步：基于 symbol relation 的 graph expansion
第三步：context packer，把结果打包成 Markdown/XML
第四步：evaluation，用真实 query 测召回质量
第五步：再加 vector DB
第六步：做 MCP / agent tool
```

一句话：

> **repo map 和 symbol index 解决“代码在哪里、叫什么、关系是什么”；下一步的 Code RAG 要解决“当前任务最需要哪些上下文”。**

[1]: https://developers.openai.com/api/docs/guides/retrieval?utm_source=chatgpt.com "Retrieval | OpenAI API"
[2]: https://aider.chat/docs/repomap.html?utm_source=chatgpt.com "Repository map"
[3]: https://github.com/scip-code/scip?utm_source=chatgpt.com "SCIP Code Intelligence Protocol"
[4]: https://sqlite.org/fts5.html?utm_source=chatgpt.com "SQLite FTS5 Extension"
[5]: https://github.com/quickwit-oss/tantivy?utm_source=chatgpt.com "Tantivy is a full-text search engine library inspired ..."
[6]: https://tree-sitter.github.io/tree-sitter/4-code-navigation.html?utm_source=chatgpt.com "Code Navigation Systems"
[7]: https://learn.microsoft.com/en-us/azure/search/hybrid-search-overview?utm_source=chatgpt.com "Hybrid Search Overview - Azure AI Search"
[8]: https://github.com/asg017/sqlite-vec?utm_source=chatgpt.com "asg017/sqlite-vec: A vector search ..."
[9]: https://docs.lancedb.com/quickstart?utm_source=chatgpt.com "Quickstart"
[10]: https://qdrant.tech/documentation/?utm_source=chatgpt.com "Qdrant Documentation"
[11]: https://docs.rs/fastembed?utm_source=chatgpt.com "fastembed - Rust"
[12]: https://github.com/huggingface/candle?utm_source=chatgpt.com "huggingface/candle: Minimalist ML framework for Rust"
[13]: https://modelcontextprotocol.io/docs/getting-started/intro?utm_source=chatgpt.com "Model Context Protocol"


````md
## Context Packer

Context Packer，也可以叫“上下文打包器”，是 Code RAG 系统里的最后一层。它的职责不是继续搜索代码，而是把前面检索到的 repo map、符号索引、代码片段、调用关系、测试文件和错误信息，整理成一份适合 AI 阅读的上下文。

它要解决的问题是：在有限 token budget 内，哪些内容应该优先放进去，哪些内容只保留摘要，哪些内容应该丢弃。

一个好的 Context Packer 需要做到三点：

1. 保留任务最相关的代码；
2. 保留 AI 修改代码时必须知道的接口、类型、错误、测试和调用关系；
3. 删除重复、低价值、噪声大的内容。

它的输入通常是多个检索器返回的候选结果：

- symbol search 命中的函数、类型、trait、method；
- full-text search 命中的错误字符串、注释、配置项；
- graph expansion 找到的 caller、callee、impl、test；
- vector search 找到的语义相关代码块；
- repo map 中相关的模块概览。

它的输出则是一份结构化的 Markdown/XML 上下文，例如：

```md
<code_context>

# Task
用户想修改登录逻辑：密码错误时返回 401，而不是 500。

# Relevant Files
- src/api/auth_handler.rs
- src/service/auth_service.rs
- src/error.rs
- tests/auth_login_test.rs

# Relevant Symbols

## AuthService::login
File: src/service/auth_service.rs:42-88

```rust
pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError>
````

Calls:

* UserRepository::find_by_email
* PasswordHasher::verify
* TokenService::issue

# Code Snippets

## src/service/auth_service.rs:42-88

```rust
// 相关代码片段
```

## src/error.rs:30-65

```rust
// 错误类型到 HTTP status 的映射
```

# Tests

## tests/auth_login_test.rs

```rust
// 登录失败相关测试
```

</code_context>

```

Context Packer 是 repo map、符号索引和 RAG 真正落到 AI 编程体验里的关键步骤。检索系统负责“找得到”，Context Packer 负责“给得好”。

如果没有 Context Packer，系统可能会把一堆相关但混乱的代码直接塞给 AI，导致上下文重复、token 浪费、重点不清晰。  
有了 Context Packer，AI 看到的是经过压缩、排序和组织后的任务上下文，更容易准确理解代码并完成修改。
```

---

# 2026-06-24：RAG / embedding 产品化落地约定

本轮已经把远程 embedding 的“能跑”补成可观测、可评测、可追责的产品层。实现位置集中在 `server/src/context_compiler`：

- `symbol_index_product.rs`：产品控制面 schema 和记录函数。
- `symbol_index_embeddings.rs`：embedding status 查询时同步返回项目索引状态、队列、成本、评测集摘要。
- `symbol_index_vector.rs`：向量 backfill 记录 job、模型、token、估算成本、远程失败原因。
- `symbol_index_eval_runs.rs`：`eval-batch` 自动沉淀真实任务检索评测集。

## 项目索引状态

`/api/admin/context/symbol-index/embedding-status` 返回 `projectStatus`，状态只允许四类：

| status | 含义 |
|---|---|
| `unindexed` | 当前没有 chunk，说明还没跑 context compiler 或没有可索引内容。 |
| `indexed` | chunk 和当前 embedding 模型均可用。 |
| `embedding_missing` | 有 chunk，但部分 chunk 缺少当前模型 embedding。 |
| `needs_rebuild` | 有 embedding，但 content hash 已和当前 chunk 不一致，需要重建。 |

这个状态面向产品和 Agent 决策：AI 不应该盲目认为“能查”；它应先看状态，遇到 `embedding_missing` 或 `needs_rebuild` 时主动回填或降级到 symbol/full-text/graph 检索。

## 后台索引队列

当前 backfill 仍是同步执行，但每次调用都会写入 `embedding_index_jobs`：

- `running / succeeded / failed` 状态；
- `trace_id`、`model`、`limit_count`、`force`；
- `scanned_count`、`upserted_count`、`skipped_count`；
- `failure_reason`、`created_at`、`started_at`、`finished_at`。

这是一层最小可用的后台队列契约。后续如果要做真正异步 worker，不需要重定义产品口径，只要从这张表扩展 `queued -> running -> succeeded/failed` 消费流程即可。

## embedding 成本和模型记录

每个写入的 embedding 会记录到 `embedding_usage_events`：

- `model` 和 `provider`；
- chunk 级 `input_token_count`；
- `estimated_cost_micro_usd`；
- `job_id` 和 `chunk_id`。

估算成本用于产品观测，不作为计费事实来源。当前按模型名粗估：`text-embedding-3-small` 约 20 micro USD / 1K tokens，`text-embedding-3-large` 约 130 micro USD / 1K tokens，其它远程模型按 100 micro USD / 1K tokens，`local-hash-v1` 为 0。

## 远程 embedding 失败原因

远程模型解析、provider 配置缺失、HTTP 调用失败、响应解析失败都会记录到 `remote_embedding_failures`。状态接口会带出最近失败原因，避免产品层只看到“没 embedding”，却不知道是 API key、网络、限流还是服务端返回异常。

## 真实任务检索评测集

`/api/admin/context/symbol-index/eval-batch` 记录 run 时，会同步把每个 case 写入 `retrieval_eval_cases`：

- `query`、`must_include_json`；
- 来源固定为 `real_task`；
- 最近一次 `run_id`、`recall_at_k`、缺失 requirement 数量。

这让评测不再停留在临时 JSON。后续调 ranker、vector 策略、graph 扩展时，应先积累真实任务 case，再用 `eval-runs`、`eval-compare` 和 `retrieval-learning` 看召回质量、噪声率、测试上下文覆盖率和 token 成本。
