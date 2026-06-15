下一步做：

# Context Compression / 上下文压缩器

你前面做了 **Reranker**，它解决的是：

> 哪些候选代码块更重要？

现在要解决的是：

> 这些重要内容应该以什么形态放进上下文，才能在 token budget 内最大化有效信息？

也就是：

```text
Reranker 负责排序
Context Compression 负责压缩
Context Packer 负责最终排版
```

---

## 为什么下一步是 Context Compression？

因为 Reranker 做完后，你会得到很多高价值候选：

```text
AuthService::login
login_handler
AppError
error_to_response
PasswordHasher::verify
UserRepository::find_by_email
tests/auth_login_test.rs
README auth section
TokenService::issue
```

但你的上下文预算可能只有：

```text
8k tokens
12k tokens
32k tokens
```

你不能把所有候选都完整塞进去。

所以系统必须决定：

```text
哪些放完整代码？
哪些只放相关行？
哪些只放签名？
哪些只放摘要？
哪些只放关系？
哪些直接丢弃？
```

这一步就是 **Context Compression**。

---

# 它在系统里的位置

你的 pipeline 现在应该变成：

```text
User Query
   ↓
Query Router
   ↓
Hybrid Retrieval
   ↓
Reranker
   ↓
Context Compression   ⬅️ 现在做这个
   ↓
Context Packer
   ↓
code_context.md
   ↓
AI
```

也可以更细地理解为：

```text
Retriever：找候选
Reranker：排重要性
Compressor：压缩表达形式
Packer：组装成最终 Markdown/XML
```

---

# Context Compression 要做什么？

它不是简单摘要。

它是一个 **budget-aware context reducer**，也就是根据任务类型、候选重要性和 token budget，决定每段代码的展示级别。

---

## 压缩级别

你可以定义 7 个级别。

### Level 0：Drop

完全不放。

适合：

```text
低分候选
重复候选
明显无关文件
generated code
vendor code
```

---

### Level 1：Relation Only

只放关系，不放代码。

例如：

```md
- TokenService::issue is called by AuthService::login.
```

适合：

```text
间接相关 callee
辅助工具函数
低优先级依赖
```

---

### Level 2：Signature Only

只放函数/类型签名。

例如：

```rust
pub async fn issue(&self, user: &User) -> Result<Token, AppError>;
```

适合：

```text
需要知道 API 怎么调用
但不需要知道内部实现
```

---

### Level 3：Summary + Signature

放摘要和签名。

例如：

````md
## TokenService::issue
Role: Creates a JWT token for an authenticated user.

```rust
pub async fn issue(&self, user: &User) -> Result<Token, AppError>;
````

````

适合：

```text
相关但不是修改核心的服务
外部依赖
repository method
helper method
````

---

### Level 4：Focused Snippet

只放相关代码行附近的一小段。

例如：

```rust
if !password_hasher.verify(password, &user.password_hash)? {
    return Err(AppError::Internal(anyhow!("password mismatch")));
}
```

适合：

```text
报错位置
分支逻辑
错误映射
测试断言
SQL 查询
配置读取
```

这是最重要的压缩形式。

---

### Level 5：Full Symbol Body

放完整函数、完整 enum、完整 trait、完整 impl method。

适合：

```text
用户要修改的目标函数
核心错误类型
核心测试函数
核心 handler
```

---

### Level 6：Full File

极少使用。

适合：

```text
文件很短
配置文件
核心宏文件
小型 error.rs
小型 test file
```

不要默认放完整文件。

---

# 推荐决策表

| 候选类型               | 默认压缩方式                             |
| ------------------ | ---------------------------------- |
| 目标函数               | Full Symbol Body                   |
| 目标 handler         | Full Symbol Body 或 Focused Snippet |
| 错误类型 enum          | Full Symbol Body                   |
| 错误到 HTTP status 映射 | Focused Snippet / Full Symbol Body |
| 相关测试函数             | Full Symbol Body                   |
| 被调用的 repository 方法 | Signature + Summary                |
| 被调用的 helper        | Signature Only                     |
| 间接依赖               | Relation Only                      |
| README / 文档        | Summary                            |
| 大文件                | Focused Snippet                    |
| generated 文件       | Drop                               |

