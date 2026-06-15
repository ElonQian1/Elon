下一步做：

# Patch Generator / Diff Generator

也就是：**根据 `patch_plan` 和 `code_context` 生成真正可应用的代码修改 diff。**

你前面已经做到了：

```text
找到相关代码
排序
压缩
打包上下文
生成修改计划
```

现在要进入真正的修改阶段：

```text
patch_plan.md / patch_plan.json
        ↓
Patch Generator
        ↓
unified diff / edit operations
        ↓
dry-run apply
        ↓
format + test
        ↓
verify / repair
```

---

# 这一步要解决什么？

Patch Planner 只回答：

```text
要改哪些文件？
为什么改？
怎么改？
跑哪些测试？
风险是什么？
```

Patch Generator 要回答：

```text
具体代码怎么改？
diff 是什么？
能不能应用？
有没有改到计划外文件？
改完能不能编译？
测试能不能过？
```

也就是说，你的系统要从：

> “给 AI 上下文和计划”

升级到：

> “生成可验证的代码补丁”。

---

# 核心原则

不要让 AI 自由发挥全仓库修改。

Patch Generator 应该被严格约束：

```text
只能修改 patch_plan 里的 must_edit / maybe_edit 文件
优先只修改 must_edit 文件
不能新增计划外文件，除非 plan 明确允许
不能删除无关代码
不能改 generated/vendor/target 文件
生成 unified diff
diff 必须能 dry-run apply
apply 后必须 format/test
```

这一步的关键不是“让 AI 写代码”，而是：

> **让 AI 在计划约束下生成可应用、可验证、可回滚的 patch。**

---

# 输入是什么？

Patch Generator 的输入应该是：

```text
1. 用户任务
2. patch_plan.json
3. code_context.md
4. 允许编辑的文件列表
5. 禁止编辑的文件列表
6. 当前文件 hash
7. 输出格式要求
```

例如：

```text
Task:
把登录失败时的 500 改成 401。

Allowed edits:
- src/service/auth_service.rs
- src/error.rs
- tests/auth_login_test.rs

Inspect only:
- src/api/auth_handler.rs

Forbidden:
- src/service/token_service.rs
- src/repository/user_repository.rs

Output:
unified diff only
```

---

# 输出是什么？

第一版建议输出标准 unified diff：

```diff
diff --git a/src/service/auth_service.rs b/src/service/auth_service.rs
--- a/src/service/auth_service.rs
+++ b/src/service/auth_service.rs
@@ -52,7 +52,7 @@ impl AuthService {
     if !self.password_hasher.verify(password, &user.password_hash)? {
-        return Err(AppError::Internal(anyhow!("password mismatch")));
+        return Err(AppError::Unauthorized);
     }
```

为什么用 unified diff？

```text
人类能读
git 能 apply
容易做 dry-run
容易审计
容易回滚
适合 patch workflow
```

内部也可以保留结构化 edit operations，但最终落地最好还是 diff。

---

# 推荐内部数据结构

你可以定义：

```rust
pub struct PatchRequest {
    pub task: String,
    pub plan: PatchPlan,
    pub context: String,
    pub allowed_files: Vec<String>,
    pub forbidden_files: Vec<String>,
    pub file_hashes: Vec<FileHash>,
    pub output_format: PatchOutputFormat,
}
```

```rust
pub enum PatchOutputFormat {
    UnifiedDiff,
    EditOperations,
}
```

```rust
pub struct PatchSet {
    pub diff: String,
    pub touched_files: Vec<String>,
    pub summary: String,
    pub test_commands: Vec<String>,
}
```

如果你想更安全，可以先让 AI 输出结构化编辑操作：

```rust
pub enum EditOperation {
    ReplaceRange {
        file_path: String,
        start_line: usize,
        end_line: usize,
        replacement: String,
    },
    InsertAfter {
        file_path: String,
        line: usize,
        content: String,
    },
    InsertBefore {
        file_path: String,
        line: usize,
        content: String,
    },
}
```

然后你自己的程序把 `EditOperation` 转成 diff。

但 MVP 可以直接用 unified diff。

---

# 新增 CLI

你现在可以加这些命令：

```bash
repoctx generate-patch "把登录失败时的 500 改成 401"
```

或者分步：

