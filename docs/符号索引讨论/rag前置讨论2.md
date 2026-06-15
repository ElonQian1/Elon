如果 **Query Router / Retrieval Planner** 已经做了，那么下一步就可以正式做：

# Vector Retriever / 语义召回层

也就是把向量检索接进你的 Code RAG，但它只能作为 **一个 retriever**，不要让它变成主系统。

你的系统现在应该从：

```text
symbol search
full-text search
graph expansion
repo map retrieval
context packing
```

升级成：

```text
symbol search
full-text search
graph expansion
repo map retrieval
vector search   ⬅️ 下一步加这个
rerank
context packing
```

---

# 现在为什么可以做向量检索了？

因为你前面已经做了：

```text
repo map
symbol index
chunks
build-context
eval
debug / explain
query router
```

这意味着你现在有能力控制向量检索：

```text
什么时候用 vector
vector 召回多少
vector 权重多高
vector 召回的东西为什么进上下文
vector 有没有真的提升 eval
```

没有这些基础，向量库很容易变成噪声源。
有了 Query Router 之后，向量检索才是可控增强。

---

# 向量检索解决什么问题？

它主要解决这类问题：

```text
用户不知道真实函数名
用户用自然语言描述功能
代码里没有完全相同的关键词
想找“相似实现”
想找项目里类似的设计模式
```

例如用户问：

```text
权限校验是怎么做的？
```

代码里可能没有“权限校验”这四个字，而是：

```rust
authorize()
check_role()
PolicyGuard
PermissionService
AccessControl
```

全文搜索可能搜不到，符号搜索也不一定命中。
这时 vector search 可以根据语义找到相关 chunk。

---

# 但它不应该解决什么？

向量检索不应该替代这些东西：

```text
精确函数名搜索
trait implementation 搜索
find references
caller / callee
错误字符串
HTTP status code
SQL 错误
配置 key
测试文件关系
```

这些仍然应该交给：

```text
symbol index
full-text search
relation graph
```

所以你的原则应该是：

> **symbol / full-text / graph 负责精确性，vector 负责语义召回率。**

---

# 你下一步要实现的最小目标

先做这个命令：

```bash
repoctx embed .
```

它负责：

```text
读取 chunks
        ↓
生成 embedding
        ↓
写入 vector store
        ↓
记录 embedding model / dim / chunk hash
```

然后做：

```bash
repoctx search-vector "权限校验是怎么做的？"
```

输出：

```text
1. src/auth/permission.rs
   chunk: PermissionService::check
   score: 0.83

2. src/middleware/auth_guard.rs
   chunk: AuthGuard::authorize
   score: 0.79

3. src/api/admin.rs
   chunk: admin route permission check
   score: 0.73
```

最后把它接入：

```bash
repoctx build-context "权限校验是怎么做的？" --with-vector
```

---

# 你应该 embed 什么？

不要把整个文件粗暴切成固定长度后全部 embedding。

你已经有 symbol index 和 chunks，所以应该优先 embed 这些：

```text
symbol chunk
module summary chunk
test chunk
route chunk
error chunk
config chunk
```

## 1. Symbol chunk

每个重要符号一个向量。

例如：

```text
chunk_type: symbol
symbol: crate::service::auth_service::AuthService::login
file: src/service/auth_service.rs
signature:
pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError>

summary:
Authenticates a user by email and password, verifies the password hash, and issues an access token.

relations:
calls UserRepository::find_by_email
calls PasswordHasher::verify
calls TokenService::issue
returns AppError
```

注意：**embedding 内容不一定等于完整函数体**。
对语义检索来说，很多时候下面这种内容更好：

```text
文件路径
符号名
函数签名
doc comment
AI summary
关键调用关系
错误类型
测试关系
```

因为用户通常用自然语言找功能，而不是用完整源码片段找功能。

---

## 2. Module summary chunk

每个文件或模块一个摘要向量。

