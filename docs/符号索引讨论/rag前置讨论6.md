下一步做：

# Agent Tooling / MCP / IDE Integration

也就是：**把你的 repo context + patch 系统暴露成 AI 可以调用的工具层**。

你前面已经做完了本地闭环：

```text
repo map
symbol index
chunks
retrieval
rerank
compression
patch plan
patch generation
verify / repair
review
apply / commit / rollback
```

现在的问题变成：

> 这些能力怎么被 ChatGPT、Claude、Cursor、VS Code 插件、CLI agent 或你自己的 IDE 调用？

所以下一步是：

```text
Tool API Layer
```

更具体一点：

```text
先做 MCP Server
再做 IDE / CLI / HTTP 集成
```

MCP 官方定义里，server 可以向 AI client 暴露 **Resources、Prompts、Tools**：Resources 用于提供上下文和数据，Prompts 用于提供可复用工作流，Tools 用于让模型调用外部函数。你的 repo context 系统非常适合包装成 MCP server。([Model Context Protocol][1])

---

## 现在为什么该做工具层？

因为你已经不是只有一个命令了。

你现在有一整套能力：

```text
search_symbols
search_code
build_context
plan_patch
generate_patch
verify_patch
review_patch
apply_patch
rollback_patch
```

如果这些能力只在 CLI 里，人每次都要手动串起来。

但如果做成工具层，AI agent 就可以这样工作：

```text
用户：把登录失败的 500 改成 401

AI:
1. 调用 build_context
2. 调用 plan_patch
3. 调用 generate_patch
4. 调用 verify_patch
5. 如果失败，调用 repair_patch
6. 调用 review_patch
7. 等用户确认后，调用 apply_patch
```

这时你的系统就从：

```text
本地代码分析器
```

升级成：

```text
AI coding backend
```

---

# 下一步的核心目标

先不要急着做完整 IDE 插件。

你现在应该先做：

```text
repoctx server
```

它提供稳定的工具接口。

可以有三种入口：

```text
1. CLI tools
2. MCP server
3. HTTP API
```

推荐顺序：

```text
先 MCP stdio server
再 HTTP API
最后 IDE plugin
```

MCP 是为 AI 应用连接外部系统而设计的开放标准，官方文档把它描述为让 AI 应用连接本地文件、数据库、工具和工作流的方式。([Model Context Protocol][2]) Rust 方向可以直接参考官方 Rust SDK `rmcp`，它用于构建暴露 tools、resources、prompts 的 MCP server/client。([GitHub][3])

---

# 你要暴露哪些 tools？

不要一开始把所有能力都暴露给 AI。

先分成两类：

```text
Read-only tools
Write tools
```

---

## 第一类：Read-only tools

这些工具安全，优先做。

```text
repoctx.search_symbols
repoctx.search_code
repoctx.get_symbol
repoctx.get_file_summary
repoctx.get_references
repoctx.get_callers
repoctx.get_callees
repoctx.get_tests
repoctx.build_context
repoctx.explain_context
repoctx.debug_retrieval
```

这些工具只读仓库，不修改代码。

例如：

```json
{
  "name": "repoctx.search_symbols",
  "description": "Search indexed symbols in the current repository.",
  "input": {
    "query": "AuthService::login",
    "limit": 10
  }
}
```

返回：

```json
{
  "results": [
    {
      "qualified_name": "crate::service::auth_service::AuthService::login",
      "kind": "method",
      "file_path": "src/service/auth_service.rs",
      "start_line": 42,
      "end_line": 88,
      "signature": "pub async fn login(...) -> Result<Token, AppError>"
    }
  ]
}
```

这一步完成后，AI 就可以自己查仓库，而不是一次性等你塞一个巨大上下文。

---

## 第二类：Write tools

这些工具会修改代码，要谨慎暴露。

```text
repoctx.plan_patch
repoctx.generate_patch
repoctx.verify_patch
repoctx.repair_patch
repoctx.review_patch
repoctx.apply_patch
repoctx.rollback_patch
```

其中：

```text
plan_patch
generate_patch
verify_patch
review_patch
```

可以先开放。

但是：

```text
apply_patch
rollback_patch
commit_patch
```

必须有用户确认。

不要让 AI 自动调用 `apply_patch` 改真实仓库。