```bash
repoctx build-context "把登录失败时的 500 改成 401" --with-patch-plan > code_context.md

repoctx generate-patch \
  --context code_context.md \
  --plan patch_plan.json \
  > change.patch
```

然后：

```bash
repoctx apply change.patch --dry-run
repoctx apply change.patch
repoctx verify
```

最终可以整合成：

```bash
repoctx patch "把登录失败时的 500 改成 401" --dry-run
```

---

# Patch Generator 的流程

建议你按这个顺序实现：

```text
1. 构造 patch prompt
2. 调用 AI 生成 unified diff
3. 解析 diff
4. 检查 touched files 是否在 allowed list
5. 检查 diff 是否能 apply
6. apply 到临时 worktree
7. cargo fmt
8. cargo test 相关测试
9. 如果失败，生成 repair context
10. 让 AI 修补 patch
```

注意这里的重点是：

```text
先 dry-run
再 apply
再 verify
```

不要直接改用户工作区。

---

# Patch Prompt 应该怎么写？

你的 patch prompt 应该非常硬约束：

```md
<patch_generation_task>

You are generating a code patch.

Rules:
- Output unified diff only.
- Only edit files listed in <allowed_files>.
- Do not edit files listed in <forbidden_files>.
- Preserve existing style.
- Add or update tests when required by the patch plan.
- Do not make unrelated refactors.
- Do not invent APIs not shown in the context.
- If a required file is missing from context, produce no patch and report MISSING_CONTEXT.

<allowed_files>
- src/service/auth_service.rs
- src/error.rs
- tests/auth_login_test.rs
</allowed_files>

<forbidden_files>
- src/service/token_service.rs
- src/repository/user_repository.rs
</forbidden_files>

<patch_plan>
...
</patch_plan>

<code_context>
...
</code_context>

</patch_generation_task>
```

如果你用模型 API，最好要求：

```text
只输出 diff，不要解释
```

解释可以放在另一个通道里，不要混进 patch。

---

# Patch Validator 很重要

生成 diff 后，你要立刻检查：

```text
diff 格式是否合法
是否能 git apply --check
是否修改了 allowed_files 之外的文件
是否包含删除大量代码
是否包含无关格式化
是否修改 generated/vendor/target
是否引入明显危险命令
是否有冲突标记
```

例如：

```bash
git apply --check change.patch
```

然后检查 diff touched files：

```bash
git diff --name-only
```

如果不合法，不要 apply。

---

# Rust 里可以这样做

Patch Generator 模块可以拆成：

```text
patch/
├── prompt.rs          # 构造 patch prompt
├── generator.rs       # 调 AI 生成 diff
├── parser.rs          # 解析 unified diff
├── validator.rs       # 检查 allowed files / diff safety
├── applier.rs         # dry-run apply / apply
├── verifier.rs        # fmt / test / clippy
└── repair.rs          # 失败后构造 repair context
```

核心接口：

```rust
pub trait PatchGenerator {
    async fn generate_patch(&self, request: PatchRequest) -> anyhow::Result<PatchSet>;
}
```

```rust
pub trait PatchValidator {
    fn validate(&self, patch: &PatchSet, plan: &PatchPlan) -> anyhow::Result<ValidationReport>;
}
```

```rust
pub trait PatchApplier {
    fn dry_run(&self, patch: &PatchSet) -> anyhow::Result<ApplyReport>;
    fn apply(&self, patch: &PatchSet) -> anyhow::Result<ApplyReport>;
}
```

---

# Verify 阶段要做什么？

Rust 项目里至少跑：

```bash
cargo fmt --check
cargo test <targeted tests>
```

如果用户允许更慢的检查，再跑：

```bash
cargo check
cargo clippy
cargo test
```

Test plan 应该来自 Patch Planner：

```json
{
  "test_plan": [
    "cargo test auth_login",
    "cargo test rejects_wrong_password"
  ]
}
```

不要默认全量测试，尤其大项目会很慢。第一轮先跑目标测试。

---

# 失败后怎么办？

这一步很关键。Patch 不可能每次一次成功。

你需要做：

# Repair Loop

如果出现：

```text
diff apply failed
cargo fmt failed
cargo test failed
cargo check failed
```

系统应该收集失败信息，生成新的 repair context：

```text
原始任务
patch_plan
已应用 diff
编译错误
测试失败输出
相关文件片段
```

然后让 AI 生成第二个修补 diff。

流程：