例如：

```text
chunk_type: module
file: src/service/auth_service.rs
summary:
Authentication business logic. Handles login, password verification, token issuing, and refresh token validation.

defines:
- AuthService::login
- AuthService::refresh_token
- AuthService::logout

depends_on:
- UserRepository
- PasswordHasher
- TokenService
```

这种 chunk 适合回答：

```text
认证流程在哪里？
用户注册逻辑在哪里？
错误处理机制在哪？
```

---

## 3. Test chunk

测试必须单独 embed。

例如：

```text
chunk_type: test
file: tests/auth_login_test.rs
test: rejects_wrong_password
summary:
Verifies that login with a wrong password returns Unauthorized and does not issue a token.

target_symbols:
- AuthService::login
- PasswordHasher::verify
```

AI 修改代码时，相关测试往往比普通业务代码更重要。

---

## 4. Error / route / config chunk

这些也很值得向量化：

```text
HTTP route
错误类型
错误字符串
日志字符串
配置 key
环境变量
SQL query
feature flag
metric name
```

例如：

```text
route: POST /api/login
handler: login_handler
service: AuthService::login
errors:
- Unauthorized
- InvalidCredentials
- InternalServerError
```

这对 debug / modify 类任务很有帮助。

---

# 你需要新增的数据结构

你已有 `chunks` 表的话，可以加一张 `embeddings` 表。

```sql
CREATE TABLE embeddings (
    chunk_id TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    dim INTEGER NOT NULL,
    vector BLOB NOT NULL,
    chunk_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

如果用外部向量库，比如 Qdrant 或 LanceDB，就在外部 vector store 里存向量，然后本地数据库保存映射关系：

```sql
CREATE TABLE vector_refs (
    chunk_id TEXT PRIMARY KEY,
    store TEXT NOT NULL,
    collection TEXT NOT NULL,
    external_id TEXT NOT NULL,
    model TEXT NOT NULL,
    dim INTEGER NOT NULL,
    chunk_hash TEXT NOT NULL
);
```

你一定要记录：

```text
chunk_hash
model
dim
created_at
```

因为代码变了以后，需要知道哪些 chunk 的 embedding 过期了。

---

# 选什么向量库？

我建议按项目阶段选。

## 本地 MVP

用：

```text
SQLite + sqlite-vec
```

适合：

```text
单机
单 repo
CLI 工具
不想部署服务
数据量不大
```

`sqlite-vec` 是一个很小的 SQLite 向量搜索扩展，官方描述是 “fast enough” 并且可以在很多环境中运行；不过它目前仍然是 pre-v1，后续可能有破坏性变化。([GitHub][1])

---

## 本地增强版

用：

```text
LanceDB
```

适合：

```text
本地持久化
更像正经 vector database
需要管理 embedding 数据
以后可能扩到更大数据
```

LanceDB 的 Rust crate 文档把它描述为开源 vector-search 数据库，带持久化存储，简化 embedding 的检索、过滤和管理。([docs.rs][2])

---

## 服务端 / 多 repo / 团队版

用：

```text
Qdrant
```

适合：

```text
多个 repo
团队使用
高并发
metadata filter
服务端部署
后续做 hybrid dense/sparse retrieval
```

Qdrant 官方定位是 vector search / semantic search engine，并强调 payload filtering；它也支持 dense、sparse、multivector 等 hybrid query 组合方式。([GitHub][3])

---

# Embedding 模型怎么选？

Rust 本地优先可以试：

```text
fastembed
```

`fastembed` 的 Rust crate 文档说明它用于 retrieval embedding generation，支持本地 ONNX 推理，模型下载一次后可以离线运行。([docs.rs][4])

第一版不要纠结模型排行榜，先选一个稳定的 embedding 模型，把 pipeline 跑通。
你的重点不是“哪个模型最高分”，而是：

```text
chunk 设计是否合理
metadata 是否完整
hybrid fusion 是否稳定
eval 是否真的提升
```

后面再替换模型做对比。

---

# Vector Retriever 的 Rust 接口

可以设计成这样：

```rust
pub struct VectorQuery {
    pub query: String,
    pub top_k: usize,
    pub filters: VectorFilters,
}