MCP 官方安全文档也强调，MCP 实现需要考虑授权、工具调用风险、数据访问和安全边界；官方规范也提醒 MCP 会打开任意数据访问和代码执行路径，所以实现者必须处理安全和信任问题。([Model Context Protocol][4])

---

# 你现在要设计 Tool Contract

这一步的重点不是写很多业务逻辑，而是把接口定清楚。

例如：

```text
repoctx.build_context
```

输入：

```json
{
  "query": "登录失败为什么返回 500？",
  "budget_tokens": 12000,
  "intent": "auto",
  "include_tests": true,
  "include_patch_plan": false
}
```

输出：

```json
{
  "run_id": "ctx_123",
  "intent": "DebugError",
  "context_markdown": "...",
  "selected_files": [
    "src/service/auth_service.rs",
    "src/error.rs",
    "tests/auth_login_test.rs"
  ],
  "debug_url": "repoctx://runs/ctx_123"
}
```

再比如：

```text
repoctx.plan_patch
```

输入：

```json
{
  "query": "把登录失败时的 500 改成 401",
  "context_run_id": "ctx_123"
}
```

输出：

```json
{
  "patch_plan_id": "plan_123",
  "must_edit": [
    "src/service/auth_service.rs",
    "src/error.rs",
    "tests/auth_login_test.rs"
  ],
  "should_inspect": [
    "src/api/auth_handler.rs"
  ],
  "test_plan": [
    "cargo test rejects_wrong_password"
  ],
  "risk_notes": [
    "Do not map repository/database failures to 401."
  ]
}
```

再比如：

```text
repoctx.apply_patch
```

输入必须带确认 token：

```json
{
  "patch_run_id": "patch_123",
  "mode": "new_branch",
  "branch_name": "ai/fix-login-401",
  "confirm": true
}
```

没有确认就拒绝。

---

# MCP 里可以怎么组织？

MCP 有三种概念很适合你：

```text
Tools     = 可执行动作
Resources = 可读取上下文
Prompts   = 固定工作流模板
```

官方 MCP 规范明确把 Resources、Prompts、Tools 作为 server 提供给 client 的能力分类。([Model Context Protocol][1])

你可以这样映射。

---

## Tools

```text
repoctx.search_symbols
repoctx.search_code
repoctx.build_context
repoctx.plan_patch
repoctx.generate_patch
repoctx.verify_patch
repoctx.review_patch
repoctx.apply_patch
repoctx.rollback_patch
```

---

## Resources

```text
repoctx://repo-map
repoctx://symbol-index
repoctx://runs/{run_id}
repoctx://patches/{patch_id}
repoctx://reviews/{review_id}
repoctx://files/{path}
```

AI 或 IDE 可以读取：

```text
当前 repo map
某次 context 构建结果
某次 patch plan
某次 verification report
某次 review report
```

---

## Prompts

你可以提供固定工作流：

```text
repoctx.prompts.debug_error
repoctx.prompts.modify_behavior
repoctx.prompts.refactor_symbol
repoctx.prompts.add_feature
repoctx.prompts.review_patch
```

例如：

```text
debug_error prompt:
1. build_context
2. explain likely root cause
3. plan_patch if user wants fix
4. generate_patch only after confirmation
```

这样 AI client 不需要自己发明流程。

---

# 你要加一个 Permission Layer

这是这一步最重要的安全设计。

工具按权限分级：

```text
Level 0: read-only
Level 1: generate artifacts only
Level 2: apply to temp worktree
Level 3: apply to real worktree
Level 4: commit / rollback
```

建议默认：

```text
Level 0 / Level 1 自动允许
Level 2 可以允许
Level 3 必须人工确认
Level 4 必须人工确认
```

也就是：

```text
search_symbols       自动允许
build_context        自动允许
plan_patch           自动允许
generate_patch       自动允许
verify_patch         自动允许，因为在 isolated worktree
apply_patch          需要确认
commit_patch         需要确认
rollback_patch       需要确认
```

不要把 `apply_patch` 设计成普通工具。它是危险工具。

---

# 你要实现的 server 架构

Rust 项目可以这样拆：

```text
server/
├── mcp.rs              # MCP server
├── http.rs             # optional HTTP API
├── tools/
│   ├── search.rs
│   ├── context.rs
│   ├── patch_plan.rs
│   ├── patch_generate.rs
│   ├── patch_verify.rs
│   ├── patch_review.rs
│   ├── patch_apply.rs
│   └── rollback.rs
├── resources.rs
├── prompts.rs
├── permissions.rs
├── sessions.rs
└── audit.rs
```

