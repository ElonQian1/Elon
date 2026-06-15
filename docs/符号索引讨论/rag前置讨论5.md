下一步做：

# Patch Review / Change Review

也就是：**最终补丁审查层**。

你前面已经做到：

```text
生成 patch
        ↓
隔离 apply
        ↓
format / check / test
        ↓
失败自动 repair
        ↓
得到一个 verified patch
```

现在下一步要解决的是：

> patch 虽然能编译、测试也过了，但它是不是“应该被接受”？
> 它有没有改多？有没有越界？有没有破坏设计？有没有测试覆盖不足？有没有安全风险？

所以现在要做 **Patch Review**。

---

# 它在系统里的位置

你的 pipeline 现在应该变成：

```text
User Query
   ↓
Context Engine
   ↓
Patch Planner
   ↓
Patch Generator
   ↓
Verification & Repair
   ↓
Patch Review        ⬅️ 下一步
   ↓
Human Approval / Auto Apply
```

前面的 verification 关注：

```text
能不能 apply
能不能 format
能不能 compile
测试能不能过
```

Patch Review 关注：

```text
该不该接受
有没有越界修改
有没有潜在风险
是否符合 patch_plan
测试是否足够
diff 是否合理
是否需要人工确认
```

---

# 为什么测试通过还需要 Review？

因为测试通过不代表 patch 一定好。

它可能：

```text
改了不该改的文件
为了通过测试硬改测试
把错误都映射成 401
引入过宽的 fallback
删除了重要逻辑
改动范围太大
绕过了原有抽象
把业务错误和系统错误混在一起
破坏 public API
引入安全问题
```

比如用户说：

```text
把登录失败时的 500 改成 401
```

一个坏 patch 可能会这样做：

```rust
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
```

它可能让测试过了，但它把所有错误都变成了 401。
Verification 可能发现不了这个语义风险，但 Patch Review 应该发现。

---

# Patch Review 要审什么？

你可以把它分成 8 类检查。

---

## 1. Plan Compliance

检查 patch 是否符合 `patch_plan`。

问题：

```text
patch 有没有修改 must_edit 文件？
patch 有没有漏掉 must_edit 文件？
patch 有没有修改计划外文件？
patch 有没有做计划外重构？
patch 是否满足 proposed_changes？
```

例如：

```text
Plan 说要改：
- AuthService::login
- AppError mapping
- tests/auth_login_test.rs

Patch 实际改：
- AuthService::login
- tests/auth_login_test.rs
- TokenService::issue

Review 应该警告：
TokenService::issue 不在计划内，可能是越界修改。
```

---

## 2. Scope Control

检查改动范围是否过大。

需要看：

```text
改了多少文件
加了多少行
删了多少行
是否格式化了整文件
是否夹带无关改动
是否修改 vendor/generated/lock 文件
```

例如：

```text
用户只要求改一个错误码，但 patch 改了 12 个文件。
```

这应该被标记为：

```text
needs_human_review
```

---

## 3. Semantic Risk

检查语义是否可能错。

比如：

```text
是否把所有错误都映射成同一个状态码
是否吞掉真实错误
是否把数据库错误当成用户错误
是否改变了 public API
是否改变 trait contract
是否改变并发/事务/缓存语义
是否改变鉴权逻辑
```

这类检查很难完全靠规则，可以用规则 + LLM reviewer。

---

## 4. Test Adequacy

检查测试是否足够。

问题：

```text
有没有新增或更新相关测试？
测试是否覆盖用户要求的行为？
测试是不是只改了期望值，但业务代码没真正改？
有没有只删除失败测试？
有没有降低断言强度？
```

比如：

```text
用户要求 wrong password 返回 401。
```

Review 应该确认测试里确实有：

```text
wrong password
expect 401
no token issued
```

而不是只检查：

```text
response is not 500
```

---

## 5. Regression Risk

检查可能影响哪些地方。

根据 symbol graph 看：