pub struct VectorFilters {
    pub language: Option<String>,
    pub chunk_types: Vec<ChunkType>,
    pub file_prefixes: Vec<String>,
    pub exclude_tests: bool,
    pub only_tests: bool,
}

pub struct VectorHit {
    pub chunk_id: String,
    pub score: f32,
    pub reason: String,
}
```

接口：

```rust
#[async_trait::async_trait]
pub trait VectorStore {
    async fn upsert(&self, chunks: Vec<EmbeddedChunk>) -> anyhow::Result<()>;

    async fn search(&self, query: VectorQuery) -> anyhow::Result<Vec<VectorHit>>;

    async fn delete_stale(&self, stale_chunk_ids: &[String]) -> anyhow::Result<()>;
}
```

然后你可以有不同实现：

```text
SqliteVecStore
LanceDbStore
QdrantStore
```

这样以后换向量库不会影响上层系统。

---

# 接入 Query Router

你的 Query Router 现在应该控制 vector 的启用和权重。

例如：

## Locate 类

```text
“AuthService::login 在哪里？”
```

策略：

```json
{
  "vector": false,
  "symbol_weight": 0.65,
  "full_text_weight": 0.20,
  "graph_weight": 0.15
}
```

因为这是精确符号问题，不需要向量。

---

## Explain 类

```text
“认证流程是怎么走的？”
```

策略：

```json
{
  "vector": true,
  "symbol_weight": 0.20,
  "full_text_weight": 0.15,
  "graph_weight": 0.30,
  "vector_weight": 0.30,
  "repo_map_weight": 0.05
}
```

解释类任务适合用 vector 找概念相关模块。

---

## Add Feature 类

```text
“新增邮箱验证码登录”
```

策略：

```json
{
  "vector": true,
  "symbol_weight": 0.15,
  "full_text_weight": 0.20,
  "graph_weight": 0.25,
  "vector_weight": 0.35,
  "repo_map_weight": 0.05
}
```

新增功能类任务很适合找“相似功能”和“已有代码风格”。

---

## Debug Error 类

```text
“登录失败为什么返回 500？”
```

策略：

```json
{
  "vector": false,
  "symbol_weight": 0.25,
  "full_text_weight": 0.40,
  "graph_weight": 0.30,
  "repo_map_weight": 0.05
}
```

报错类通常优先用全文搜索、错误字符串、调用图、测试。
vector 可以开，但权重不要太高。

---

# Hybrid fusion 怎么做？

当你有这些结果：

```text
symbol hits
full-text hits
graph hits
vector hits
repo map hits
```

不要简单把分数相加，因为不同 retriever 的分数尺度不一样。

第一版可以用 **RRF，Reciprocal Rank Fusion**。
Azure AI Search 文档把 RRF 描述为把多个已经排序的结果集合并成一个统一结果集的算法，常用于 hybrid query；Qdrant 的 hybrid query 文档也提到可以用 RRF 或 DBSF 融合 dense、sparse、multivector 结果。([Microsoft Learn][5])

RRF 简化版：

```rust
fn rrf_score(rank: usize, k: f32) -> f32 {
    1.0 / (k + rank as f32)
}
```

融合：

```rust
final_score =
    symbol_weight * rrf(symbol_rank)
  + fts_weight    * rrf(fts_rank)
  + graph_weight  * rrf(graph_rank)
  + vector_weight * rrf(vector_rank)