---

# 不同任务的压缩策略

你的 Query Router 已经能识别 intent，所以 Context Compression 应该按 intent 调整。

---

## DebugError

例如：

```text
登录失败为什么返回 500？
```

优先完整保留：

```text
报错附近代码
错误类型
错误映射
handler
相关测试
日志/panic/错误字符串
```

压缩掉：

```text
普通 helper
无关 service
README
不相关测试
```

策略：

```text
目标函数：完整
错误映射：完整或 focused snippet
测试：完整相关 test function
callee：签名 + 摘要
repo map：小片段
```

---

## ModifyBehavior

例如：

```text
把密码错误改成返回 401。
```

优先完整保留：

```text
要改的函数
错误类型
调用方
测试
配置项
```

策略：

```text
目标函数：完整
caller：focused snippet
callee：签名
测试：完整
error mapping：完整
```

---

## Explain

例如：

```text
认证流程是怎么走的？
```

不应该塞大量完整函数体。

优先：

```text
repo map slice
模块摘要
关键函数签名
调用链
少量入口代码
```

策略：

```text
模块摘要：多放
函数体：少放
调用关系：多放
测试：摘要
```

---

## Refactor

例如：

```text
把 UserRepository::find_by_email 改名。
```

优先完整保留：

```text
定义
trait
impl
所有引用点
测试
public API export
```

策略：

```text
定义：完整
trait/impl：完整或 focused
引用点：focused snippet
测试：focused snippet
间接依赖：relation only
```

---

## AddFeature

例如：

```text
新增邮箱验证码登录。
```

优先：

```text
相似功能
项目分层模式
handler/service/repository/test 风格
错误处理模式
```

策略：

```text
相似功能核心代码：完整
其他相似文件：摘要
测试风格：完整一个代表性测试
接口/trait：签名
repo map：适当多放
```

---

# 你要新增的数据结构

可以定义：

```rust
#[derive(Debug, Clone)]
pub enum CompressionLevel {
    Drop,
    RelationOnly,
    SignatureOnly,
    SummaryAndSignature,
    FocusedSnippet,
    FullSymbolBody,
    FullFile,
}
```

然后给每个 ranked candidate 生成一个压缩计划：

```rust
#[derive(Debug, Clone)]
pub struct CompressionDecision {
    pub chunk_id: String,
    pub level: CompressionLevel,
    pub reason: Vec<String>,

    pub original_tokens: usize,
    pub compressed_tokens: usize,

    pub selected_line_ranges: Vec<LineRange>,
}
```

```rust
#[derive(Debug, Clone)]
pub struct LineRange {
    pub start_line: usize,
    pub end_line: usize,
    pub reason: String,
}
```

最终输出：

```rust
pub struct CompressedBlock {
    pub chunk_id: String,
    pub file_path: String,
    pub title: String,
    pub level: CompressionLevel,
    pub content: String,
    pub token_count: usize,
    pub provenance: Provenance,
}
```

---

# 核心函数

你现在应该实现这个：

```rust
pub fn compress_context(
    query: &str,
    intent: QueryIntent,
    ranked: Vec<RankedCandidate>,
    budget_tokens: usize,
    policy: CompressionPolicy,
) -> Vec<CompressedBlock> {
    // 1. 先保留 MustInclude
    // 2. 根据 intent 分配预算
    // 3. 为每个 candidate 决定压缩级别
    // 4. 抽取 focused snippets
    // 5. 去重和合并相邻行
    // 6. 超预算时逐级降级
    // 7. 输出 compressed blocks
}
```

---

# Budget 分配

不要把 token budget 当成一个大桶随便塞。

建议分区。

例如总预算 12000 tokens：