```text
generate patch
   ↓
apply
   ↓
verify
   ↓ fail
repair context
   ↓
generate repair patch
   ↓
verify again
```

第一版限制最多 1-2 次 repair，避免无限循环。

---

# Debug 输出也要升级

`repoctx debug` 应该新增：

```text
Patch Generation Trace:
```

示例：

```text
Patch Generation Trace:

Allowed files:
✅ src/service/auth_service.rs
✅ src/error.rs
✅ tests/auth_login_test.rs

Generated diff:
- touched files: 3
- added lines: 18
- removed lines: 6

Validation:
✅ unified diff parsed
✅ all touched files allowed
✅ git apply --check passed
✅ no forbidden files modified

Verification:
✅ cargo fmt --check
❌ cargo test rejects_wrong_password

Failure:
expected 401, got 500

Repair suggestion:
- error mapping may still map InvalidCredentials to 500
- include src/error.rs focused snippet in repair context
```

这会让整个修改过程可解释、可回放。

---

# Evaluation 也要继续升级

现在 eval 不只评测上下文和计划，还要评测 patch。

新增指标：

```text
Patch Apply Rate
diff 能否成功 apply

Allowed File Compliance
是否只改允许文件

Test Pass Rate
目标测试是否通过

Regression Risk
是否改了不该改的文件

Patch Size
修改规模是否合理

Repair Success Rate
失败后修补是否成功
```

eval case 可以变成：

```json
{
  "id": "auth_wrong_password_401",
  "query": "把登录失败时的 500 改成 401",
  "must_edit_files": [
    "src/service/auth_service.rs",
    "src/error.rs",
    "tests/auth_login_test.rs"
  ],
  "must_not_edit_files": [
    "src/service/token_service.rs",
    "src/repository/user_repository.rs"
  ],
  "test_commands": [
    "cargo test rejects_wrong_password"
  ]
}
```

输出：

```text
Case: auth_wrong_password_401

Patch:
✅ apply passed
✅ only allowed files touched
✅ must_edit files touched
✅ forbidden files untouched

Tests:
✅ cargo test rejects_wrong_password

Metrics:
- patch apply rate: 100%
- allowed file compliance: 100%
- test pass: yes
- changed files: 3
- added lines: 18
- removed lines: 6
```

---

# 最小可用版本

第一版 Patch Generator 不要做太大。

只做这 6 件事：

```text
1. 根据 code_context + patch_plan 生成 unified diff
2. 检查 touched files 是否在 allowed list
3. git apply --check
4. apply 到临时分支或临时 worktree
5. 跑 cargo fmt --check 和目标测试
6. 保存 patch generation trace
```

暂时不做复杂 AST patch、不做全自动多轮 agent、不做大规模重构。

---

# 做完 Patch Generator 后，下一步是什么？

做完之后，下一步就是：

# Verification & Repair Loop

也就是：

```text
patch 生成了
        ↓
能否应用？
        ↓
能否格式化？
        ↓
能否编译？
        ↓
测试是否通过？
        ↓
失败时如何自动修？
```

但当前这一步先聚焦：

> **生成一个受约束、可验证、可 dry-run 的 diff。**

---

一句话：

> **下一步是 Patch Generator：把 patch_plan 和 code_context 转成可应用的 unified diff，并用 validator 保证它只修改允许文件、能 dry-run apply。**
下一步做：

# Verification & Repair Loop / 验证与自动修复闭环

你前面已经有了：

```text
Patch Generator
    ↓
生成 unified diff
    ↓
dry-run apply
```

现在下一步要解决的是：

> patch 生成之后，如何确认它真的能用？
> 如果不能用，系统如何知道失败原因，并自动生成修复 patch？

也就是把系统从：

```text
能生成代码修改
```

升级成：

```text
能生成、应用、检查、测试、失败归因、自动修复代码修改
```

---

# 它在整个系统里的位置

现在 pipeline 应该变成：

```text
User Query
   ↓
Context Engine
   ↓
Patch Planner
   ↓
Patch Generator
   ↓
Patch Validator
   ↓
Verification Loop      ⬅️ 下一步
   ↓
Repair Loop
   ↓
Final Patch / Failure Report
```

前面的 Patch Generator 只负责生成 diff。

Verification & Repair Loop 负责：