```text
修改的函数被谁调用？
修改的错误类型影响哪些 handler？
修改 trait 是否影响所有 impl？
修改 config 是否影响启动路径？
```

输出：

```text
Potentially affected:
- login_handler
- register_handler
- auth middleware
- auth_login_test
- user_register_test
```

这一步可以复用你的 symbol relation graph。

---

## 6. Security / Safety Review

代码系统里这一步很重要。

检查：

```text
是否泄露错误细节
是否把认证失败和系统失败混淆
是否绕过权限检查
是否硬编码 token / secret
是否扩大了输入信任边界
是否打印敏感信息
是否降低密码/hash/token 校验
是否引入 SQL 拼接
```

例如登录错误场景里，Review 应该提醒：

```text
用户不存在和密码错误是否应该返回同一种错误？
是否暴露了用户枚举信息？
```

---

## 7. Style / Architecture Review

检查 patch 是否符合项目风格。

比如：

```text
是否绕过 service/repository 分层
是否重复已有 helper
是否新增了和现有错误类型重复的 enum variant
是否破坏 module boundary
是否使用了项目不常用的 crate 或模式
是否把业务逻辑塞进 handler
```

这一步能防止 AI 生成“能跑但不像这个项目”的代码。

---

## 8. Review Decision

最后必须给出决策。

建议用四种：

```text
Approve
ApproveWithNotes
NeedsHumanReview
Reject
```

含义：

```text
Approve:
  patch 符合计划，测试通过，风险低，可以应用。

ApproveWithNotes:
  patch 可接受，但有注意事项。

NeedsHumanReview:
  patch 可能正确，但风险较高，需要人看。

Reject:
  patch 不符合计划、越界、测试不足或有明显语义问题。
```

---

# 输出应该长什么样？

建议输出两份：

```text
review_report.json
review_report.md
```

---

## review_report.md 示例

```md
<patch_review>

# Decision
NeedsHumanReview

# Summary
Patch passed verification, but it changes error mapping in a way that may affect non-authentication errors.

# Plan Compliance
✅ Modified required file: src/service/auth_service.rs
✅ Modified required file: src/error.rs
✅ Modified required test: tests/auth_login_test.rs
⚠️ Also modified src/service/token_service.rs, which was not in the patch plan.

# Verification
✅ git apply --check
✅ cargo fmt --check
✅ cargo check
✅ cargo test rejects_wrong_password

# Findings

## High
1. Possible over-broad error mapping
File: src/error.rs
Reason:
The patch appears to map multiple AppError variants to 401. The task only requires wrong-password authentication failures to return 401.

Recommendation:
Only map InvalidCredentials / Unauthorized to 401. Keep Internal errors as 500.

## Medium
2. Plan scope violation
File: src/service/token_service.rs
Reason:
This file was not listed in must_edit or maybe_edit.

Recommendation:
Remove this change unless it is required and update patch_plan accordingly.

## Low
3. Test should assert no token is issued
File: tests/auth_login_test.rs
Reason:
The test checks status code 401 but does not verify that no token/session is created.

# Affected Symbols
- AuthService::login
- AppError::InvalidCredentials
- impl IntoResponse for AppError
- login_handler

# Recommendation
Do not auto-apply. Ask for human review or generate a narrower repair patch.

</patch_review>
```

---

# Rust 数据结构

可以这样定义。

```rust
pub struct PatchReview {
    pub patch_run_id: String,
    pub decision: ReviewDecision,
    pub summary: String,

    pub plan_compliance: PlanComplianceReport,
    pub verification_summary: VerificationSummary,
    pub findings: Vec<ReviewFinding>,
    pub affected_symbols: Vec<String>,
    pub recommended_actions: Vec<String>,
}
```

```rust
pub enum ReviewDecision {
    Approve,
    ApproveWithNotes,
    NeedsHumanReview,
    Reject,
}
```