核心接口：

```rust
pub struct ToolRequest {
    pub tool_name: String,
    pub input_json: serde_json::Value,
    pub session_id: String,
    pub user_confirmed: bool,
}
```

```rust
pub struct ToolResponse {
    pub ok: bool,
    pub output_json: serde_json::Value,
    pub artifacts: Vec<ArtifactRef>,
    pub warnings: Vec<String>,
}
```

---

# 你要新增 Audit Log

只要 AI 可以调用工具，就必须记录。

记录：

```text
谁调用了什么工具
输入是什么
输出是什么
读了哪些文件
生成了哪些 patch
是否尝试修改仓库
是否被 permission layer 拦截
是否用户确认
```

表结构可以是：

```sql
CREATE TABLE tool_invocations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    permission_level TEXT NOT NULL,
    input_json TEXT NOT NULL,
    output_json TEXT,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

这样以后你可以回放：

```text
AI 为什么改了这个文件？
它调用了哪些工具？
是哪个 patch run 产生的？
谁确认 apply 的？
```

---

# 最小可用版本

第一版 MCP server 只做 5 个工具：

```text
repoctx.search_symbols
repoctx.search_code
repoctx.build_context
repoctx.plan_patch
repoctx.verify_patch
```

先不要开放：

```text
apply_patch
commit_patch
rollback_patch
```

第二版再开放：

```text
repoctx.generate_patch
repoctx.review_patch
```

第三版再开放：

```text
repoctx.apply_patch
repoctx.rollback_patch
```

而且必须带确认。

---

# CLI 入口

你现在可以新增：

```bash
repoctx serve --mcp
```

或者：

```bash
repoctx mcp
```

然后 AI client 可以连接这个本地 server。

也可以加：

```bash
repoctx serve --http --port 8765
```

给你自己的 IDE 插件或 Web UI 用。

---

# 这一步完成后的效果

完成后，你的系统就可以被 AI 这样调用：

```text
AI:
- 我先调用 repoctx.build_context 获取相关代码。
- 我发现这是 ModifyBehavior。
- 我调用 repoctx.plan_patch 生成修改计划。
- 我调用 repoctx.generate_patch 生成 diff。
- 我调用 repoctx.verify_patch 验证。
- 验证失败，我调用 repoctx.repair_patch。
- 验证通过后，我调用 repoctx.review_patch。
- 最后我请求用户确认是否 apply。
```

这才是真正的 agent workflow。

---

# 做完 Tooling / MCP 后，下一步是什么？

做完后，下一步是：

# IDE UX / Human-in-the-loop UI

也就是给人一个界面：

```text
查看 repo map
查看 context
查看 patch plan
查看 diff
查看 verification report
查看 review findings
一键 approve / reject / repair / apply / rollback
```

但现在先做工具层。

---

# 当前任务清单

你现在按这个顺序做：

```text
1. 定义 Tool API contract
2. 把核心能力拆成 read-only tools 和 write tools
3. 实现 permission layer
4. 实现 audit log
5. 实现 repoctx serve --mcp
6. 先暴露 search_symbols / search_code / build_context / plan_patch / verify_patch
7. 再暴露 generate_patch / review_patch
8. 最后谨慎暴露 apply_patch / rollback_patch
9. 增加 resources：repo-map、symbol-index、runs、patches、reviews
10. 增加 prompts：debug、modify、refactor、add-feature、review
```

一句话：

> **下一步是 Agent Tooling / MCP Server：把你已经做好的 repo context 和 patch workflow 包装成 AI 可调用的安全工具接口。**

[1]: https://modelcontextprotocol.io/specification/2025-06-18?utm_source=chatgpt.com "Specification"
[2]: https://modelcontextprotocol.io/docs/getting-started/intro?utm_source=chatgpt.com "What is the Model Context Protocol (MCP)?"
[3]: https://github.com/modelcontextprotocol/rust-sdk?utm_source=chatgpt.com "modelcontextprotocol/rust-sdk: The official Rust ..."
[4]: https://modelcontextprotocol.io/docs/tutorials/security/security_best_practices?utm_source=chatgpt.com "Security Best Practices"
下一步做：

# Human-in-the-loop UI / 人类审查界面

也就是：**给你的 repo context + patch agent 做一个人类可控的操作台**。

你前面已经有了：

```text
repo map
symbol index
Code RAG
query router
vector retriever
reranker
context compression
patch planner
patch generator
verify / repair
patch review
apply / commit / rollback
MCP / tool API
```

现在能力已经很完整了，但问题变成：

> 人怎么信任它？
> 人怎么审查它？
> 人怎么批准、拒绝、局部应用、回滚？
> 人怎么知道 AI 为什么选这些文件、为什么这么改？

所以下一步不是再加模型能力，而是做：

```text
Review Console / Human-in-the-loop UI
```

---

# 为什么这一步重要？

因为你的系统已经可以自动做很多事：

```text
检索代码
生成上下文
规划修改
生成 patch
自动 repair
验证测试
审查风险
应用补丁
```

能力越强，越需要一个清晰的控制界面。

否则用户只能看到一堆 CLI 输出：

```text
patch_run_123
verification_report.json
review_report.md
change.patch
```

这对开发者来说不够直观。

你需要让用户可以一眼看到：

```text
这次任务是什么
系统找了哪些文件
为什么认为这些文件相关
计划改哪些地方
实际 diff 是什么
测试有没有过
风险是什么
是否可以安全 apply
如何 rollback
```

这就是 Human-in-the-loop UI 的价值。

---

# 它在系统里的位置

完整 pipeline 现在应该变成：

```text
User Request
   ↓