```text
1. patch 能不能应用？
2. 应用后格式是否正确？
3. 能不能编译？
4. 目标测试能不能过？
5. 失败原因是什么？
6. 需要把哪些错误信息喂回 AI？
7. 修复 patch 是否只改允许的文件？
8. 最终是否得到一个可用 patch？
```

---

# 为什么这一步很关键？

因为 AI 生成的 patch 很常见会出现：

```text
diff 格式错误
上下文行对不上，patch apply 失败
函数签名记错
引入不存在的类型
忘记 import
测试断言没同步
改了 service 但漏了 error mapping
通过目标测试但破坏编译
修改了计划外文件
```

所以不能停在：

```text
AI 生成了 patch
```

而要继续做到：

```text
patch 经过机器验证
```

这一步会让你的系统从“代码建议器”变成“代码修改执行器”。

---

# 你现在要做的核心命令

先加这几个命令：

```bash
repoctx verify change.patch
```

用于验证一个 patch。

然后加：

```bash
repoctx repair change.patch --from-last-failure
```

用于根据失败结果生成修复 patch。

最后合成一个完整命令：

```bash
repoctx patch "把登录失败时的 500 改成 401" --verify --repair 2
```

它的完整流程是：

```text
build context
  ↓
plan patch
  ↓
generate patch
  ↓
apply in temporary worktree
  ↓
run verification
  ↓
if failed, repair
  ↓
verify again
  ↓
output final patch
```

---

# 第一层：隔离执行环境

不要直接在用户工作区 apply patch。

推荐用：

```bash
git worktree
```

或者复制到临时目录。

流程：

```text
当前 repo
  ↓
创建临时 worktree
  ↓
在临时 worktree 里 apply patch
  ↓
运行 fmt / check / test
  ↓
成功则输出最终 patch
  ↓
失败则保存日志和失败上下文
```

这样即使 patch 失败，也不会污染用户工作区。

CLI 可以是：

```bash
repoctx verify change.patch --worktree /tmp/repoctx-run-123
```

或者自动创建：

```bash
repoctx verify change.patch --isolated
```

---

# 第二层：验证顺序

验证不要一开始就跑全量测试。

建议按成本从低到高：

```text
1. patch policy check
2. git apply --check
3. apply patch
4. format check
5. cargo check
6. targeted tests
7. broader tests
8. clippy / full test，可选
```

对 Rust 项目，第一版可以这样：

```bash
git apply --check change.patch
git apply change.patch
cargo fmt --check
cargo check
cargo test <targeted_test>
```

更严格时再加：

```bash
cargo clippy
cargo test
```

---

# 第三层：失败分类

Verification Loop 不应该只说：

```text
失败了
```

它要分类。

常见失败类型：

```rust
pub enum FailureKind {
    PatchParseFailed,
    PatchApplyFailed,
    ForbiddenFileModified,
    FormatFailed,
    CompileFailed,
    TestFailed,
    LintFailed,
    Timeout,
    Unknown,
}
```

不同失败类型对应不同 repair 策略。

---

## 1. PatchApplyFailed

说明 diff 上下文不匹配。

修复策略：

```text
重新读取目标文件最新内容
把失败 hunk、目标文件相关片段、patch_plan 喂给 AI
要求生成新的 diff
```

---

## 2. FormatFailed

说明代码格式不符合 Rustfmt。

修复策略：

```text
优先自动运行 cargo fmt
如果 cargo fmt 成功，记录格式化 diff
如果 cargo fmt 失败，把格式错误发给 AI repair
```

---

## 3. CompileFailed

说明代码不能编译。

常见原因：

```text
缺 import
类型名错
函数签名不匹配
Result 类型不匹配
trait impl 没同步
生命周期/借用错误
```

修复策略：

```text
提取 compiler error
定位文件和行号
召回相关符号
构造 repair context
生成 incremental repair diff
```

---

## 4. TestFailed

说明编译通过，但行为不对。

常见原因：

```text
业务逻辑没改完整
测试期望没同步
错误映射还没修
mock 没改
fixture 没改
```

修复策略：

```text
把失败测试名、assertion diff、stdout/stderr、相关测试代码喂回 AI
要求修行为，而不是盲改测试
```

---

## 5. ForbiddenFileModified

说明 AI 改了不该改的文件。

修复策略：

```text
直接拒绝该 patch
要求重新生成，只允许修改 allowed_files
```

