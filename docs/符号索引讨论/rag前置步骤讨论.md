你现在应该做的不是马上上向量数据库，而是先做一个**最小可用闭环**：

```text
用户输入一个问题
        ↓
系统自动找到相关符号、文件、代码片段、测试
        ↓
打包成 code_context.md
        ↓
交给 AI 使用
```

也就是先实现这个命令：

```bash
repoctx build-context "登录失败为什么返回 500？" --budget 12000
```

输出：

```text
code_context.md
```

这一步做好了，你的 repo map 和符号索引才真正变成 AI 可用的上下文系统。

---

## 你下一步做什么？

我建议按这个顺序做：

```text
1. 做 chunks
2. 做全文检索
3. 做符号检索
4. 做关系扩展
5. 做 context packer
6. 做 evaluation
7. 最后再加 vector search
```

重点是：**先把非向量版本跑通。**

---

# 第一步：把代码切成 chunks

你现在有 symbol index，下一步应该把每个重要符号变成一个可检索的 chunk。

比如这个符号：

```rust
pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError>
```

应该生成一个 chunk：

```json
{
  "chunk_id": "chunk_auth_service_login",
  "chunk_type": "symbol",
  "symbol_id": "sym_auth_service_login",
  "qualified_name": "crate::service::auth_service::AuthService::login",
  "file_path": "src/service/auth_service.rs",
  "start_line": 42,
  "end_line": 88,
  "content": "pub async fn login(...) -> Result<Token, AppError> { ... }",
  "summary": "Handles user login, verifies password, issues token.",
  "hash": "abc123"
}
```

你不要只按固定 token 切代码。
代码库里更好的切块单位是：

```text
函数 chunk
struct chunk
enum chunk
trait chunk
impl chunk
test chunk
module chunk
error chunk
route chunk
config chunk
```

你的第一版可以只做：

```text
symbol chunk
file summary chunk
test chunk
```

---

# 第二步：做全文检索

先不要急着 embedding。
你应该先能搜：

```text
函数名
类型名
错误字符串
路由
配置项
测试名
注释
日志文本
SQL 片段
```

例如用户问：

```text
duplicate email 是在哪里处理的？
```

全文检索应该能命中：

```rust
"duplicate email"
AppError::DuplicateEmail
UserService::register
tests/register_duplicate_email_test.rs
```

Rust 里你可以先用 SQLite FTS5，或者简单一点，先用 Tantivy。

MVP 表结构可以这样：

```sql
CREATE TABLE chunks (
    id TEXT PRIMARY KEY,
    chunk_type TEXT NOT NULL,
    symbol_id TEXT,
    qualified_name TEXT,
    file_path TEXT NOT NULL,
    start_line INTEGER,
    end_line INTEGER,
    content TEXT NOT NULL,
    summary TEXT,
    hash TEXT NOT NULL
);

CREATE VIRTUAL TABLE chunks_fts USING fts5(
    content,
    summary,
    qualified_name,
    file_path
);
```

---

# 第三步：做符号检索

你已经有 symbol index，所以要做一个 `search_symbols(query)`。

它需要支持：

```text
精确匹配：AuthService::login
模糊匹配：login
路径匹配：auth_service
类型匹配：AppError
方法匹配：find_by_email
```

例如：

```bash
repoctx search-symbols "AuthService::login"
```

输出：

```text
crate::service::auth_service::AuthService::login
src/service/auth_service.rs:42-88
kind: method
signature: pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError>
```

这个检索器比向量检索更重要，因为它能保证 AI 看到的函数、类型、trait 是真实存在的。

---

# 第四步：做关系扩展

当系统命中一个符号时，不要只返回这个符号本身。

例如命中：

```text
AuthService::login
```

你应该自动扩展一跳关系：

```text
它被谁调用？
它调用了谁？
它依赖哪些类型？
它返回什么错误？
它对应哪些测试？
它属于哪个 handler？
```

比如：

```text
AuthService::login
  callers:
    - login_handler
  callees:
    - UserRepository::find_by_email
    - PasswordHasher::verify
    - TokenService::issue
  returns:
    - AppError
    - Token
  tests:
    - tests/auth_login_test.rs
```

这一层很关键。

因为用户问：

```text
登录失败为什么返回 500？
```

真正需要的上下文可能不是只有 `login` 函数，还包括：

```text
错误类型定义
错误到 HTTP status 的映射
handler 层
测试文件
```

所以你要做一个：

```rust
expand_context(symbol_id, policy)
```

比如：

```rust
GraphPolicy {
    include_callers: true,
    include_callees: true,
    include_tests: true,
    include_types: true,
    max_depth: 1,
}
```

第一版只做一跳就够了，不要一开始做复杂图算法。

---

# 第五步：做 Context Packer