```

这样比直接混合 raw score 稳定很多。

---

# 你的 debug trace 要加 vector 信息

做完 vector retriever 后，`repoctx debug` 必须显示：

```text
这个 chunk 是不是 vector 找到的？
vector score 是多少？
query embedding 用的哪个模型？
有没有 metadata filter？
最后是因为 vector 进来的，还是因为 symbol/graph 进来的？
```

例如：

```text
Candidate:
src/middleware/auth_guard.rs:AuthGuard::authorize

Found by:
- vector_search: score 0.81
  reason: semantically similar to "权限校验"
  model: jinaai/jina-embeddings-v2-base-code
- repo_map: score 0.44
  reason: module summary mentions access control

Final:
selected: yes
final_score: 0.72
```

这很重要。
否则 vector 检索一旦引入噪声，你很难定位问题。

---

# eval 怎么验证 vector 是否有用？

你要新增一组专门测试 vector 的 cases。

这些 query 应该故意不使用真实符号名：

```json
[
  {
    "id": "permission_flow_natural_language",
    "intent": "explain",
    "query": "权限校验是怎么做的？",
    "must_include_files": [
      "src/middleware/auth_guard.rs",
      "src/service/permission_service.rs"
    ]
  },
  {
    "id": "similar_feature_refresh_token",
    "intent": "add_feature",
    "query": "我想新增邮箱验证码登录，参考现有登录流程",
    "must_include_files": [
      "src/service/auth_service.rs",
      "src/api/auth_handler.rs",
      "tests/auth_login_test.rs"
    ]
  },
  {
    "id": "cache_invalidation",
    "intent": "locate",
    "query": "缓存失效策略在哪里？",
    "must_include_files": [
      "src/cache/mod.rs",
      "src/service/cache_invalidator.rs"
    ]
  }
]
```

然后对比：

```bash
repoctx eval eval_cases.json --no-vector
repoctx eval eval_cases.json --with-vector
repoctx compare runs/no_vector.json runs/with_vector.json
```

你要看：

```text
File Recall 有没有提升
Symbol Recall 有没有提升
Avg Tokens 有没有暴涨
Noise Ratio 有没有变高
Debug / Modify 类任务有没有被 vector 伤害
```

如果 vector 只提升了 Explain / AddFeature，但伤害了 Debug / Refactor，那就让 Query Router 只在特定 intent 启用 vector。

---

# 你现在的具体任务清单

按这个顺序做：

```text
1. 定义 EmbeddingInput
2. 为 chunk 生成 embedding_text
3. 实现 repoctx embed .
4. 接一个本地 embedding 模型
5. 选一个 vector store
6. 实现 VectorRetriever
7. 接入 Query Router
8. 用 RRF 融合 vector hits
9. debug trace 显示 vector 来源
10. eval 对比 with-vector / no-vector
```

---

# 推荐最小实现

第一版可以这样：

```text
Embedding:
fastembed

Vector store:
SQLite + sqlite-vec 或 LanceDB

Embedding 对象:
只 embed symbol chunk + module summary chunk + test chunk

Query Router:
只在 Explain / AddFeature / Unknown 中启用 vector

Fusion:
RRF

Eval:
必须对比 no-vector vs with-vector
```

命令：

```bash
repoctx embed .
repoctx search-vector "权限校验是怎么做的？"
repoctx build-context "权限校验是怎么做的？" --with-vector --show-plan
repoctx eval eval_cases.json --with-vector
```

---

# 做完这一步后，下一步是什么？

做完 Vector Retriever 之后，下一步才是：

```text
Reranker
```

因为那时你的系统会有很多候选：

```text
symbol top 20
fts top 20
graph top 20
vector top 20
repo map top 10
```

这时需要一个 reranker 从 top 100 里选 top 20，再交给 Context Packer。

但现在不要先做 reranker。
先把 vector retriever 接进来，并用 eval 证明它确实提高了自然语言查询的召回率。

---

# 一句话

现在下一步是：

> **实现 Vector Retriever，把语义召回接入你的 Hybrid Code RAG。**

但要记住它的位置：

```text
Query Router
    ↓