这个不要自动宽容。

---

# Rust 数据结构可以这样设计

## VerificationReport

```rust
pub struct VerificationReport {
    pub run_id: String,
    pub patch_id: String,
    pub success: bool,
    pub checks: Vec<CheckResult>,
    pub touched_files: Vec<String>,
    pub failure_summary: Option<String>,
}
```

## CheckResult

```rust
pub struct CheckResult {
    pub name: String,
    pub command: Option<String>,
    pub success: bool,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub failure_kind: Option<FailureKind>,
}
```

## RepairRequest

```rust
pub struct RepairRequest {
    pub task: String,
    pub patch_plan: PatchPlan,
    pub original_patch: String,
    pub verification_report: VerificationReport,
    pub repair_context: String,
    pub allowed_files: Vec<String>,
    pub forbidden_files: Vec<String>,
    pub attempt: usize,
    pub max_attempts: usize,
}
```

## RepairResult

```rust
pub struct RepairResult {
    pub repair_patch: String,
    pub combined_patch: String,
    pub verification_report: VerificationReport,
    pub success: bool,
}
```

---

# Repair Context 应该包含什么？

不要把整个 repo 都重新喂给 AI。

Repair context 应该非常聚焦：

```text
1. 原始用户任务
2. patch_plan
3. 原始 patch
4. 失败类型
5. 失败命令
6. 编译/测试错误摘要
7. 相关文件片段
8. allowed_files / forbidden_files
9. repair 输出要求
```

例如：

````md
<repair_context>

# Original Task
把登录失败时的 500 改成 401。

# Failure Kind
CompileFailed

# Failed Command
cargo check

# Compiler Error
```text
error[E0599]: no variant named `Unauthorized` found for enum `AppError`
 --> src/service/auth_service.rs:57:30
````

# Relevant Code

## src/error.rs

```rust
pub enum AppError {
    Internal(anyhow::Error),
    InvalidCredentials,
}
```

## src/service/auth_service.rs

```rust
return Err(AppError::Unauthorized);
```

# Repair Instructions

* Generate an incremental unified diff.
* Only edit allowed files.
* Do not undo unrelated successful changes.
* Prefer using existing AppError::InvalidCredentials if it already maps to 401.
* If mapping is missing, update error mapping.

</repair_context>

````

这样 AI 更容易修对。

---

# Repair Loop 流程

第一版最多修 1 到 2 次。

不要无限循环。

流程：

```text
generate patch
  ↓
verify
  ↓
failed?
  ↓
classify failure
  ↓
build repair context
  ↓
generate repair patch
  ↓
validate repair patch
  ↓
apply repair patch
  ↓
verify again
  ↓
success or final failure report
````

伪代码：

```rust
pub async fn verify_and_repair(
    task: &str,
    patch: PatchSet,
    plan: PatchPlan,
    max_repairs: usize,
) -> anyhow::Result<FinalPatchResult> {
    let mut current_patch = patch;

    for attempt in 0..=max_repairs {
        let report = verifier.verify(&current_patch).await?;

        if report.success {
            return Ok(FinalPatchResult::Success {
                patch: current_patch,
                report,
            });
        }

        if attempt == max_repairs {
            return Ok(FinalPatchResult::Failed {
                patch: current_patch,
                report,
            });
        }

        let failure = failure_classifier.classify(&report)?;
        let repair_context = repair_context_builder.build(
            task,
            &plan,
            &current_patch,
            &report,
            &failure,
        )?;

        let repair_patch = repair_generator
            .generate_repair(repair_context)
            .await?;

        current_patch = patch_combiner.combine(current_patch, repair_patch)?;
    }

    unreachable!()
}
```

---

# Debug 输出要升级

`repoctx debug` 里应该新增：

```text
Verification Trace
Repair Trace
```

示例：

```text
Verification Trace:

1. policy_check
   ✅ passed
   touched files:
   - src/service/auth_service.rs
   - src/error.rs
   - tests/auth_login_test.rs

2. git apply --check
   ✅ passed

3. cargo fmt --check
   ✅ passed

4. cargo check
   ❌ failed
   failure_kind: CompileFailed

Compiler error:
- AppError::Unauthorized does not exist
- suggested existing variant: AppError::InvalidCredentials

Repair Trace:

Attempt 1:
- repair strategy: use existing error variant
- generated repair patch touched:
  - src/service/auth_service.rs
  - src/error.rs

Verification after repair:
✅ cargo fmt --check
✅ cargo check
✅ cargo test rejects_wrong_password

Final:
✅ patch verified
```