```text
Task / Plan:        500
Repo map slice:     1000
Core symbols:       5000
Related snippets:   2500
Tests:              2000
Relations:          700
Safety margin:      300
```

不同 intent 可以不同。

---

## DebugError 预算

```text
Core code:      35%
Error mapping:  20%
Tests:          20%
Call chain:     15%
Repo map:        5%
Margin:          5%
```

---

## Explain 预算

```text
Repo map:       25%
Module summary: 25%
Call graph:     20%
Signatures:     20%
Code snippets:  10%
```

---

## Refactor 预算

```text
Definitions:    25%
References:     30%
Implementations:20%
Tests:          15%
Exports/API:    10%
```

---

# Focused Snippet 怎么抽取？

这是 Context Compression 的关键。

不要只会完整函数体和摘要。
要能抽取相关代码窗口。

来源可以有几类：

```text
全文搜索命中的行
错误字符串所在行
状态码所在行
符号定义行
调用表达式所在行
测试 assert 所在行
return Err 所在行
match arm 所在行
```

然后扩展上下文窗口：

```text
命中行前 5 行
命中行后 8 行
遇到函数边界停止
多个相近窗口合并
```

例如：

```rust
fn select_focused_ranges(
    candidate: &RankedCandidate,
    query_features: &QueryFeatures,
) -> Vec<LineRange> {
    let mut ranges = Vec::new();

    for hit in &candidate.matched_lines {
        ranges.push(LineRange {
            start_line: hit.line.saturating_sub(5),
            end_line: hit.line + 8,
            reason: format!("matched query term: {}", hit.term),
        });
    }

    merge_overlapping_ranges(ranges)
}
```

---

# 超预算时怎么降级？

当最终上下文超过 budget，不要直接从后面砍掉。

应该逐级降级：

```text
FullFile
  ↓
FullSymbolBody
  ↓
FocusedSnippet
  ↓
SummaryAndSignature
  ↓
SignatureOnly
  ↓
RelationOnly
  ↓
Drop
```

但是有些内容不能随便降级。

例如：

```text
目标函数
错误映射
关键测试
用户明确点名的符号
```

这些可以标记为：

```rust
pub enum BudgetPriority {
    Required,
    High,
    Medium,
    Low,
}
```

`Required` 内容尽量不降级，除非极端超预算。

---

# 输出格式应该怎么变？

最终的 `code_context.md` 应该明确标注每个块的压缩级别。

例如：

````md
<code_context>

# Query
登录失败为什么返回 500？

# Context Budget
- budget_tokens: 12000
- used_tokens: 9340
- compression: enabled

# Core Code

## src/service/auth_service.rs:42-88
Symbol: AuthService::login
Compression: FullSymbolBody
Reason: target login method; returns AppError; DebugError intent

```rust
pub async fn login(&self, email: &str, password: &str) -> Result<Token, AppError> {
    // ...
}
````

## src/error.rs:30-58

Symbol: impl IntoResponse for AppError
Compression: FocusedSnippet
Reason: maps AppError to HTTP status

```rust
match self {
    AppError::Unauthorized => StatusCode::UNAUTHORIZED,
    AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
}
```

# Related APIs

## src/service/token_service.rs

Symbol: TokenService::issue
Compression: SignatureOnly
Reason: callee of AuthService::login, but not directly related to 500 error

```rust
pub async fn issue(&self, user: &User) -> Result<Token, AppError>;
```

# Tests

## tests/auth_login_test.rs:20-55

Compression: FullSymbolBody
Reason: tests wrong password behavior

```rust
#[tokio::test]
async fn rejects_wrong_password() {
    // ...
}
```

</code_context>

````

这样 AI 和你自己都能知道：

```text
这段是完整代码
这段是摘要
这段只是签名
这段为什么被放进来
````

---

# Debug 里要显示压缩过程

你的 `repoctx debug` 应该新增一段：