这是你现在最该实现的核心。

前面的检索器会返回很多候选内容：

```text
symbol hits
text hits
graph hits
test hits
repo map hits
```

Context Packer 要负责把它们整理成一份干净的 Markdown。

目标输出：

````md
<code_context>

# User Query
登录失败为什么返回 500？

# Relevant Files
- src/api/auth_handler.rs
- src/service/auth_service.rs
- src/error.rs
- tests/auth_login_test.rs

# Relevant Symbols

## crate::service::auth_service::AuthService::login
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
// 实际代码
```

## src/error.rs:20-70

```rust
// 错误映射代码
```

# Tests

## tests/auth_login_test.rs

```rust
// 相关测试
```

</code_context>

````

这一步完成后，你的系统就能真正服务 AI 了。

---

# 第六步：做一个最小 CLI

你可以先做这几个命令：

```bash
repoctx index .
repoctx search-symbols "login"
repoctx search-text "duplicate email"
repoctx build-context "登录失败为什么返回 500？" --budget 12000
repoctx show-symbol "crate::service::auth_service::AuthService::login"
````

最重要的是这个：

```bash
repoctx build-context "用户的问题" --budget 12000
```

它应该完成：

```text
查符号
查全文
扩展关系
排序
打包上下文
输出 Markdown
```

---

# 第七步：做 evaluation

这一步不要跳过。

你应该准备一组真实问题：

```json
[
  {
    "query": "登录失败为什么返回 500？",
    "must_include": [
      "src/service/auth_service.rs",
      "src/error.rs",
      "src/api/auth_handler.rs",
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
    "query": "JWT 是在哪里生成的？",
    "must_include": [
      "src/service/token_service.rs",
      "src/service/auth_service.rs"
    ]
  }
]
```

然后跑：

```bash
repoctx eval eval_cases.json
```

统计：

```text
Recall@5
Recall@10
是否包含测试文件
是否包含错误映射文件
最终上下文 token 数
无关文件比例
```

这一步可以帮你判断系统是不是真的变强，而不是只是变复杂。

---

# 第八步：再加向量检索

等你已经有：

```text
chunk
symbol search
full-text search
graph expansion
context packer
evaluation
```

再加 vector search。

因为这时你能客观比较：

```text
没有 vector 的效果
加了 vector 的效果
vector 是否真的提升召回
vector 是否引入噪声
```

向量检索主要补的是这种问题：

```text
“认证流程在哪里？”
“权限校验是怎么做的？”
“缓存失效策略在哪？”
“用户状态变更逻辑在哪？”
```

也就是用户不知道真实符号名时的语义召回。

---

# 你现在最小可做版本

你现在就可以开始写这个函数：

```rust
pub async fn build_context(query: &str, budget_tokens: usize) -> Result<String> {
    let symbol_hits = search_symbols(query).await?;
    let text_hits = search_text(query).await?;

    let mut candidates = CandidateSet::new();
    candidates.add_symbol_hits(symbol_hits);
    candidates.add_text_hits(text_hits);

    let expanded = expand_graph(candidates, GraphPolicy {
        include_callers: true,
        include_callees: true,
        include_tests: true,
        include_types: true,
        max_depth: 1,
    }).await?;

    let ranked = rank_candidates(query, expanded).await?;

    let markdown = pack_context(ranked, PackOptions {
        token_budget: budget_tokens,
        include_repo_map_slice: true,
        include_signatures: true,
        include_code_snippets: true,
        include_tests: true,
    }).await?;

    Ok(markdown)
}
```

这就是你的下一阶段核心。

---

# 推荐路线图

```text
当前状态：
repo map ✅
symbol index ✅

下一步：
chunks ⬅️ 现在做这个
full-text search
symbol search
graph expansion
context packer
evaluation
vector search
reranker
MCP / IDE integration
```

你现在的目标应该非常明确：

> **先做一个能输入 query、输出 code_context.md 的系统。**

只要这个闭环跑通，你后面加 RAG、向量库、reranker、MCP、agent 都会很自然。
如果你已经开始做 `build-context`，那么**下一步应该做 Evaluation，也就是检索评测闭环**。

不要马上加向量库、reranker、MCP。
你现在最需要知道的是：

> 你的 repo map、符号索引、全文检索、关系扩展、Context Packer，到底有没有把 AI 真正需要的代码找出来。

所以下一步是做：

```text
repoctx eval
```

它的目标是：**用一批真实问题测试你的上下文系统是否召回了正确文件、正确符号、正确测试。**

---

## 下一步：做 Evaluation Harness

你需要准备一个评测文件，比如：