MCP / CLI / IDE
   ↓
Context Engine
   ↓
Patch Planner
   ↓
Patch Generator
   ↓
Verify / Repair
   ↓
Patch Review
   ↓
Human Review UI     ⬅️ 下一步
   ↓
Approve / Reject / Apply / Rollback
```

前面的系统负责“自动化”。
这一步负责“可控性”和“信任”。

---

# 你要做的不是聊天 UI，而是变更审查 UI

不要一开始做一个普通 ChatGPT-like 聊天框。

你真正需要的是一个类似：

```text
AI Pull Request Review Console
```

它应该围绕一次 `patch_run` 展示所有证据。

---

# UI 应该有哪些页面？

第一版可以做 7 个核心页面。

---

## 1. Runs 页面

显示所有 AI 任务记录。

```text
Patch Runs

1. fix auth wrong password 401
   status: verified + reviewed
   decision: Approve
   branch: ai/auth-wrong-password-401
   tests: passed
   created: 2026-06-15 14:32

2. refactor UserRepository::find_by_email
   status: needs human review
   decision: NeedsHumanReview
   tests: failed
   created: 2026-06-15 13:20
```

用户可以点进去看详情。

---

## 2. Context 页面

展示这次任务系统选了哪些上下文。

需要显示：

```text
用户 query
query intent
retrieval plan
selected files
selected symbols
token budget
compression decisions
debug trace
```

例如：

```text
Query:
把登录失败时的 500 改成 401

Intent:
ModifyBehavior

Selected context:
✅ src/service/auth_service.rs
✅ src/error.rs
✅ src/api/auth_handler.rs
✅ tests/auth_login_test.rs

Why selected:
- AuthService::login: exact match + target behavior
- AppError: returned by login and controls HTTP mapping
- tests/auth_login_test.rs: regression test target
```

这个页面解决的是：

> AI 为什么看这些代码？

---

## 3. Patch Plan 页面

展示修改计划。

```text
Must edit:
- src/service/auth_service.rs::AuthService::login
- src/error.rs::AppError
- tests/auth_login_test.rs::rejects_wrong_password

Should inspect:
- src/api/auth_handler.rs::login_handler

Risks:
- 不要把数据库错误映射成 401
- 不要泄露用户是否存在
- 用户不存在和密码错误应该保持一致错误策略
```

这个页面解决的是：

> AI 准备怎么改？

---

## 4. Diff 页面

这是最重要的页面。

展示最终 diff，支持按文件、按 hunk 审查。

用户应该可以：

```text
查看完整 diff
展开 / 折叠文件
按 hunk approve / reject
只应用部分文件
只应用部分 hunk
查看每个 hunk 对应的 patch plan reason
查看该 hunk 是否通过测试覆盖
```

例如：

```text
src/service/auth_service.rs

@@ -52,7 +52,7 @@
 if !self.password_hasher.verify(password, &user.password_hash)? {
-    return Err(AppError::Internal(anyhow!("password mismatch")));
+    return Err(AppError::InvalidCredentials);
 }