```text
Compression Trace:

1. AuthService::login
   original_tokens: 1320
   compressed_tokens: 1320
   level: FullSymbolBody
   reason:
   - MustInclude
   - exact target symbol
   - within core code budget

2. TokenService::issue
   original_tokens: 980
   compressed_tokens: 48
   level: SignatureOnly
   reason:
   - callee of target function
   - not directly related to error status
   - compressed to save budget

3. README.md auth section
   original_tokens: 2100
   compressed_tokens: 0
   level: Drop
   reason:
   - lower ranked than code/test chunks
   - DebugError intent prefers executable code
```

这会让你调试 token 使用非常方便。

---

# 新增 CLI

你现在可以加：

```bash
repoctx compress "登录失败为什么返回 500？" --budget 12000
```

输出压缩后的 block 列表：

```text
Compression Plan:

Budget: 12000
Estimated used: 9340

1. AuthService::login
   level: FullSymbolBody
   tokens: 1320

2. AppError
   level: FullSymbolBody
   tokens: 420

3. impl IntoResponse for AppError
   level: FocusedSnippet
   tokens: 310

4. TokenService::issue
   level: SignatureOnly
   tokens: 48

5. tests/auth_login_test.rs::rejects_wrong_password
   level: FullSymbolBody
   tokens: 760
```

然后：

```bash
repoctx build-context "登录失败为什么返回 500？" --show-compression
```

---

# Eval 要新增压缩指标

之前你评测：

```text
Recall@K
MRR
Noise Ratio
```

现在要加：

```text
Token Efficiency
Required Context Preservation
Compression Ratio
Dropped Required Items
Snippet Accuracy
```

例如：

```text
Case: auth_login_500

Before compression:
- candidates tokens: 31,200

After compression:
- context tokens: 9,340
- compression ratio: 70.1%

Required files preserved:
✅ src/service/auth_service.rs
✅ src/error.rs
✅ src/api/auth_handler.rs
✅ tests/auth_login_test.rs

Required symbols preserved:
✅ AuthService::login full body
✅ AppError full body

Problems:
- none
```

重点关注：

```text
压缩后有没有丢掉必须信息
压缩后 token 有没有明显下降
压缩后 AI 是否还能完成任务
```

---

# 最小可用版本

第一版不要做复杂摘要模型。
先用规则压缩。

你可以先实现这 5 个能力：

```text
1. token 估算
2. FullSymbolBody / SignatureOnly / FocusedSnippet / Drop
3. intent-specific compression policy
4. 超预算逐级降级
5. compression debug trace
```

第一版甚至不需要 AI summary。

因为你已有：

```text
符号签名
文件路径
行号
关系
测试
调用图
```

这些已经可以做很强的结构化压缩。

---

# 做完 Context Compression 后，下一步是什么？

做完后，下一步是：

# Patch Planner / 修改计划器

因为到那时，你的系统已经能：

```text
找到相关代码
排好优先级
压缩成高质量上下文
```

接下来就可以进入真正的 AI coding 环节：

```text
根据上下文生成修改计划
决定要改哪些文件
决定改动顺序
决定要跑哪些测试
生成 patch
验证 patch
```

但现在先不要跳到 patch。
当前目标是：

> **在有限 token 内，把最关键的代码、类型、错误、测试和关系保留下来。**

---

# 当前任务清单

你现在按顺序做：

```text
1. 定义 CompressionLevel
2. 定义 CompressionDecision
3. 实现 token estimator
4. 实现 focused snippet extractor
5. 实现 intent-specific compression policy
6. 实现 budget allocator
7. 实现超预算降级策略
8. build-context 接入 compressor
9. debug 输出 compression trace
10. eval 增加 compression metrics
```

---

一句话：

> **下一步是 Context Compression：让系统不只是知道哪些代码重要，还知道在 token 有限时该完整放、摘取放、签名放、摘要放，还是丢掉。**
下一步做：

# Patch Planner / 修改计划器

你前面已经做到：

```text
repo map
symbol index
chunks
build-context
eval
debug / explain / compare
query router
vector retriever
reranker
context compression
```

这些能力解决的是：