Symbol / FTS / Graph / Vector
    ↓
RRF fusion
    ↓
Debug trace
    ↓
Eval comparison
    ↓
Context Packer
```

向量检索不是核心大脑，它是补充召回率的一个工具。真正的核心仍然是：

```text
symbol index + relation graph + query router + context packer + evaluation
```

[1]: https://github.com/asg017/sqlite-vec?utm_source=chatgpt.com "asg017/sqlite-vec: A vector search ..."
[2]: https://docs.rs/lancedb?utm_source=chatgpt.com "lancedb - Rust"
[3]: https://github.com/qdrant/qdrant?utm_source=chatgpt.com "GitHub - qdrant/qdrant: Qdrant - High-performance ..."
[4]: https://docs.rs/fastembed?utm_source=chatgpt.com "fastembed - Rust"
[5]: https://learn.microsoft.com/en-us/azure/search/hybrid-search-ranking?utm_source=chatgpt.com "Hybrid Search Scoring (RRF) - Azure AI Search"
下一步做：

# Reranker / 候选重排序器

你现在已经有：

```text
repo map
symbol index
chunks
build-context
eval
debug / explain / compare
query router
vector retriever
```

那么下一步要解决的问题是：

> 检索器找回来了很多候选代码块，但哪些最值得放进 AI 上下文？

所以现在要做 **Reranker**。

---

## 为什么下一步是 Reranker？

因为你的系统现在会有很多来源：

```text
symbol search top 20
full-text search top 20
graph expansion top 30
repo map retrieval top 10
vector search top 20
recent files top 5
```

合起来可能有几十到上百个候选 chunk。

但最后给 AI 的上下文可能只能放：

```text
10 个文件
20 个符号
若干代码片段
12000 tokens
```

所以你需要一个模块判断：

```text
哪些必须放？
哪些可以摘要？
哪些应该丢掉？
哪些虽然相关，但重复？
哪些对当前任务更关键？
```

这就是 Reranker 的作用。

---

# 它在系统里的位置

你的 pipeline 现在应该变成：

```text
User Query
   ↓
Query Router / Retrieval Planner
   ↓
Symbol Search
Full-text Search
Graph Expansion
Vector Search
Repo Map Search
   ↓
Candidate Merge
   ↓
Reranker     ⬅️ 下一步做这个
   ↓
Context Packer
   ↓
code_context.md
   ↓
AI
```

注意：

```text
Retriever 负责“找得到”
Reranker 负责“排得准”
Context Packer 负责“装得好”
```

这三者不要混在一起。

---

# Reranker 具体干什么？

比如用户问：

```text
登录失败为什么返回 500？
```

检索器可能找到了：

```text
AuthService::login
login_handler
AppError
HttpErrorMapper
TokenService
UserRepository
PasswordHasher
README login section
tests/auth_login_test.rs
tests/user_register_test.rs
src/config.rs
```

Reranker 要判断：

```text
高优先级：
- AuthService::login
- login_handler
- AppError
- error_to_response / IntoResponse impl
- tests/auth_login_test.rs

中优先级：
- PasswordHasher
- UserRepository::find_by_email

低优先级：
- TokenService
- README login section
- src/config.rs
- 不相关注册测试
```

然后把候选变成一个排序列表：

```text
1. AuthService::login
2. AppError / error mapping
3. login_handler
4. tests/auth_login_test.rs
5. PasswordHasher::verify
6. UserRepository::find_by_email
...
```

再交给 Context Packer。

---

# 第一版不要上复杂模型

Reranker 第一版可以先用规则和特征打分。

不要一开始就用大模型 rerank。
因为你已经有很多结构化信号：

```text
符号匹配
全文匹配
图关系
chunk 类型
query intent
文件路径
测试关系
错误类型关系
vector score
最近修改
```

这些足够做出一个很强的第一版。

---

# Reranker 的输入

你需要定义一个统一的候选结构。

```rust
pub struct Candidate {
    pub chunk_id: String,
    pub file_path: String,
    pub symbol_id: Option<String>,
    pub qualified_name: Option<String>,
    pub chunk_type: ChunkType,