```rust
pub struct ReviewFinding {
    pub severity: Severity,
    pub category: ReviewCategory,
    pub file_path: Option<String>,
    pub symbol: Option<String>,
    pub message: String,
    pub evidence: Vec<String>,
    pub recommendation: String,
}
```

```rust
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}
```

```rust
pub enum ReviewCategory {
    PlanCompliance,
    ScopeControl,
    SemanticRisk,
    TestAdequacy,
    RegressionRisk,
    Security,
    Architecture,
    Style,
}
```

```rust
pub struct PlanComplianceReport {
    pub required_files_touched: Vec<String>,
    pub required_files_missing: Vec<String>,
    pub unexpected_files_touched: Vec<String>,
    pub forbidden_files_touched: Vec<String>,
    pub must_edit_coverage: f32,
}
```

---

# Review 怎么实现？

第一版可以规则驱动，后面再加 LLM reviewer。

---

## 第一层：规则检查

这些不需要 AI。

```text
是否改了 forbidden files
是否漏掉 must_edit files
是否改了计划外文件
diff 是否过大
是否删除测试
是否降低测试断言
是否修改 Cargo.lock / package files
是否修改 generated/vendor
是否有 TODO / unwrap / panic 新增
是否新增 unsafe
是否新增敏感日志
```

例如：

```rust
pub fn review_scope(
    patch: &PatchSet,
    plan: &PatchPlan,
) -> Vec<ReviewFinding> {
    let mut findings = Vec::new();

    for file in &patch.touched_files {
        if plan.forbidden_files.contains(file) {
            findings.push(ReviewFinding {
                severity: Severity::Critical,
                category: ReviewCategory::ScopeControl,
                file_path: Some(file.clone()),
                symbol: None,
                message: "Patch modified a forbidden file".to_string(),
                evidence: vec![file.clone()],
                recommendation: "Reject this patch and regenerate within allowed files".to_string(),
            });
        }
    }

    findings
}
```

---

## 第二层：图关系检查

利用 symbol graph。

检查：

```text
改了 public function，有没有调用方测试？
改了 trait，有没有所有 impl？
改了 error enum，有没有 error mapping？
改了 handler，有没有 route/test？
改了 repository trait，有没有 mock/test impl？
```

例如：

```text
Patch modified UserRepository trait
但没有修改 MockUserRepository
```

Review 应该报：

```text
High: trait implementation may be incomplete.
```

---

## 第三层：LLM Review

这一步可以可选。

把这些喂给 reviewer：

```text
用户任务
patch_plan
final diff
verification report
affected symbols
compressed context
```

让它输出结构化 review findings。

注意：LLM Review 不能替代规则检查。
它适合发现语义风险，不适合做 hard policy。

---

# 新增 CLI

加这些命令：

```bash
repoctx review change.patch --plan patch_plan.json
```

输出 review report。

也可以集成到完整流程：

```bash
repoctx patch "把登录失败时的 500 改成 401" \
  --verify \
  --repair 2 \
  --review
```

输出：

```text
Patch:
✅ generated
✅ verified
⚠️ review decision: NeedsHumanReview

Findings:
- High: possible over-broad error mapping in src/error.rs
- Medium: unexpected file touched src/service/token_service.rs

Final:
Not auto-applied.
```

如果 review 通过：

```text
Patch:
✅ generated
✅ verified
✅ reviewed

Decision:
Approve

Final:
Ready to apply.
```

---

# Review 通过后怎么办？

你可以定义 apply policy：

```text
Approve:
  可以自动 apply，或者提示用户确认。

ApproveWithNotes:
  默认需要用户确认。

NeedsHumanReview:
  不自动 apply。

Reject:
  自动进入 repair 或重新生成 patch。
```

例如：

```rust
pub enum ApplyPolicy {
    AutoApplyIfApproved,
    AlwaysAsk,
    NeverAutoApply,
}
```

我建议默认：

```text
AlwaysAsk
```