> 如何找到、排序、压缩最适合给 AI 的代码上下文。

下一步要解决的是：

> 拿到高质量上下文之后，系统应该如何决定“要改哪些文件、改哪些符号、按什么顺序改、改完跑哪些测试”。

这就是 **Patch Planner**。

---

## 它在系统里的位置

现在 pipeline 应该变成：

```text
User Query
   ↓
Query Router
   ↓
Hybrid Retrieval
   ↓
Reranker
   ↓
Context Compression
   ↓
Context Packer
   ↓
Patch Planner      ⬅️ 下一步
   ↓
Patch Generator
   ↓
Apply Patch
   ↓
Test / Verify
```

前面的系统负责：

```text
给 AI 正确上下文
```

Patch Planner 负责：

```text
把用户需求转成可执行的修改计划
```

---

# Patch Planner 具体干什么？

比如用户说：

```text
把登录失败时的 500 改成 401。
```

你的系统已经通过 context engine 找到了：

```text
src/service/auth_service.rs
src/error.rs
src/api/auth_handler.rs
tests/auth_login_test.rs
```

Patch Planner 接下来要生成：

```text
1. 修改 AuthService::login 中密码错误分支
2. 不要返回 AppError::Internal
3. 改成返回 AppError::Unauthorized 或 AppError::InvalidCredentials
4. 检查 AppError 到 HTTP status 的映射
5. 如果没有 401 映射，补充映射
6. 修改或新增登录失败测试
7. 跑 auth_login_test
```

也就是它不直接写代码，而是先产出一份 **修改计划**。

---

# 为什么不要直接生成 patch？

因为直接让 AI 根据 context 改代码，容易出现这些问题：

```text
改错文件
漏改测试
只改 service，忘了 error mapping
只改实现，忘了 trait
只改调用方，忘了所有引用
新增行为但没有测试
重构时漏掉 pub use / export
```

Patch Planner 的价值是：

> 在真正动代码前，先把修改范围、风险点、测试策略明确下来。

它能显著减少 AI coding 的盲改。

---

# Patch Planner 的输出应该长什么样？

建议输出两份：

```text
patch_plan.json  给程序用
patch_plan.md    给 AI / 人类读
```

---

## patch_plan.md 示例

````md
<patch_plan>

# Task
把登录失败时的 500 改成 401。

# Intent
ModifyBehavior

# Edit Scope

## Must Edit
1. src/service/auth_service.rs
   Symbol: AuthService::login
   Reason: 密码校验失败时当前可能返回 Internal error。

2. src/error.rs
   Symbol: AppError / IntoResponse for AppError
   Reason: 需要确认 Unauthorized / InvalidCredentials 映射到 HTTP 401。

3. tests/auth_login_test.rs
   Symbol: rejects_wrong_password
   Reason: 需要验证密码错误返回 401，而不是 500。

## Maybe Edit
1. src/api/auth_handler.rs
   Symbol: login_handler
   Reason: 如果 handler 层覆盖了错误响应，需要同步调整。

# Proposed Changes

## Change 1
File: src/service/auth_service.rs
Target: AuthService::login

Current behavior:
- 密码校验失败时返回 AppError::Internal 或等价内部错误。

Desired behavior:
- 密码校验失败时返回 AppError::Unauthorized 或 AppError::InvalidCredentials。

Edit instruction:
- 找到 password verify 失败分支。
- 不要把密码错误包装成 Internal。
- 返回明确的认证失败错误。

## Change 2
File: src/error.rs
Target: AppError HTTP mapping

Desired behavior:
- Unauthorized / InvalidCredentials 映射到 StatusCode::UNAUTHORIZED。

Edit instruction:
- 检查 IntoResponse / error_to_response / status_code 方法。
- 如果缺少 401 映射，添加它。

## Change 3
File: tests/auth_login_test.rs
Target: wrong password test

Desired behavior:
- wrong password should return HTTP 401。
- 不应该返回 500。
- 不应该 issue token。