    pub content_preview: String,
    pub signature: Option<String>,
    pub summary: Option<String>,

    pub sources: Vec<CandidateSourceHit>,
    pub graph_distance: Option<usize>,

    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
    pub token_count: usize,
}
```

来源命中：

```rust
pub struct CandidateSourceHit {
    pub source: CandidateSource,
    pub rank: usize,
    pub score: f32,
    pub reason: String,
}
```

来源类型：

```rust
pub enum CandidateSource {
    SymbolSearch,
    FullTextSearch,
    GraphExpansion,
    VectorSearch,
    RepoMap,
    RecentFile,
}
```

---

# Reranker 的输出

```rust
pub struct RankedCandidate {
    pub candidate: Candidate,
    pub final_score: f32,
    pub rank: usize,
    pub decision: RankDecision,
    pub reasons: Vec<String>,
}
```

```rust
pub enum RankDecision {
    MustInclude,
    Include,
    Summarize,
    Maybe,
    Drop,
}
```

这样 Context Packer 就能根据决策做不同处理：

```text
MustInclude → 放完整代码片段
Include     → 放关键片段
Summarize   → 放摘要和签名
Maybe       → 有 token 剩余再放
Drop        → 不放
```

---

# 第一版打分怎么做？

可以先做一个简单公式：

```text
final_score =
  source_score
+ intent_boost
+ chunk_type_boost
+ graph_boost
+ exact_match_boost
+ path_boost
+ test_boost
+ error_boost
- noise_penalty
- duplicate_penalty
- token_cost_penalty
```

---

## 1. source_score

来自不同检索器的基础分。

不要直接用原始分数相加，因为 symbol search、FTS、vector 的分数尺度不一样。

你可以先用 RRF：

```text
rrf(rank) = 1 / (60 + rank)
```

然后按 Query Router 的权重融合：

```text
source_score =
  symbol_weight * rrf(symbol_rank)
+ fts_weight    * rrf(fts_rank)
+ graph_weight  * rrf(graph_rank)
+ vector_weight * rrf(vector_rank)
+ repo_weight   * rrf(repo_rank)
```

---

## 2. intent_boost

根据 query 类型加权。

### DebugError

用户问：

```text
登录失败为什么返回 500？
```

提高这些：

```text
错误类型
错误映射
handler
测试
日志
panic
HTTP status code
调用链
```

降低这些：

```text
README
无关配置
普通工具函数
```

---

### ModifyBehavior

用户问：

```text
把密码错误改成返回 401
```

提高这些：

```text
目标函数
caller
callee
测试
错误类型
配置项
公共 API
```

---

### Refactor

用户问：

```text
把 UserRepository::find_by_email 改名
```

提高这些：

```text
定义
所有引用
所有实现
所有调用方
测试
导出边界
```

---

### Explain

用户问：

```text
认证流程是怎么走的？
```

提高这些：

```text
repo map slice
module summary
关键入口
调用链
公共类型
```

降低完整函数体比例。

---

## 3. chunk_type_boost

不同 chunk 类型权重不同。

例如：

```text
symbol chunk        +0.20
test chunk          +0.18
error chunk         +0.16
route chunk         +0.15
module summary      +0.10
README/doc chunk    +0.05
large file chunk    -0.10
generated file      -0.50
```

当然这些权重应该根据 intent 改变。

Debug 类任务里：

```text
error chunk
test chunk
log chunk
```

权重应该更高。

Explain 类任务里：

```text
module summary
repo map slice
public API
```

权重应该更高。

---

## 4. graph_boost

图关系非常重要。

比如当前核心符号是：

```text
AuthService::login
```

那么：

```text
distance 0: AuthService::login             +0.30
distance 1: caller/callee/type/test         +0.20
distance 2: 间接关系                       +0.08
distance >2                                +0.00
```

特殊关系可以额外加权：

```text
tested_by          +0.20
implements         +0.18
called_by          +0.15
calls              +0.12
returns_error      +0.18
maps_error_to_http +0.25
```

---

## 5. exact_match_boost

如果用户 query 明确出现符号名：

```text
AuthService::login
find_by_email
AppError
```

那么该符号应该大幅加权：

```text
exact qualified name match   +0.50
exact symbol name match      +0.35
file path match              +0.30
method name match            +0.25
```

这种情况下不要让 vector search 抢走第一名。

---

## 6. token_cost_penalty

长 chunk 要惩罚。

```text
0-300 tokens      无惩罚
300-1000 tokens   -0.05
1000-2500 tokens  -0.15
2500+ tokens      -0.30
```

但要注意：不是长就一定丢。

如果是核心函数，可以：

```text
完整函数太长 → 先放签名 + 摘要 + 关键行号
```

而不是直接丢弃。

---

# Reranker 还要做去重

检索器经常会返回重复内容：

```text
同一个函数被 symbol search 命中
同一个函数被 full-text search 命中
同一个函数被 vector search 命中
同一个文件的多个相邻 chunk 都被命中
```

你需要做三种去重。

---

## 1. chunk 去重

相同 `chunk_id` 合并来源：

```text
AuthService::login
  found_by:
    - symbol search
    - graph expansion
    - vector search