```json
[
  {
    "id": "auth_login_500",
    "query": "登录失败为什么返回 500？",
    "must_include_files": [
      "src/service/auth_service.rs",
      "src/error.rs",
      "src/api/auth_handler.rs",
      "tests/auth_login_test.rs"
    ],
    "must_include_symbols": [
      "AuthService::login",
      "AppError"
    ]
  },
  {
    "id": "duplicate_email",
    "query": "用户注册邮箱重复在哪里处理？",
    "must_include_files": [
      "src/service/user_service.rs",
      "src/repository/user_repository.rs",
      "src/error.rs"
    ],
    "must_include_symbols": [
      "UserService::register",
      "UserRepository::find_by_email"
    ]
  },
  {
    "id": "jwt_generation",
    "query": "JWT token 是在哪里生成的？",
    "must_include_files": [
      "src/service/token_service.rs",
      "src/service/auth_service.rs"
    ],
    "must_include_symbols": [
      "TokenService::issue",
      "AuthService::login"
    ]
  }
]
```

然后实现命令：

```bash
repoctx eval eval_cases.json
```

它内部做：

```text
读取每个 query
        ↓
调用 build-context
        ↓
记录被选中的 files / symbols / chunks
        ↓
和 must_include 对比
        ↓
输出命中率、遗漏项、token 使用量、噪声比例
```

---

## 你要评测哪些指标？

第一版不用复杂，先做这几个就够了：

```text
File Recall
必须包含的文件，有多少被召回了

Symbol Recall
必须包含的符号，有多少被召回了

Context Token Count
最终上下文用了多少 token

Noise Ratio
进入上下文但明显无关的文件比例

Missing Items
哪些关键文件/符号没被召回
```

输出可以长这样：

```text
Eval Result

Case: auth_login_500
Query: 登录失败为什么返回 500？

Required files:
✅ src/service/auth_service.rs
✅ src/error.rs
✅ src/api/auth_handler.rs
❌ tests/auth_login_test.rs

Required symbols:
✅ AuthService::login
✅ AppError

Stats:
- file recall: 3/4 = 75%
- symbol recall: 2/2 = 100%
- context tokens: 8,420
- selected chunks: 11
- missing: tests/auth_login_test.rs
```

这个结果会直接告诉你下一步该优化哪里。

比如这里漏了测试文件，说明你需要增强：

```text
symbol → test 的关系扩展
```

而不是盲目加向量数据库。

---

## 为什么 Evaluation 是现在最重要的一步？

因为没有评测，你后面加任何东西都无法判断是否真的变好了。

你可能加了向量库，感觉系统更高级了，但实际可能出现：

```text
召回更多无关代码
token 浪费更多
关键测试还是找不到
AI 修改成功率没有提升
```

有了 evaluation，你就可以对比：

```text
只有 symbol search 的效果
symbol + full-text 的效果
symbol + full-text + graph expansion 的效果
再加 vector search 的效果
再加 reranker 的效果
```

这样你的系统会变成工程系统，而不是玄学调参。

---

## eval 之后你会知道该做什么

评测结果会把下一步自动暴露出来。

如果你发现：

```text
函数名查得准，但用户自然语言问题找不到相关模块
```

那下一步是：

```text
加 vector search
```

如果你发现：

```text
命中了核心函数，但漏了 caller / callee / test
```

那下一步是：

```text
增强 relation graph
```

如果你发现：

```text
召回了很多东西，但上下文很乱
```

那下一步是：

```text
优化 Context Packer
```

如果你发现：

```text
经常漏掉错误字符串、日志、SQL、配置项
```

那下一步是：

```text
增强 full-text search / chunk 类型
```

如果你发现：

```text
检索到了正确 chunk，但排序太靠后
```

那下一步是：

```text
做 reranker
```

---

## 你的路线应该变成这样

```text
repo map ✅
symbol index ✅
chunks ✅ / 进行中
build-context ✅ / 进行中

下一步：
Evaluation Harness  ⬅️ 现在做这个

然后：
根据 eval 结果优化检索
再加 vector search
再加 reranker
再做 MCP / IDE / agent tool
```

---

## 最小实现目标

你现在只要实现这个命令：

```bash
repoctx eval eval_cases.json
```

它能输出：

```text
每个 case 召回了哪些文件
每个 case 召回了哪些符号
漏了哪些关键项
最终上下文用了多少 token
整体 Recall@5 / Recall@10
```

这一步完成后，你的系统就有了“自我改进能力”。

一句话：

> **下一步不是继续堆功能，而是做评测。先证明你的 context engine 能稳定找对上下文，然后再决定要不要加向量库、reranker 和 MCP。**
在 `repoctx eval` 之后，下一步不是马上做向量库，也不是马上做 MCP，而是做：

# Explainable Retrieval / 检索调试层

也就是让你的系统能回答：