也就是即使 review approve，也让用户确认。

---

# Debug Trace 要加 Review

你的 debug 输出现在应该包含：

```text
Review Trace:
```

示例：

```text
Review Trace:

1. Plan compliance
   ✅ all must_edit files touched
   ✅ no forbidden files touched
   ⚠️ 1 unexpected file touched: src/service/token_service.rs

2. Test adequacy
   ✅ related test updated
   ⚠️ test only checks status code, not token/session side effect

3. Semantic risk
   ⚠️ AppError mapping may be too broad

Decision:
NeedsHumanReview
```

---

# Eval 也要升级

现在 eval 要加 review 指标。

```text
Review Accuracy
review 是否能发现坏 patch

False Reject Rate
好 patch 被误拒的比例

False Approve Rate
坏 patch 被放过的比例

Scope Violation Detection
是否发现越界修改

Test Adequacy Detection
是否发现测试不足

Security Finding Rate
是否发现安全风险
```

你可以做一些“坏 patch fixtures”：

```text
1. 把所有错误都改成 401
2. 删除失败测试
3. 修改计划外文件
4. 使用 unwrap 绕过错误处理
5. 新增敏感日志
6. 修改 trait 但漏 impl
```

然后看 review 能不能拦住。

---

# 最小可用版本

第一版 Patch Review 先做这些：

```text
1. plan compliance check
2. allowed / forbidden files check
3. diff size check
4. must_edit coverage check
5. test changed / deleted check
6. unexpected files warning
7. affected symbols summary
8. final decision
```

先别上复杂 LLM review。

最小命令：

```bash
repoctx review change.patch --plan patch_plan.json --verification verification_report.json
```

最小输出：

```text
Decision:
Approve / NeedsHumanReview / Reject

Findings:
- severity
- category
- file
- message
- recommendation
```

---

# 做完 Patch Review 后，下一步是什么？

做完之后，下一步就是：

# Apply / Commit / Rollback Workflow

也就是把已经：

```text
generated
verified
reviewed
```

的 patch 安全应用到用户仓库。

包括：

```text
人工确认
apply 到当前工作区
生成 commit message
创建 git branch
支持 rollback
保存 patch run record
和 IDE / MCP 集成
```

但现在先做 Patch Review。

---

# 当前任务清单

你现在按顺序做：

```text
1. 定义 PatchReview / ReviewFinding / ReviewDecision
2. 实现 plan compliance checker
3. 实现 scope checker
4. 实现 diff size checker
5. 实现 test adequacy checker
6. 实现 affected symbol analyzer
7. 根据 findings 生成 review decision
8. 输出 review_report.md / review_report.json
9. repoctx patch 接入 --review
10. eval 加 bad patch fixtures
```

一句话：

> **下一步是 Patch Review：在 patch 已经验证通过之后，检查它是否符合计划、范围是否合理、测试是否足够、语义是否有风险，并决定能否应用。**
下一步做：

# Apply / Commit / Rollback Workflow

也就是：**安全落地补丁的工作流**。

你前面已经做到：

```text
生成 patch
    ↓
验证 patch
    ↓
自动 repair
    ↓
review patch
    ↓
得到一个可接受的 verified + reviewed patch
```

现在下一步要解决的是：

> 这个 patch 怎么安全地应用到用户真实仓库？
> 怎么创建分支？
> 怎么提交 commit？
> 怎么回滚？
> 怎么保留完整运行记录？
> 怎么避免污染用户工作区？

这一步是把系统从：

```text
AI 代码修改器
```

升级成：

```text
AI 代码变更管理器
```

---

# 现在的 pipeline

你现在应该进入这一层：

```text
User Query
   ↓
Context Engine
   ↓
Patch Planner
   ↓
Patch Generator
   ↓
Verification & Repair
   ↓
Patch Review
   ↓
Apply / Commit / Rollback   ⬅️ 下一步
```