Reason:
- Password mismatch should be authentication failure, not internal server error.
- Required by patch plan Change 1.

Review:
[Approve hunk] [Reject hunk] [Ask AI to revise]
```

这个页面解决的是：

> AI 实际改了什么？

---

## 5. Verification 页面

展示验证过程。

```text
Verification:

✅ git apply --check
✅ cargo fmt --check
✅ cargo check
✅ cargo test rejects_wrong_password
✅ cargo test auth_login

Repair attempts:
1 repair attempt
- compile failed because AppError::Unauthorized did not exist
- repair used existing AppError::InvalidCredentials
- verification passed after repair
```

用户应该能展开失败日志：

```text
compiler error
test failure
repair context
repair patch
```

这个页面解决的是：

> 这次修改真的能跑吗？

---

## 6. Review 页面

展示 Patch Review 结果。

```text
Decision:
ApproveWithNotes

Findings:
Medium:
- Test checks HTTP 401 but does not verify token/session absence.

Low:
- Handler was inspected but not modified.

Affected symbols:
- AuthService::login
- AppError::InvalidCredentials
- impl IntoResponse for AppError
- login_handler
```

这个页面解决的是：

> 这次修改有没有风险？

---

## 7. Apply / Rollback 页面

展示落地操作。

用户可以选择：

```text
Apply to current worktree
Apply to new branch
Apply to temporary worktree
Commit
Rollback
Open branch
Export patch
```

推荐默认操作：

```text
Apply to new branch
```

例如：

```text
Apply target:
branch: ai/auth-wrong-password-401

Actions:
[Apply to new branch]
[Apply without commit]
[Apply and commit]
[Export patch]
[Reject]
```

应用后显示：

```text
Applied:
✅ branch created: ai/auth-wrong-password-401
✅ commit created: 9f3a21b

Rollback:
repoctx rollback patch_run_123
```

这个页面解决的是：

> 人如何安全接受或撤销修改？

---

# 第一版做什么形态最好？

我建议第一版不要直接做 VS Code 插件。

先做：

```text
Local Web UI
```

也就是：

```bash
repoctx ui
```

启动本地服务：

```text
http://localhost:8765
```

原因：

```text
diff 展示容易
日志展示容易
交互按钮容易
后面可以复用给 IDE
和 Rust 后端集成简单
可以直接读取 patch_runs 数据库
```

后端可以用：

```text
Rust + axum
```

前端可以用你熟悉的任意方案。

如果你想保持 Rust 技术栈，也可以后面做：

```text
Tauri desktop app
```

但 MVP 用 local web UI 最快。

---

# 也可以先做 TUI

如果你不想做前端，可以先做终端 UI：

```bash
repoctx review-tui patch_run_123
```

用法类似：

```text
↑ ↓ 选择文件
Enter 展开 diff
a approve hunk
r reject hunk
v 查看 verification
p 查看 patch plan
q 退出
```

Rust 里可以用 TUI 方案实现。

但从长期看，diff 审查和日志浏览更适合 Web UI。

---

# UI 后端 API 需要哪些？

你已经有 MCP / Tool API，但 UI 最好有自己的 HTTP API。

第一版 endpoints：

```text
GET  /api/runs
GET  /api/runs/{run_id}
GET  /api/runs/{run_id}/context
GET  /api/runs/{run_id}/plan
GET  /api/runs/{run_id}/diff
GET  /api/runs/{run_id}/verification
GET  /api/runs/{run_id}/review