> 为什么这次选中了这些文件？
> 为什么漏掉了关键文件？
> 某个 chunk 是在哪一层被召回的？
> 是没搜到，还是搜到了但排序太低，还是排序够高但被 Context Packer 丢掉了？

这一步非常关键。
没有它，你后面加 vector search、reranker、agent tool，都会变成黑盒调参。

---

## 你现在要做的命令

在已有的：

```bash
repoctx build-context "登录失败为什么返回 500？" --budget 12000
repoctx eval eval_cases.json
```

基础上，加这几个命令：

```bash
repoctx debug "登录失败为什么返回 500？" --budget 12000
```

以及：

```bash
repoctx eval eval_cases.json --explain
```

再加一个对比命令：

```bash
repoctx compare runs/baseline.json runs/current.json
```

这样你的系统就能从“能跑”变成“能优化”。

---

# 1. debug 命令应该输出什么？

例如：

```bash
repoctx debug "登录失败为什么返回 500？" --budget 12000
```

输出应该长这样：

```text
Query:
登录失败为什么返回 500？

Selected Context:
✅ src/service/auth_service.rs
✅ src/error.rs
✅ src/api/auth_handler.rs
❌ tests/auth_login_test.rs

Candidate Trace:

1. src/service/auth_service.rs:42-88
   chunk: AuthService::login
   selected: yes
   final_score: 0.91

   Found by:
   - symbol_search: login, score 0.82
   - full_text: 登录/login, score 0.44
   - graph_expansion: from login_handler, score 0.63

   Why selected:
   - exact method name match
   - high graph centrality
   - contains AppError return type
   - within token budget

2. src/error.rs:12-80
   chunk: AppError
   selected: yes
   final_score: 0.78

   Found by:
   - graph_expansion: AuthService::login returns AppError
   - full_text: 500, InternalServerError

   Why selected:
   - error type related to selected login function
   - contains HTTP status mapping

3. tests/auth_login_test.rs
   selected: no
   final_score: 0.21

   Found by:
   - not found

   Problem:
   - no test relation from AuthService::login to this test
   - test file not matched by full-text search
```

这个输出会直接告诉你：

```text
测试文件漏了，不是因为排序低，而是根本没被召回。
```

那么下一步就不是加 reranker，而是增强：

```text
symbol → test
function → test
file → test
```

的关系扩展。

---

# 2. 你要记录候选项的生命周期

每个候选 chunk 都应该有一条 trace。

比如：

```rust
pub struct CandidateTrace {
    pub chunk_id: String,
    pub file_path: String,
    pub symbol_id: Option<String>,
    pub qualified_name: Option<String>,

    pub found_by: Vec<RetrievalHit>,
    pub score_events: Vec<ScoreEvent>,

    pub final_score: f32,
    pub selected: bool,
    pub dropped_reason: Option<String>,
}
```

其中：

```rust
pub struct RetrievalHit {
    pub retriever: RetrieverKind,
    pub raw_score: f32,
    pub reason: String,
}
```

例如：

```rust
pub enum RetrieverKind {
    SymbolSearch,
    FullTextSearch,
    GraphExpansion,
    RepoMap,
    VectorSearch,
    RecentFiles,
}
```

评分变化也要记录：

```rust
pub struct ScoreEvent {
    pub stage: String,
    pub delta: f32,
    pub reason: String,
}
```

例如：

```json
{
  "chunk_id": "chunk_auth_service_login",
  "file_path": "src/service/auth_service.rs",
  "qualified_name": "crate::service::auth_service::AuthService::login",
  "found_by": [
    {
      "retriever": "SymbolSearch",
      "raw_score": 0.82,
      "reason": "query term 'login' matched method name"
    },
    {
      "retriever": "GraphExpansion",
      "raw_score": 0.63,
      "reason": "called by login_handler"
    }
  ],
  "score_events": [
    {
      "stage": "ranker",
      "delta": 0.15,
      "reason": "public method"
    },
    {
      "stage": "ranker",
      "delta": 0.20,
      "reason": "contains error return type"
    }
  ],
  "final_score": 0.91,
  "selected": true,
  "dropped_reason": null
}
```

这就是你的检索系统的“黑盒记录仪”。

---

# 3. 你要区分四种失败

Evaluation 只能告诉你：

```text
漏了 src/error.rs
漏了 tests/auth_login_test.rs
```

但 Debug 层要告诉你为什么漏。

常见失败分四类。

---

## 失败一：根本没有召回

例如：

```text
tests/auth_login_test.rs 没有进入 candidates
```

说明问题在召回层。

解决方向：

```text
增强 full-text search
增强 test chunk
增强 symbol → test 关系
增加文件名/路径别名
必要时加 vector search
```

---

## 失败二：召回了，但排序太低

例如：

```text
src/error.rs 被找到了，但排名第 37，没进上下文
```