前面所有步骤都还在“准备变更”。

这一层开始真正影响用户仓库。

所以它必须非常谨慎。

---

# 这一步的核心目标

你要实现一个安全流程：

```text
1. 检查当前工作区是否干净
2. 创建独立 branch 或 worktree
3. 应用 verified patch
4. 再跑一次轻量 verify
5. 生成 commit message
6. 让用户确认
7. commit
8. 保存 patch run 记录
9. 支持 rollback
```

也就是说，不要直接：

```bash
git apply change.patch
```

而是要有完整保护。

---

# 为什么需要这一层？

因为就算 patch 已经 verified/reviewed，真实 apply 时仍然可能遇到：

```text
用户工作区有未提交改动
main 分支已经变了
patch 基于旧 hash
文件被用户手动改过
patch apply 到真实 repo 失败
用户想撤销
用户想只应用部分文件
用户想先看 diff
用户想创建 branch 而不是直接改当前分支
```

所以必须有：

```text
pre-apply check
apply mode
commit workflow
rollback workflow
```

---

# 你要支持的 apply 模式

建议先支持 4 种模式。

---

## 1. Dry Run

只检查，不修改真实仓库。

```bash
repoctx apply change.patch --dry-run
```

输出：

```text
✅ patch can be applied
✅ touched files allowed
✅ current worktree clean
✅ file hashes match
```

这是默认最安全模式。

---

## 2. Apply to Current Worktree

直接应用到当前工作区，但不 commit。

```bash
repoctx apply change.patch
```

适合用户想自己检查。

应用前要确认：

```text
当前工作区是否干净
patch 是否 verified
patch 是否 reviewed
用户是否确认
```

---

## 3. Apply to New Branch

推荐默认模式。

```bash
repoctx apply change.patch --branch ai/fix-login-401
```

流程：

```text
git checkout -b ai/fix-login-401
git apply change.patch
cargo fmt
cargo test targeted
```

这样不会污染原分支。

---

## 4. Apply to Temporary Worktree

更安全。

```bash
repoctx apply change.patch --worktree
```

流程：

```text
创建临时 worktree
在临时 worktree 应用 patch
验证通过后，再问用户是否合并/复制/创建 branch
```

这个适合 agent 自动化。

---

# 推荐默认策略

我建议默认这样：

```text
默认不直接改当前分支
默认创建新 branch
默认应用前要求确认
默认保存 patch run record
默认支持 rollback
```

也就是：

```bash
repoctx patch "把登录失败时的 500 改成 401" --apply
```

默认行为等价于：

```bash
repoctx patch "..." \
  --verify \
  --repair 2 \
  --review \
  --branch ai/auth-wrong-password-401 \
  --confirm
```

---

# 新增 CLI

你现在可以加这些命令：

```bash
repoctx apply change.patch --dry-run
```

```bash
repoctx apply change.patch --branch ai/fix-login-401
```

```bash
repoctx commit --run patch_run_123
```

```bash
repoctx rollback patch_run_123
```

```bash
repoctx runs
```

```bash
repoctx show-run patch_run_123
```

最终的一键命令：

```bash
repoctx patch "把登录失败时的 500 改成 401" \
  --verify \
  --repair 2 \
  --review \
  --apply \
  --branch ai/fix-login-401
```

---

# Apply 前必须检查什么？

你要做一个 `PreApplyCheck`。

至少检查：

```text
1. 当前目录是不是 git repo
2. 当前 branch 是什么
3. 工作区是否干净
4. 是否有 staged changes
5. patch 是否基于当前文件 hash
6. patch 是否已经通过 verification
7. patch 是否通过 review
8. touched files 是否仍然允许
9. 是否存在冲突风险
```

输出类似：

```text
Pre-apply checks:

✅ git repository detected
✅ current branch: main
✅ working tree clean
✅ patch verified
✅ patch reviewed: Approve
✅ touched files allowed
✅ file hashes match

Ready to apply.
```