这个 trace 会让整个系统非常容易调试。

---

# 存储运行记录

建议新增几张表。

## patch_runs

```sql
CREATE TABLE patch_runs (
    id TEXT PRIMARY KEY,
    task TEXT NOT NULL,
    patch_plan_json TEXT NOT NULL,
    final_status TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

## verification_runs

```sql
CREATE TABLE verification_runs (
    id TEXT PRIMARY KEY,
    patch_run_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    success INTEGER NOT NULL,
    report_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
```

## repair_attempts

```sql
CREATE TABLE repair_attempts (
    id TEXT PRIMARY KEY,
    patch_run_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    failure_kind TEXT NOT NULL,
    repair_context TEXT NOT NULL,
    repair_patch TEXT NOT NULL,
    success INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
```

这样你可以回放：

```text
某次 patch 为什么失败？
repair 是怎么修的？
是哪条命令失败？
最终 patch 改了哪些文件？
```

---

# Evaluation 也要升级

现在你的 eval 不应该只看：

```text
上下文召回率
patch 是否能 apply
```

还要看完整闭环成功率。

新增指标：

```text
Patch Apply Rate
Format Pass Rate
Compile Pass Rate
Target Test Pass Rate
Repair Success Rate
Average Repair Attempts
Forbidden File Violation Rate
Final Success Rate
```

示例输出：

```text
Eval Summary:

Cases: 50

Patch apply rate:          86%
Format pass rate:          82%
Compile pass rate:         68%
Target test pass rate:     54%
Repair success rate:       61%
Final success rate:        73%
Forbidden file violations: 2%
Average repair attempts:   0.8
```

这比单纯看“AI 是否生成了 patch”有意义得多。

---

# 第一版最小实现

不要一开始做太复杂。

先实现：

```text
1. 临时 worktree
2. git apply --check
3. allowed files 检查
4. cargo fmt --check
5. cargo check
6. targeted cargo test
7. 失败分类
8. 一次 repair
9. verification report
```

第一版 repair 只需要支持：

```text
PatchApplyFailed
CompileFailed
TestFailed
ForbiddenFileModified
```

其他失败先归为：

```text
Unknown
```

---

# 推荐 CLI 形态

分步命令：

```bash
repoctx generate-patch "把登录失败时的 500 改成 401" > change.patch

repoctx verify change.patch --isolated

repoctx repair change.patch --last-failure
```

一键命令：

```bash
repoctx patch "把登录失败时的 500 改成 401" \
  --verify \
  --repair 2 \
  --isolated
```

输出：

```text
Patch run: patch_run_017

Plan:
✅ generated

Patch:
✅ generated
✅ policy check passed
✅ apply check passed

Verification:
✅ cargo fmt --check
❌ cargo check

Repair attempt 1:
✅ repair patch generated
✅ apply check passed
✅ cargo fmt --check
✅ cargo check
✅ cargo test rejects_wrong_password

Final:
✅ verified patch ready
```

---

# 做完这一步后，下一步是什么？

Verification & Repair Loop 做完后，下一步是：

# Patch Review / Change Review

也就是让系统在最终 patch 生成之后，做一次审查：

```text
这次修改是否符合 patch_plan？
有没有越界修改？
有没有潜在回归？
测试是否覆盖新行为？
diff 是否过大？
是否应该拆分 patch？
是否需要人工确认？
```

但现在先不要跳过去。

当前目标是：

> **让每个 patch 都能被隔离应用、格式化、编译、测试，并且失败后能自动修复。**

---

# 当前任务清单

你现在按这个顺序做：

```text
1. 实现 isolated worktree runner
2. 实现 patch validator
3. 实现 verification command runner
4. 实现 failure classifier
5. 实现 repair context builder
6. 实现 repair patch generator
7. 实现 max repair attempts
8. 保存 verification / repair trace
9. eval 增加 final success rate
10. repoctx patch 支持 --verify --repair
```

一句话：

> **下一步是 Verification & Repair Loop：让系统不仅能生成 patch，还能验证 patch、定位失败原因，并自动生成修复 patch，直到得到一个可用的最终修改。**