说明问题在 ranking 层。

解决方向：

```text
提高 error mapping 文件权重
提高被核心函数返回的类型权重
提高 caller/callee 一跳关系权重
降低普通文本命中的权重
```

---

## 失败三：排序够高，但被 Context Packer 丢了

例如：

```text
tests/auth_login_test.rs 排名第 6，但 packer 因 token budget 丢了
```

说明问题在打包层。

解决方向：

```text
测试文件只放相关 test function
长文件改成摘要 + 精确片段
减少重复 chunk
优先保留 test / error / public API
```

---

## 失败四：上下文进来了，但太吵

例如：

```text
找到了 10 个 auth 文件，但真正相关的只有 3 个
```

说明问题在去重和压缩层。

解决方向：

```text
按 file 聚合
按 symbol 聚合
相邻 chunk 合并
低分 chunk 摘要化
重复 signature 去掉
```

---

# 4. 你需要做 ablation test

也就是分别测试每个检索器的贡献。

实现这些参数：

```bash
repoctx eval eval_cases.json --only symbol
repoctx eval eval_cases.json --symbol --fts
repoctx eval eval_cases.json --symbol --fts --graph
repoctx eval eval_cases.json --symbol --fts --graph --vector
```

你要看到类似结果：

```text
Strategy                      File Recall   Symbol Recall   Avg Tokens
symbol only                   52%           71%             4,200
symbol + fts                  68%           74%             6,700
symbol + fts + graph          81%           86%             9,100
symbol + fts + graph + vector 88%           87%             11,800
```

这样你才能知道：

```text
graph expansion 到底有没有用
vector search 到底有没有提升
是不是 token 消耗太高
某个模块是不是只增加噪声
```

这一步比盲目加功能重要得多。

---

# 5. 你现在应该新增的数据表

如果你用 SQLite，可以加这些表。

## retrieval_runs

```sql
CREATE TABLE retrieval_runs (
    id TEXT PRIMARY KEY,
    query TEXT NOT NULL,
    budget_tokens INTEGER NOT NULL,
    selected_context TEXT NOT NULL,
    total_tokens INTEGER,
    created_at INTEGER NOT NULL
);
```

## candidate_traces

```sql
CREATE TABLE candidate_traces (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    chunk_id TEXT NOT NULL,
    file_path TEXT NOT NULL,
    symbol_id TEXT,
    qualified_name TEXT,
    final_score REAL NOT NULL,
    selected INTEGER NOT NULL,
    dropped_reason TEXT,
    trace_json TEXT NOT NULL
);
```

## eval_results

```sql
CREATE TABLE eval_results (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    case_id TEXT NOT NULL,
    query TEXT NOT NULL,
    file_recall REAL NOT NULL,
    symbol_recall REAL NOT NULL,
    missing_files_json TEXT NOT NULL,
    missing_symbols_json TEXT NOT NULL,
    total_tokens INTEGER
);
```

这几张表的意义是：

```text
每次检索都可以回放
每次评测都可以比较
每次优化都有证据
```

---

# 6. 你的下一阶段目标

你现在的目标应该是：

```text
让每一次 build-context 都可解释、可回放、可比较。
```

也就是实现：

```bash
repoctx debug "query"
repoctx eval eval_cases.json --explain
repoctx compare old_run new_run
```

当你完成这一步后，你就可以非常清楚地知道：

```text
该增强 symbol search？
该增强 graph？
该优化 packer？
该加 vector？
该加 reranker？
```

而不是靠感觉决定。

---

# 7. Debug 层完成后，再进入下一步

等你有了 debug/eval/compare，下一步才是：

```text
根据失败类型做定向增强
```

一般顺序是：

```text
1. 修 symbol search
2. 修 full-text search
3. 修 graph expansion
4. 修 context packer
5. 加 vector search
6. 加 reranker
7. 做 MCP / IDE integration
```

不要跳到第 5 步。

---

# 最终建议

你现在下一步做这个：

```bash
repoctx debug
```

它的核心价值是：

> 把每个被召回、被排序、被丢弃、被打包的 chunk 全部记录下来，并解释原因。

你的系统路线现在应该是：

```text
repo map ✅
symbol index ✅
chunks ✅
build-context ✅
eval ✅

下一步：
debug / explain / compare  ⬅️ 现在做这个

然后：
基于失败类型优化检索
再加 vector search
再加 reranker
最后做 MCP / IDE / agent
```

一句话：

> **先让检索系统变得可解释，再让它变得更智能。**
下一步做：

# Retrieval Planner / Query Router

也就是：**查询规划器**。

你现在已经有了：

```text
repo map
symbol index
chunks
build-context
eval
debug / explain / compare
```

这些能力解决了：