如果工作区不干净：

```text
❌ working tree has uncommitted changes

Changed files:
- src/service/auth_service.rs
- README.md

Recommended:
- commit/stash your changes
- or use --worktree
```

不要强行 apply。

---

# File Hash 很重要

Patch Generator 输入时应该记录每个文件的 hash：

```json
{
  "file_path": "src/service/auth_service.rs",
  "hash_before": "abc123"
}
```

Apply 前再检查真实文件 hash 是否一致。

如果不一致：

```text
src/service/auth_service.rs changed since patch was generated
```

这时应该：

```text
拒绝直接 apply
重新 build-context
重新生成 patch
或者进入 rebase/repair 流程
```

这能避免把旧 patch 应用到新代码上。

---

# Commit Message Generator

应用 patch 后，下一步是生成 commit message。

这个可以从：

```text
用户任务
patch_plan
final diff
review summary
test report
```

生成。

示例：

```text
fix(auth): return 401 for invalid login credentials

- return InvalidCredentials instead of Internal on password mismatch
- ensure auth errors map to HTTP 401
- update wrong-password login regression test

Tests:
- cargo test rejects_wrong_password
```

建议格式：

```text
<type>(<scope>): <summary>

<body>

Tests:
- ...
```

type 可以根据 intent 自动判断：

```text
ModifyBehavior → fix / change
DebugError     → fix
Refactor       → refactor
AddFeature     → feat
Test-only      → test
Docs           → docs
```

---

# Rollback 怎么做？

必须支持 rollback。

因为 patch 即使通过 review，用户也可能后悔。

你可以支持两种 rollback。

---

## 1. 未 commit 的 rollback

如果只是 apply 到工作区：

```bash
repoctx rollback patch_run_123
```

内部可以用：

```bash
git apply -R final.patch
```

或者：

```bash
git checkout -- touched_files
```

更安全的是反向 apply 你保存的 patch。

---

## 2. 已 commit 的 rollback

如果已经 commit：

```bash
repoctx rollback patch_run_123
```

可以用：

```bash
git revert <commit_sha>
```

不要默认 `reset --hard`，因为这会丢用户后续改动。

---

# 需要保存哪些运行记录？

你现在应该把一次完整 patch run 存起来。

建议保存：

```text
用户 query
retrieval plan
code_context.md
patch_plan.json
generated patch
repair patches
final patch
verification report
review report
apply mode
branch name
commit sha
touched files
rollback info
```

这样你可以：

```text
复盘一次 AI 修改
重新 apply
rollback
对比不同策略
做 eval
生成审计日志
```

---

# 数据表建议

## patch_runs