```

不要重复出现三次。

---

## 2. 文件内相邻 chunk 合并

例如：

```text
src/error.rs:10-30
src/error.rs:31-60
src/error.rs:61-90
```

可以合并成：

```text
src/error.rs:10-90
```

或者：

```text
AppError enum + IntoResponse impl
```

---

## 3. 多样性控制

避免上下文全被一个文件占满。

比如候选 top 10 都是：

```text
src/service/auth_service.rs
```

但任务还需要：

```text
handler
error mapping
test
repository
```

Reranker 要保留覆盖面。

可以加一个简单规则：

```text
同一个文件最多 3 个完整 chunk
同一个模块最多 5 个 chunk
除非它们是 MustInclude
```

---

# Reranker 要产生解释

你的 debug 层已经有了，所以 Reranker 必须输出原因。

例如：

```text
1. AuthService::login
   final_score: 0.94
   decision: MustInclude
   reasons:
   - exact symbol match: login
   - graph distance 0
   - method returns AppError
   - matched DebugError intent

2. src/error.rs::AppError
   final_score: 0.87
   decision: MustInclude
   reasons:
   - related error type returned by AuthService::login
   - contains HTTP status mapping
   - DebugError intent boosts error chunks

3. TokenService::issue
   final_score: 0.41
   decision: Summarize
   reasons:
   - callee of AuthService::login
   - not directly related to 500 error
   - token cost too high for full snippet
```

这对后续调参非常关键。

---

# 你要新增的 CLI

加这个命令：

```bash
repoctx rerank "登录失败为什么返回 500？"
```

输出 top candidates：

```text
Intent: DebugError

Ranked Candidates:

1. AuthService::login
   file: src/service/auth_service.rs:42-88
   score: 0.94
   decision: MustInclude
   found_by: symbol, graph, fts
   reason: exact login match; returns AppError; central target

2. AppError
   file: src/error.rs:10-72
   score: 0.87
   decision: MustInclude
   found_by: graph, fts
   reason: returned error type; contains HTTP mapping

3. login_handler
   file: src/api/auth_handler.rs:20-55
   score: 0.81
   decision: Include
   found_by: graph, symbol
   reason: caller of AuthService::login

4. tests/auth_login_test.rs
   score: 0.77
   decision: Include
   found_by: graph
   reason: tests target symbol