```text
能不能找？
找得对不对？
为什么这样找？
哪里漏了？
```

下一步要解决的是：

> **面对不同类型的问题，系统应该自动选择不同的检索策略。**

不要让所有 query 都走同一套流程：

```text
symbol search + full-text + graph expansion + packer
```

因为不同问题需要的上下文完全不一样。

---

## 现在要做的核心功能

你要加一个模块：

```text
planner/
  query_analyzer.rs
  retrieval_profile.rs
  graph_policy.rs
  pack_policy.rs
```

它负责把用户问题变成一个检索计划。

例如用户问：

```text
登录失败为什么返回 500？
```

Planner 应该判断这是：

```text
intent: debug_error
```

然后生成计划：

```text
优先：
- full-text search 错误码 / 错误字符串
- symbol search login / auth / error
- graph expansion 找 caller / callee / error mapping / tests

降低：
- 大范围 repo map
- 无关模块摘要

Context Packer：
- 必须保留错误类型
- 必须保留 HTTP status 映射
- 必须保留相关测试
```

而不是所有问题都用一个固定权重。

---

# 为什么现在做它？

因为你已经有了 eval 和 debug。

现在你可以看到很多失败类型，比如：

```text
测试文件没召回
错误映射没召回
核心函数召回了但 caller 漏了
自然语言问题找不到真实符号
上下文塞太多无关文件
```

这些问题不能只靠“加向量库”解决。

你需要先让系统知道：

> 这次用户到底是在问定位、解释、调试、修改、重构，还是新增功能？

这就是 Query Router 的作用。

---

# 你应该支持的 query 类型

第一版可以分 6 类。

## 1. Locate / 定位类

用户问：

```text
登录逻辑在哪里？
JWT 是在哪里生成的？
哪个模块处理邮箱重复？
```

策略：

```text
symbol search 高权重
full-text search 中权重
repo map module summary 中权重
graph expansion 一跳即可
```

输出重点：

```text
文件
符号
模块职责
入口函数
```

---

## 2. Explain / 解释类

用户问：

```text
认证流程是怎么走的？
注册用户的完整链路是什么？
这个项目的错误处理机制是什么？
```

策略：

```text
repo map 高权重
symbol graph 高权重
module summary 高权重
代码片段中等
测试中等
```

输出重点：

```text
流程
模块关系
关键符号
少量代码签名
```

不要塞太多完整函数体。

---

## 3. Debug / 报错类

用户问：

```text
登录失败为什么返回 500？
duplicate key value violates unique constraint 是哪里来的？
这个 panic 是怎么触发的？
```

策略：

```text
full-text search 高权重
错误字符串 / 日志 / panic / status code 高权重
symbol search 中权重
graph expansion 高权重
tests 高权重
```

输出重点：

```text
报错位置
错误类型
错误转换层
调用链
相关测试
日志字符串
```

---

## 4. Modify / 修改行为类

用户问：

```text
把密码错误改成返回 401
注册时邮箱重复返回 Conflict
修改 token 过期时间逻辑
```

策略：

```text
symbol search 高权重
graph expansion 高权重
tests 高权重
callee/caller 都要
error/config/type 关系都要
```

输出重点：

```text
要改的函数
调用方
被调用方
错误类型
配置项
测试
```

这是 AI coding 最常见的类型。

---

## 5. Refactor / 重构类

用户问：

```text
把 UserRepository::find_by_email 改名
把 AuthService 拆成 AuthService 和 TokenService
把这个 trait 抽出来
```

策略：

```text
references 高权重
implementations 高权重
callers 高权重
public API 边界高权重
tests 高权重
```

输出重点：

```text
定义
所有引用
所有实现
所有调用方
测试
对外暴露 API
```

这种任务不能只靠 vector search。

---

## 6. Add Feature / 新增功能类

用户问：

```text
增加 refresh token
新增邮箱验证码登录
新增管理员冻结用户功能
```

策略：

```text
相似功能搜索
repo map 高权重
symbol search 中权重
vector search 后面可加入
tests 高权重
interface / trait / handler / service / repository 全链路
```

输出重点：

```text
相似功能
项目分层模式
已有接口风格
测试风格
错误处理风格
路由风格
```

---

# 你要实现的输出：RetrievalPlan

用户输入：

```text
登录失败为什么返回 500？
```

系统先不要直接检索，而是先生成：

```json
{
  "intent": "debug_error",
  "retrievers": {
    "symbol": true,
    "full_text": true,
    "graph": true,
    "repo_map": true,
    "vector": false
  },
  "weights": {
    "symbol": 0.25,
    "full_text": 0.35,
    "graph": 0.30,
    "repo_map": 0.05,
    "recent": 0.05
  },
  "graph_policy": {
    "include_callers": true,
    "include_callees": true,
    "include_tests": true,
    "include_types": true,
    "include_error_mappers": true,
    "max_depth": 1
  },
  "pack_policy": {
    "include_code_snippets": true,
    "include_signatures": true,
    "include_tests": true,
    "include_error_mapping": true,
    "prefer_exact_snippets": true,
    "prefer_summaries_for_large_files": true
  }
}
```