```sql
CREATE TABLE patch_runs (
    id TEXT PRIMARY KEY,
    task TEXT NOT NULL,
    status TEXT NOT NULL,
    branch_name TEXT,
    commit_sha TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

## patch_artifacts

```sql
CREATE TABLE patch_artifacts (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    artifact_type TEXT NOT NULL,
    path TEXT,
    content TEXT,
    created_at INTEGER NOT NULL
);
```

`artifact_type` 可以是：

```text
code_context
patch_plan
generated_patch
repair_patch
final_patch
verification_report
review_report
commit_message
```

## apply_records

```sql
CREATE TABLE apply_records (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    apply_mode TEXT NOT NULL,
    branch_name TEXT,
    commit_sha TEXT,
    touched_files_json TEXT NOT NULL,
    rollback_patch TEXT,
    applied_at INTEGER NOT NULL
);
```

---

# Rust 模块可以这样拆

```text
apply/
├── precheck.rs          # 工作区、branch、hash 检查
├── mode.rs              # dry-run/current-branch/new-branch/worktree
├── applier.rs           # 真正 git apply
├── commit.rs            # commit message + git commit
├── rollback.rs          # revert / reverse patch
├── records.rs           # 保存 run artifacts
└── policy.rs            # 是否允许自动 apply
```

核心接口：

```rust
pub struct ApplyRequest {
    pub run_id: String,
    pub patch: String,
    pub mode: ApplyMode,
    pub branch_name: Option<String>,
    pub require_clean_worktree: bool,
    pub require_review_approval: bool,
    pub commit: bool,
}
```

```rust
pub enum ApplyMode {
    DryRun,
    CurrentWorktree,
    NewBranch,
    TemporaryWorktree,
}
```

```rust
pub struct ApplyResult {
    pub success: bool,
    pub mode: ApplyMode,
    pub branch_name: Option<String>,
    pub commit_sha: Option<String>,
    pub touched_files: Vec<String>,
    pub rollback_available: bool,
}
```

---

# Apply Policy

不要所有 patch 都自动 apply。

你可以定义策略：

```rust
pub enum AutoApplyPolicy {
    Never,
    IfApprovedAndLowRisk,
    IfApprovedAndTestsPass,
    AlwaysAsk,
}
```

推荐默认：

```text
AlwaysAsk
```

也就是：

```text
即使 patch verified + approved，也要用户确认。
```

如果以后做全自动 agent，可以允许：

```text
IfApprovedAndLowRisk
```

条件可以是：

```text
review decision = Approve
verification success = true
changed files <= 3
no high severity findings
no public API changes
no migration files
no security-sensitive files
```

---

# 安全边界

这一层必须保守。

遇到这些情况，不要自动 apply：

```text
工作区不干净
patch 没有通过 verify
review 是 NeedsHumanReview 或 Reject
patch 修改 security/auth/payment/permission 核心文件
patch 改动超过阈值
patch 修改数据库 migration
patch 修改 Cargo.toml / lockfile
patch 删除测试
patch 引入 unsafe
patch 修改 public API
```

这类情况应该：

```text
只输出 patch
要求人工确认
```

---

# 用户体验可以这样

一键命令输出：

```text
repoctx patch "把登录失败时的 500 改成 401" --apply

Context:
✅ built

Patch plan:
✅ generated

Patch:
✅ generated

Verification:
✅ cargo fmt --check
✅ cargo check
✅ cargo test rejects_wrong_password

Review:
✅ Approve

Pre-apply:
✅ worktree clean
✅ patch hashes match

Apply target:
branch: ai/auth-wrong-password-401

Apply patch? [y/N]
```

用户确认后：

```text
✅ branch created: ai/auth-wrong-password-401
✅ patch applied
✅ commit created: 9f3a21b

Commit:
fix(auth): return 401 for invalid login credentials

Rollback:
repoctx rollback patch_run_123
```

---

# 做完这一步后，下一步是什么？

Apply / Commit / Rollback 做完后，下一步就是：

# Agent Tooling / MCP / IDE Integration

因为到那时，你的系统已经具备完整闭环：

```text
理解仓库
检索上下文
生成计划
生成 patch
验证修复
审查 patch
安全 apply
支持 rollback
```

接下来就可以把这些能力暴露给外部 AI 工具：

```text
MCP server
IDE extension
CLI agent
HTTP API
ChatGPT / Claude / Cursor 工具调用
```

但现在先不要做 MCP。

当前目标是：

> **让一个 verified + reviewed patch 能被安全地应用、提交、记录和回滚。**

---

# 当前任务清单

你现在按这个顺序做：

```text
1. 实现 PreApplyCheck
2. 支持 --dry-run apply
3. 支持 apply 到新 branch
4. 支持 apply 到 temporary worktree
5. 保存 final patch 和 rollback patch
6. 生成 commit message
7. 支持 commit
8. 支持 rollback
9. 保存 patch run artifacts
10. repoctx patch 接入 --apply / --branch / --commit
```

一句话：

> **下一步是 Apply / Commit / Rollback Workflow：把已经验证和审查通过的 patch 安全应用到真实仓库，并支持分支、提交、记录和回滚。**