# Test Plan
Run:
```bash
cargo test auth_login
cargo test rejects_wrong_password
````

# Risk Notes

* 不要把数据库错误也映射成 401。
* 只有认证失败应该返回 401。
* UserRepository 查询失败仍然应该是 500 或内部错误。
* 用户不存在和密码错误是否都返回同一种错误，需要遵循现有安全策略。

</patch_plan>

````

这份计划可以直接交给 AI，让它按计划生成 patch。

---

# patch_plan.json 示例

程序内部建议用结构化格式：

```json
{
  "task": "把登录失败时的 500 改成 401",
  "intent": "ModifyBehavior",
  "must_edit": [
    {
      "file_path": "src/service/auth_service.rs",
      "symbol": "AuthService::login",
      "reason": "password verification failure is handled here",
      "edit_type": "modify_behavior",
      "priority": "required"
    },
    {
      "file_path": "src/error.rs",
      "symbol": "AppError",
      "reason": "HTTP status mapping is defined here",
      "edit_type": "modify_error_mapping",
      "priority": "required"
    },
    {
      "file_path": "tests/auth_login_test.rs",
      "symbol": "rejects_wrong_password",
      "reason": "test should assert 401 instead of 500",
      "edit_type": "update_test",
      "priority": "required"
    }
  ],
  "maybe_edit": [
    {
      "file_path": "src/api/auth_handler.rs",
      "symbol": "login_handler",
      "reason": "handler may override error response",
      "edit_type": "inspect_or_modify",
      "priority": "medium"
    }
  ],
  "test_plan": [
    "cargo test auth_login",
    "cargo test rejects_wrong_password"
  ],
  "risk_notes": [
    "Do not map database errors to 401",
    "Only authentication failures should become 401",
    "Preserve existing behavior for repository failures"
  ]
}
````

---

# Patch Planner 要分几类任务处理

你的 Query Router 已经能识别 intent，所以 Patch Planner 应该按 intent 生成不同类型的修改计划。

---

## 1. ModifyBehavior

例如：

```text
把密码错误改成返回 401。
注册邮箱重复时返回 409。
修改 token 过期时间。
```

Patch Plan 应该包含：

```text
目标函数
错误类型
配置项
调用方
测试
行为风险
```

重点是：

```text
改行为 + 改测试
```

---

## 2. DebugError

例如：

```text
登录失败为什么返回 500？
duplicate key 是哪里来的？
```

Patch Plan 不一定马上改代码。

它可能先生成：

```text
diagnostic plan
```

例如：

```text
1. 检查错误产生点
2. 检查错误包装层
3. 检查错误映射层
4. 检查 handler 是否覆盖状态码
5. 添加回归测试
6. 再决定具体 patch
```

Debug 类任务常常需要先定位，再修改。

---

## 3. Refactor

例如：

```text
把 UserRepository::find_by_email 改名。
把 AuthService 拆开。
```

Patch Plan 必须包含：

```text
定义
所有引用
所有 impl
所有 trait method
所有测试
public export
迁移顺序
```

重构类最怕漏引用，所以计划里要有：

```text
references checklist
```

---

## 4. AddFeature

例如：

```text
新增 refresh token。
新增邮箱验证码登录。
```

Patch Plan 应该包含：

```text
相似功能参考
新增类型
新增 handler
新增 service method
新增 repository method
新增 error variant
新增 route
新增 tests
配置项
迁移文件
```

新增功能类不应该直接一上来生成代码，必须先明确项目分层模式。

---

## 5. Explain

例如：

```text
解释认证流程。
```

Patch Planner 可以跳过。

因为解释类任务不需要生成 patch。

所以 planner 应该返回：

```text
patch_required: false
```

---

# Rust 数据结构可以这样设计