POST /api/runs/{run_id}/approve
POST /api/runs/{run_id}/reject
POST /api/runs/{run_id}/request-repair
POST /api/runs/{run_id}/apply
POST /api/runs/{run_id}/commit
POST /api/runs/{run_id}/rollback
```

这些 API 不需要重新实现业务逻辑。
它们只是调用你已有的模块：

```text
patch store
verification report
review report
apply workflow
rollback workflow
```

---

# 需要新增哪些状态？

你现在要给 `patch_run` 加状态机。

例如：

```rust
pub enum PatchRunStatus {
    ContextBuilt,
    PatchPlanned,
    PatchGenerated,
    VerificationFailed,
    VerificationPassed,
    Repairing,
    ReviewPassed,
    NeedsHumanReview,
    Rejected,
    Approved,
    Applied,
    Committed,
    RolledBack,
}
```

UI 上每个 run 都显示当前状态。

---

# Human Actions 要记录

任何人工操作都要记录。

例如：

```text
用户 approve 了哪个 hunk
用户 reject 了哪个文件
用户要求 AI repair
用户确认 apply
用户创建 branch
用户 rollback
```

建议加表：

```sql
CREATE TABLE human_actions (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    target TEXT,
    note TEXT,
    created_at INTEGER NOT NULL
);
```

`action_type` 可以是：

```text
approve_run
reject_run
approve_file
reject_file
approve_hunk
reject_hunk
request_repair
apply_patch
commit_patch
rollback_patch
```

这样以后你可以追踪：

```text
这次修改是谁批准的？
哪些 hunk 被人工拒绝？
为什么重新 repair？
```

---

# UI 里最关键的是“解释链”

不要只显示结果。

每个主要决策都应该能展开看到原因：

```text
为什么这个文件进了 context？
为什么这个符号是 must_edit？
为什么这个 patch 被 repair？
为什么 review 认为风险低？
为什么允许 apply？
```

这可以直接复用你前面做的：

```text
retrieval trace
rerank trace
compression trace
patch planning trace
verification trace
review findings
```

你的 UI 应该把它们串成一条链：

```text
Query
  ↓
Retrieval reason
  ↓
Patch plan reason
  ↓
Diff hunk reason
  ↓
Verification result
  ↓
Review decision
  ↓
Human decision
```

这就是信任感来源。

---

# 第一版 UI 最小功能

不要做太大。

MVP 只需要 5 个页面：

```text
1. Runs list
2. Run detail summary
3. Context + Plan
4. Diff + Review
5. Apply / Rollback
```

MVP 操作只需要：

```text
Approve run
Reject run
Request repair
Apply to new branch
Rollback
```

暂时不要做：

```text
按 hunk 局部 approve
在线编辑 patch
多人协作
评论系统
权限系统
PR 创建
```

这些可以后面做。

---

# CLI 也要配合 UI

新增命令：

```bash
repoctx ui
```

打开本地 UI。

```bash
repoctx open patch_run_123
```

在浏览器打开某次 run。

```bash
repoctx runs
```

列出历史 run。

```bash
repoctx show-run patch_run_123
```

终端查看详情。

---

# 和 MCP 的关系

MCP 是给 AI 调用的。

UI 是给人控制的。

它们的关系是：

```text
AI client 通过 MCP 调工具
        ↓
产生 patch_run
        ↓
UI 展示 patch_run
        ↓
人 approve / reject / apply
```

也就是说：

```text
MCP = agent interface
UI = human control interface
```

两个都需要，但职责不同。

---

# 做完 UI 后，下一步是什么？

Human-in-the-loop UI 做完后，下一步才是：

# Team / Multi-repo / Policy Layer

也就是把单人本地工具升级成团队可用系统：

```text
多 repo 管理
用户权限
项目策略
审批策略
安全策略
CI 集成
PR 集成
审计日志
共享 eval
```

但现在先不要跳到团队版。

当前目标是：

> **让人能够看懂、审查、批准、拒绝、应用和回滚 AI 生成的 patch。**

---

# 当前任务清单

你现在按这个顺序做：

```text
1. 定义 patch run 状态机
2. 保存所有 run artifacts
3. 实现 repoctx ui 本地 Web 服务
4. 做 Runs list 页面
5. 做 Run detail 页面
6. 做 Context / Plan 页面
7. 做 Diff / Verification / Review 页面
8. 做 Approve / Reject / Request Repair 操作
9. 做 Apply to new branch 操作
10. 做 Rollback 操作
11. 记录 human_actions
12. 把 retrieval / patch / verify / review trace 串起来展示
```

一句话：

> **下一步是 Human-in-the-loop UI：把你的 AI 代码修改系统做成一个可审查、可批准、可回滚的本地 Review Console。**
下一步已经不是再“加一层能力”了，而是：

# System Integration / 系统整合与产品化

也就是：**把你现在这套 AI codebase 引擎，从“模块集合”变成“稳定系统”**。

你现在已经有：

```text
理解仓库（repo map + symbol index）
检索（hybrid retrieval）
排序（reranker）
压缩（context compression）
规划（patch planner）
生成（patch generator）
验证（verify + repair）
审查（review）
执行（apply / rollback）
工具层（MCP / API）
人机界面（UI）
```

如果继续往下加“新功能”，只会变复杂，不会变强。

---

# 所以下一步只有一个核心目标：

> **让整个系统稳定运行、可复现、可扩展、可交付。**

---

# 具体要做什么？

我帮你收束成 4 件真正关键的事：

---

# 1. Pipeline Orchestration（流水线编排）

现在你的系统是很多模块拼起来的：

```text
search → plan → generate → verify → review → apply
```

下一步必须做：

```text
统一执行引擎（Pipeline Engine）
```

### 目标：

```text
一个 run_id 贯穿整个生命周期
所有步骤可重放
所有步骤可中断
所有步骤可恢复
```

---

### 你要实现一个：

```rust
PipelineStateMachine
```

例如：

```text
Created
  ↓