```

然后让：

```bash
repoctx build-context "登录失败为什么返回 500？" --show-rerank
```

能显示：

```text
哪些候选被放进上下文
哪些被摘要
哪些被丢弃
为什么
```

---

# Eval 要新增 ranking 指标

之前你评测的是：

```text
有没有召回 must_include 文件
有没有召回 must_include 符号
```

现在要评测排序质量：

```text
Recall@5
Recall@10
MRR
NDCG
MustInclude 排名
Selected Token Efficiency
Noise@K
```

第一版可以先做这些：

```text
Recall@5
Recall@10
MRR
Avg rank of required files
Avg context tokens
Noise ratio
```

例如：

```text
Case: auth_login_500

Required files:
- src/service/auth_service.rs rank 1
- src/error.rs rank 2
- src/api/auth_handler.rs rank 3
- tests/auth_login_test.rs rank 4

Metrics:
- Recall@5: 100%
- MRR: 1.0
- Avg required rank: 2.5
- Noise@10: 20%
- Context tokens: 8,900
```

---

# 你现在的具体任务清单

按顺序做：

```text
1. 定义 Candidate / RankedCandidate
2. 合并不同 retriever 的候选
3. 用 RRF 做初始融合
4. 加 intent-specific boost
5. 加 chunk_type boost
6. 加 graph relation boost
7. 加 exact match boost
8. 加 token cost penalty
9. 加去重和多样性控制
10. 输出 rerank debug reasons
11. build-context 接入 reranker
12. eval 增加 Recall@K / MRR / Noise@K
```

---

# 最小可用版本

第一版只需要做到：

```text
输入：query + candidates
输出：ranked candidates
```

伪代码：

```rust
pub fn rerank(
    query: &str,
    intent: QueryIntent,
    candidates: Vec<Candidate>,
    weights: RetrievalWeights,
) -> Vec<RankedCandidate> {
    let merged = merge_duplicate_candidates(candidates);

    let mut ranked = merged
        .into_iter()
        .map(|candidate| {
            let mut score = 0.0;
            let mut reasons = Vec::new();

            score += rrf_source_score(&candidate, &weights, &mut reasons);
            score += intent_boost(&candidate, &intent, &mut reasons);
            score += chunk_type_boost(&candidate, &intent, &mut reasons);
            score += graph_boost(&candidate, &intent, &mut reasons);
            score += exact_match_boost(query, &candidate, &mut reasons);
            score -= token_cost_penalty(&candidate, &mut reasons);
            score -= noise_penalty(&candidate, &intent, &mut reasons);

            let decision = decide(score, &candidate, &intent);

            RankedCandidate {
                candidate,
                final_score: score,
                rank: 0,
                decision,
                reasons,
            }
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    apply_diversity_control(ranked)
}
```

---

# 做完 Reranker 之后，下一步是什么？

Reranker 做完后，下一步是：

```text
Context Compression / 上下文压缩
```

因为 Reranker 会告诉你：

```text
哪些内容重要
```

但 Context Compression 要决定：

```text
重要内容怎么放得更省 token
```

比如：

```text
完整函数体
只放签名
只放摘要
只放相关行
放 caller/callee 关系
放测试断言摘要
```

但现在不要跳过去。

当前阶段的核心是：

> **让候选内容排得准。**

---

# 一句话

现在下一步做：

> **Reranker：把 symbol / full-text / graph / vector 找回来的候选代码块重新排序，决定哪些 MustInclude、哪些 Include、哪些 Summarize、哪些 Drop。**

你的路线现在是：

```text
repo map ✅
symbol index ✅
chunks ✅
build-context ✅
eval ✅
debug / explain ✅
query router ✅
vector retriever ✅

下一步：
Reranker ⬅️ 现在做这个

之后：
Context Compression
Patch Planner
MCP / IDE / Agent
```

Reranker 做好后，你的系统才会从“找得到很多东西”升级成“知道哪些东西最重要”。