```rust
#[derive(Debug, Clone)]
pub struct PatchPlan {
    pub task: String,
    pub intent: QueryIntent,
    pub patch_required: bool,

    pub must_edit: Vec<EditTarget>,
    pub should_inspect: Vec<InspectTarget>,
    pub maybe_edit: Vec<EditTarget>,

    pub proposed_changes: Vec<ProposedChange>,
    pub test_plan: TestPlan,
    pub risk_notes: Vec<String>,
    pub open_questions: Vec<String>,
}
```

---

## EditTarget

```rust
#[derive(Debug, Clone)]
pub struct EditTarget {
    pub file_path: String,
    pub symbol_id: Option<String>,
    pub qualified_name: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,

    pub edit_type: EditType,
    pub priority: EditPriority,
    pub reason: String,
}
```

---

## EditType

```rust
#[derive(Debug, Clone)]
pub enum EditType {
    ModifyBehavior,
    ModifyErrorMapping,
    AddErrorVariant,
    UpdateTest,
    AddTest,
    RenameSymbol,
    UpdateReferences,
    AddRoute,
    AddServiceMethod,
    AddRepositoryMethod,
    AddConfig,
    InspectOnly,
}
```

---

## ProposedChange

```rust
#[derive(Debug, Clone)]
pub struct ProposedChange {
    pub target: EditTarget,
    pub current_behavior: Option<String>,
    pub desired_behavior: String,
    pub instructions: Vec<String>,
    pub constraints: Vec<String>,
}
```

---

## TestPlan

```rust
#[derive(Debug, Clone)]
pub struct TestPlan {
    pub commands: Vec<String>,
    pub target_tests: Vec<String>,
    pub expected_behavior: Vec<String>,
}
```

---

# 第一版 Patch Planner 怎么实现？

第一版可以规则驱动，不需要复杂模型。

你已经有：

```text
query intent
ranked candidates
compressed context
symbol relations
chunk types
debug trace
```

所以可以根据这些生成计划。

---

## 规则示例：ModifyBehavior

如果 intent 是：

```text
ModifyBehavior
```

并且 ranked candidates 里有：

```text
method/function chunk
error chunk
test chunk
handler chunk
```

那么：

```text
最高分 function/method → must_edit
error mapping chunk → must_edit 或 should_inspect
test chunk → must_edit
handler chunk → should_inspect
callee helper → maybe_edit 或 inspect_only
```

---

## 规则示例：DebugError

如果 query 里有：

```text
500
401
403
panic
error
duplicate
failed
```

则优先计划：

```text
检查错误产生点
检查错误包装点
检查错误映射点
检查 handler
检查测试
```

如果找到错误映射：

```text
src/error.rs
impl IntoResponse for AppError
```

就标成：

```text
must_edit 或 should_inspect
```

---

## 规则示例：Refactor

如果 intent 是：

```text
Refactor
```

则：

```text
目标 symbol definition → must_edit
references → must_edit
implementations → must_edit
tests → must_edit
exports → should_inspect
docs/examples → maybe_edit
```

---

# 新增 CLI

你现在应该加：

```bash
repoctx plan-patch "把登录失败时的 500 改成 401"
```

输出：

```text
Patch Plan

Intent:
ModifyBehavior

Must edit:
1. src/service/auth_service.rs::AuthService::login
2. src/error.rs::AppError
3. tests/auth_login_test.rs::rejects_wrong_password

Should inspect:
1. src/api/auth_handler.rs::login_handler

Proposed changes:
1. Return Unauthorized instead of Internal when password verification fails.
2. Ensure Unauthorized maps to HTTP 401.
3. Update wrong-password test to expect 401.

Test plan:
- cargo test auth_login
- cargo test rejects_wrong_password

Risks:
- Do not convert repository/database failures into 401.
```

然后让：

```bash
repoctx build-context "把登录失败时的 500 改成 401" --with-patch-plan
```

在输出的 `code_context.md` 末尾附上：

```md
<patch_plan>
...
</patch_plan>
```

这样 AI 拿到的不只是上下文，还有明确的改动计划。

---

# Patch Planner 要接入 debug

你的 `repoctx debug` 应该新增：

```text
Patch Planning Trace:
```