ContextBuilt
  ↓
Planned
  ↓
Generated
  ↓
Verified
  ↓
Repaired
  ↓
Reviewed
  ↓
Applied
  ↓
Committed
  ↓
Finished
```

每一步都：

```text
可 replay
可 debug
可 audit
```

---

# 2. Reproducibility（可复现性）

现在最大的问题是：

> 同一个 query，多跑几次结果可能不一样。

所以下一步必须加：

```text
deterministic run mode
```

你需要记录：

```text
repo snapshot hash
symbol index version
embedding model version
retrieval weights
query router config
LLM model version
temperature / seed
chunking strategy version
```

---

### 目标：

```bash
repoctx replay run_123
```

必须得到：

```text
完全一致的 context / plan / patch
```

---

# 3. Incremental Indexing（增量索引）

你现在大概率是：

```text
全量 repo scan
全量 symbol rebuild
全量 embedding
```

下一步必须升级：

```text
只处理变更
```

---

### 你要实现：

```text
git diff aware indexing
```

每次：

```text
只重新解析 changed files
只更新 affected symbols
只重算 affected chunks
只更新 embeddings
只更新 graph edges
```

---

### 目标：

```text
repo update 10x faster
```

否则系统规模一大就崩。

---

# 4. Performance + Cost Control（性能与成本控制）

你现在已经有：

```text
symbol search
full-text
graph
vector
LLM planning
LLM patch generation
LLM review
LLM repair
```

下一步必须控制：

## 你要加三个预算系统：

---

### ① Token Budget Controller

```text
context budget
generation budget
repair budget
review budget
```

---

### ② Latency Budget

```text
symbol search < 50ms
context build < 300ms
rerank < 100ms
vector search < 200ms
LLM call < configurable
```

---

### ③ Cost Budget

```text
每个 run cost tracking
每个 user cost tracking
每个 repo cost tracking
```

---

### 目标：

```text
可以知道：
- 这个 patch 花了多少钱
- 哪一步最贵
- 哪一步最慢
```

---

# 你现在的阶段本质是：

你已经不是在做“功能”了，而是在做：

```text
AI Code Execution System
```

下一步不是加模块，而是：

> 把系统变成“工程级可靠基础设施”。

---

# 如果你要一句话总结下一步

> 下一步是：**把所有能力串成一个可复现、可调度、可观测、可优化的统一执行系统（Pipeline + Observability + Cost Control + Incremental Indexing）**

---

# 最小下一步（真正要做的第一件事）

如果只能选一个，我建议你先做这个：

```text
Pipeline Engine + Run State Machine
```

因为它会统一所有东西：

```text
context / patch / verify / review / apply
```

全部都会变成：

```text
pipeline steps
```

---

# 做完这一层之后，你的系统会变成什么？

会从：

```text
一堆 AI 工具
```

变成：

```text
AI Codebase Operating System
```

---

# 下一步之后才会出现的新阶段（但你还没到）

先给你提前看一下后面路线：

```text
1. multi-agent collaboration
2. distributed repo indexing
3. team-level policy engine
4. PR/CI integration
5. IDE-native deep integration
6. autonomous refactoring loops
```

但这些现在都不该做。

---

# 当前你真正该做的清单

```text
1. Pipeline Engine（统一状态机）
2. Run ID 全链路追踪
3. Replay system（可复现）
4. Incremental indexing（git diff aware）
5. Budget system（token/latency/cost）
6. Observability（trace + metrics + logs）
```

---

# 一句话结束

> 下一步不是“再做一个功能”，而是把你现有所有能力统一成一个可控的执行系统（Pipeline + Reproducibility + Incremental + Budget + Observability）。