然后 `build-context` 根据这个 plan 去执行。

---

# Rust 结构可以这样设计

```rust
#[derive(Debug, Clone)]
pub enum QueryIntent {
    Locate,
    Explain,
    DebugError,
    ModifyBehavior,
    Refactor,
    AddFeature,
    Unknown,
}
```

```rust
#[derive(Debug, Clone)]
pub struct QueryFeatures {
    pub raw_query: String,

    pub symbol_like_terms: Vec<String>,
    pub file_like_terms: Vec<String>,
    pub error_like_terms: Vec<String>,
    pub route_like_terms: Vec<String>,
    pub status_codes: Vec<u16>,
    pub quoted_strings: Vec<String>,

    pub mentions_test: bool,
    pub mentions_refactor: bool,
    pub mentions_error: bool,
    pub mentions_modify: bool,
    pub mentions_explain: bool,
}
```

```rust
#[derive(Debug, Clone)]
pub struct RetrievalPlan {
    pub intent: QueryIntent,
    pub retrievers: RetrieverSwitches,
    pub weights: RetrievalWeights,
    pub graph_policy: GraphPolicy,
    pub pack_policy: PackPolicy,
}
```

```rust
#[derive(Debug, Clone)]
pub struct RetrieverSwitches {
    pub symbol: bool,
    pub full_text: bool,
    pub graph: bool,
    pub repo_map: bool,
    pub vector: bool,
    pub recent_files: bool,
}
```

```rust
#[derive(Debug, Clone)]
pub struct RetrievalWeights {
    pub symbol: f32,
    pub full_text: f32,
    pub graph: f32,
    pub repo_map: f32,
    pub vector: f32,
    pub recent_files: f32,
}
```

```rust
#[derive(Debug, Clone)]
pub struct GraphPolicy {
    pub include_callers: bool,
    pub include_callees: bool,
    pub include_tests: bool,
    pub include_types: bool,
    pub include_implementations: bool,
    pub include_references: bool,
    pub include_error_mappers: bool,
    pub max_depth: usize,
}
```

```rust
#[derive(Debug, Clone)]
pub struct PackPolicy {
    pub include_repo_map_slice: bool,
    pub include_signatures: bool,
    pub include_code_snippets: bool,
    pub include_tests: bool,
    pub include_error_mapping: bool,
    pub prefer_exact_snippets: bool,
    pub prefer_summaries_for_large_files: bool,
}
```

---

# 第一版不要用 LLM 做 planner

第一版用规则就够了。

比如：

```rust
pub fn detect_intent(features: &QueryFeatures) -> QueryIntent {
    if features.mentions_refactor {
        return QueryIntent::Refactor;
    }

    if features.mentions_error || !features.error_like_terms.is_empty() || !features.status_codes.is_empty() {
        return QueryIntent::DebugError;
    }

    if features.mentions_modify {
        return QueryIntent::ModifyBehavior;
    }

    if features.mentions_explain {
        return QueryIntent::Explain;
    }

    if !features.symbol_like_terms.is_empty() || !features.file_like_terms.is_empty() {
        return QueryIntent::Locate;
    }

    QueryIntent::Unknown
}
```

关键词规则可以先这样：

```text
DebugError:
为什么、报错、panic、500、401、403、duplicate、failed、error、exception、trace、日志

ModifyBehavior:
修改、改成、返回、支持、增加、删除、调整、变更

Refactor:
重构、改名、移动、抽取、拆分、合并、rename、extract

Explain:
解释、流程、机制、怎么走、架构、调用链、为什么设计

Locate:
在哪里、哪个文件、哪个函数、入口、定义
```

后面你可以加 LLM-based planner，但不要一开始就加。规则版更容易 eval、debug、compare。

---

# 新增 CLI 命令

你现在应该加这个：

```bash
repoctx plan "登录失败为什么返回 500？"
```

输出：

```text
Intent:
DebugError

Detected features:
- status code: 500
- error-like term: 失败
- symbol-like term: 登录/login

Retrievers:
- symbol: enabled, weight 0.25
- full_text: enabled, weight 0.35
- graph: enabled, weight 0.30
- repo_map: enabled, weight 0.05
- vector: disabled

Graph policy:
- include callers: yes
- include callees: yes
- include tests: yes
- include error mappers: yes
- max depth: 1

Pack policy:
- include exact code snippets: yes
- include tests: yes
- include error mapping: yes
```