例如：

```text
Patch Planning Trace:

1. AuthService::login
   decision: MustEdit
   reason:
   - top ranked target function
   - matches query term login
   - contains password verification
   - returns AppError

2. AppError
   decision: MustEdit
   reason:
   - error type returned by AuthService::login
   - HTTP status mapping affects 500 vs 401

3. tests/auth_login_test.rs
   decision: MustEdit
   reason:
   - test file related to AuthService::login
   - ModifyBehavior intent requires regression test

4. TokenService::issue
   decision: InspectOnly
   reason:
   - callee of AuthService::login
   - unrelated to wrong password status
```

这样你能解释：

```text
为什么这个文件要改
为什么那个文件不用改
```

---

# Evaluation 也要升级

现在 eval 不只评测“有没有找对上下文”，还要评测：

```text
有没有选对编辑目标
有没有包含必要测试
有没有避免无关修改
```

新增 eval case：

```json
{
  "id": "auth_wrong_password_401",
  "query": "把登录失败时的 500 改成 401",
  "must_edit_files": [
    "src/service/auth_service.rs",
    "src/error.rs",
    "tests/auth_login_test.rs"
  ],
  "should_inspect_files": [
    "src/api/auth_handler.rs"
  ],
  "must_not_edit_files": [
    "src/service/token_service.rs",
    "src/repository/user_repository.rs"
  ]
}
```

评测指标：

```text
Edit Target Recall
必须修改的文件是否都被选中

Edit Target Precision
是否选了太多不该改的文件

Test Inclusion Rate
是否包含相关测试

Must-not-edit Violation
是否错误建议修改无关文件
```

输出示例：

```text
Case: auth_wrong_password_401

Must edit:
✅ src/service/auth_service.rs
✅ src/error.rs
✅ tests/auth_login_test.rs

Should inspect:
✅ src/api/auth_handler.rs

Must not edit:
✅ src/service/token_service.rs not selected
✅ src/repository/user_repository.rs not selected

Metrics:
- edit target recall: 100%
- edit target precision: 75%
- test inclusion: yes
- must-not-edit violations: 0
```

---

# Patch Planner 和 Patch Generator 的区别

这两个不要混在一起。

## Patch Planner

回答：

```text
要改什么？
为什么改？
按什么顺序改？
哪些地方只检查不改？
要跑哪些测试？
有什么风险？
```

## Patch Generator

回答：

```text
具体代码怎么改？
diff 是什么？
patch 能不能应用？
```

现在做的是 Planner，不是 Generator。

---

# 当前任务清单

你现在按这个顺序实现：

```text
1. 定义 PatchPlan / EditTarget / ProposedChange / TestPlan
2. 根据 QueryIntent 写 planner profile
3. 根据 ranked candidates 推断 must_edit / should_inspect / maybe_edit
4. 根据关系图补齐 caller / callee / error / test
5. 生成 patch_plan.md 和 patch_plan.json
6. build-context 支持 --with-patch-plan
7. debug 输出 patch planning trace
8. eval 增加 edit target recall / precision
```

---

# 最小可用版本

第一版只需要支持三类任务：

```text
ModifyBehavior
DebugError
Refactor
```

因为这三类最容易受益。

最小命令：

```bash
repoctx plan-patch "把登录失败时的 500 改成 401"
```

最小输出：

```text
must_edit
should_inspect
maybe_edit
test_plan
risk_notes
```

这一步做好后，你的系统就从：

```text
能给 AI 上下文
```

升级成：

```text
能告诉 AI 应该怎么改
```

---

# 做完 Patch Planner 之后，下一步是什么？

下一步才是：

```text
Patch Generator / Diff Generator
```

也就是根据：

```text
code_context.md
patch_plan.md
```

生成真正的代码 diff。

但现在先不要跳到 Diff Generator。
先把修改计划做好。

一句话：

> **下一步是 Patch Planner：把检索到的代码上下文转成明确的修改范围、修改步骤、测试计划和风险说明。**