然后让：

```bash
repoctx build-context "登录失败为什么返回 500？" --show-plan
```

输出时包含：

```md
<retrieval_plan>
intent: debug_error
symbol_weight: 0.25
full_text_weight: 0.35
graph_weight: 0.30
</retrieval_plan>
```

这样 debug 时你能看到：

```text
这次为什么使用了这种策略。
```

---

# 它如何接入现有 build-context

你现在的流程应该从：

```text
query
  ↓
symbol search
full-text search
graph expansion
ranking
packing
```

变成：

```text
query
  ↓
query analyzer
  ↓
retrieval planner
  ↓
按 plan 执行 symbol / fts / graph / repo map / vector
  ↓
按 plan 计算分数
  ↓
按 plan 打包 context
  ↓
debug trace 记录 plan 和候选生命周期
```

伪代码：

```rust
pub async fn build_context(query: &str, budget_tokens: usize) -> Result<String> {
    let features = query_analyzer.analyze(query)?;
    let plan = retrieval_planner.plan(&features);

    let mut candidates = CandidateSet::new();

    if plan.retrievers.symbol {
        let hits = symbol_retriever.search(query).await?;
        candidates.add(hits, CandidateSource::Symbol);
    }

    if plan.retrievers.full_text {
        let hits = fts_retriever.search(query).await?;
        candidates.add(hits, CandidateSource::FullText);
    }

    if plan.retrievers.repo_map {
        let hits = repo_map_retriever.search(query).await?;
        candidates.add(hits, CandidateSource::RepoMap);
    }

    if plan.retrievers.graph {
        candidates = graph_expander
            .expand(candidates, &plan.graph_policy)
            .await?;
    }

    if plan.retrievers.vector {
        let hits = vector_retriever.search(query).await?;
        candidates.add(hits, CandidateSource::Vector);
    }

    let ranked = ranker.rank(query, candidates, &plan.weights).await?;

    let packed = context_packer
        .pack(ranked, budget_tokens, &plan.pack_policy)
        .await?;

    Ok(packed)
}
```

---

# eval 也要按 intent 分组

做了 planner 之后，你的 eval 输出不要只看整体 recall。

要按任务类型拆开：

```text
Locate cases:
- File recall
- Symbol recall

DebugError cases:
- Error mapping recall
- Test recall
- Caller/callee recall

ModifyBehavior cases:
- Target symbol recall
- Related test recall
- Type/error recall

Refactor cases:
- Reference recall
- Implementation recall
- Public API recall

Explain cases:
- Module recall
- Graph coverage
- Token compression ratio
```

这样你会知道：

```text
定位类问题已经很好
调试类问题漏测试
重构类问题缺 references
解释类问题上下文太长
```

这比一个总分有用得多。

---

# 做完 planner 后，下一步才是向量搜索

当你有了 Query Router，向量搜索就不会乱用。

例如：

```text
Locate / Refactor:
vector 权重低

Explain / AddFeature:
vector 权重高

DebugError:
vector 可选，full-text 和 graph 更重要

ModifyBehavior:
vector 用来找相似实现，但不能替代 symbol / tests
```

这时你加 vector search 才是可控的。

否则你会遇到：

```text
自然语言相似但代码关系无关
vector 召回很多“看起来像”的代码
token 被噪声吃掉
AI 看到上下文反而更混乱
```

---

# 你现在的具体任务清单

按顺序做：

```text
1. 实现 QueryFeatures
2. 实现 query_analyzer
3. 实现 QueryIntent
4. 实现 RetrievalPlan
5. 为每种 intent 写默认 profile
6. build-context 接入 plan
7. debug 输出 plan
8. eval 按 intent 分组统计
9. compare 支持对比不同 planner profile
```

最小目标是：

```bash
repoctx plan "用户问题"
```

能输出一份清楚的 retrieval plan。

然后：

```bash
repoctx build-context "用户问题" --show-plan
```

能按照这个 plan 检索和打包上下文。

---

# 这一步完成后的系统状态

完成 Query Router 后，你的系统会从：

```text
固定检索流程
```

升级成：

```text
自适应检索流程
```

也就是：

```text
用户问定位 → 精确符号优先
用户问报错 → 全文/错误/测试优先
用户问修改 → 目标符号/调用图/测试优先
用户问重构 → 引用/实现/API 边界优先
用户问解释 → repo map/模块摘要/关系图优先
用户问新增功能 → 相似实现/项目模式/测试风格优先
```

这会明显提升后续所有模块的效果，包括 vector search、reranker、MCP 和 agent。

---

一句话：

> **下一步做 Query Router / Retrieval Planner，让系统先判断“这是什么类型的问题”，再决定怎么检索、怎么扩展关系、怎么打包上下文。**
