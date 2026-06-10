**Agent Harness** 可以理解成：

> **把大模型包装成“能干活的 AI 代理”的运行框架。**

或者更口语一点：

> **大模型本身只是大脑，Agent Harness 是身体、工具箱、工作台、权限系统和执行流程。**

在 Codex CLI、Claude Code、Copilot CLI 这类工具里，真正工作的不是一个裸 API key，而是：

```text
LLM 模型
+ Agent Harness
+ 工具系统
+ 项目上下文
+ 文件读写
+ 命令执行
+ 权限控制
+ 多轮任务循环
```

---

## 1. 为什么叫 Harness？

`Harness` 这个词本来有几种意思：

```text
马具
安全带
线束
测试框架
把某个东西接入并控制起来的装置
```

在 AI agent 语境里，**Agent Harness** 大概就是：

```text
把 LLM 接入真实工作环境，
给它工具，
限制它权限，
管理它的上下文，
让它一步一步完成任务的框架。
```

所以你可以把它翻译成：

```text
AI 代理执行框架
AI 代理运行壳
Agent 承载框架
代理工作流框架
```

我个人更推荐叫：

> **Agent 执行框架**

因为它强调“不是只聊天，而是真的执行任务”。

---

## 2. 裸模型和 Agent Harness 的区别

### 裸模型

裸模型大概是这样：

```text
用户：请帮我修改这个 Rust 项目
模型：好的，你可以把 xxx 文件改成这样……
```

它只能：

```text
看你发给它的内容
根据上下文生成文字
给你建议
给你代码片段
```

它自己不能：

```text
打开你的项目
搜索代码
读取文件
修改文件
运行 cargo test
看报错
继续修
生成 git diff
```

---

### 有 Agent Harness 的模型

有了 Agent Harness 之后，流程变成这样：

```text
用户：请帮我修改这个 Rust 项目

Agent：
1. 读取项目目录
2. 查看 Cargo.toml
3. 搜索相关函数
4. 读取关键文件
5. 修改代码
6. 运行 cargo check
7. 根据报错继续修改
8. 运行 cargo test
9. 展示 git diff
10. 总结改了什么
```

这个时候，模型不再只是“回答问题”，而是被 Harness 包装成一个“会操作环境的代理”。

---

## 3. 一个 Agent Harness 通常包含什么？

可以把它想成下面这个结构：

```text
┌──────────────────────────────┐
│          用户需求              │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│        Agent Harness          │
│                              │
│  1. 上下文管理                 │
│  2. 工具调用                   │
│  3. 任务循环                   │
│  4. 权限控制                   │
│  5. 状态记录                   │
│  6. 错误恢复                   │
│  7. 结果验证                   │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│            LLM                │
│   GPT / Claude / Gemini 等     │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│            工具层              │
│                              │
│ read_file                    │
│ write_file                   │
│ search_code                  │
│ apply_patch                  │
│ run_command                  │
│ git_diff                     │
│ cargo_test                   │
│ browser                      │
│ database                     │
│ MCP tools                    │
└──────────────────────────────┘
```

也就是说，Agent Harness 不是模型本身，而是模型外面的那一整套运行系统。

---

## 4. 用做饭来比喻

裸模型像一个很聪明的厨师，但他只能隔着电话告诉你：

```text
你应该先切菜，然后下锅，然后调味。
```

Agent Harness 则是给这个厨师配上：

```text
厨房
刀
锅
冰箱
燃气灶
食材
安全规则
计时器
试吃反馈
清洁流程
```

于是厨师真的可以做菜。

对应到代码开发就是：

```text
LLM = 厨师的大脑
Agent Harness = 厨房和操作系统
工具 = 刀、锅、炉子
项目文件 = 食材
测试/编译器 = 试吃和质检
权限系统 = 防止把厨房炸了
```

---

## 5. 在 AI 编程工具里，Agent Harness 具体负责什么？

以一个 Rust 项目为例，用户说：

```text
帮我把用户登录模块从同步接口改成 async
```

Agent Harness 需要做很多事。

### 第一步：构造上下文

它不能把整个大项目一次性全塞给模型，所以要先收集上下文：

```text
读取文件树
读取 Cargo.toml
读取 README
查找 login / auth / user / session 相关代码
找到相关函数、struct、trait、测试
```

对应工具可能是：

```text
list_dir
read_file
rg
find_definition
find_references
semantic_search
```

---

### 第二步：让模型制定计划

Harness 会把上下文交给模型，让模型判断：

```text
需要改哪些文件？
函数签名怎么改？
调用方有哪些？
测试需要怎么更新？
是否会影响 trait？
是否会影响数据库层？
```

模型可能输出一个计划：

```text
1. 修改 AuthService::login 为 async
2. 修改 UserRepository trait
3. 修改 PgUserRepository 实现
4. 修改 handler 调用处
5. 修改测试
6. 运行 cargo check
```

---

### 第三步：执行修改

Agent Harness 调用工具真的改文件：

```text
apply_patch
write_file
edit_file
```

这一步不是模型自己“凭空改”，而是 Harness 把模型生成的 patch 应用到真实文件系统。

---

### 第四步：运行验证

然后 Harness 执行：

```bash
cargo fmt
cargo check
cargo test
cargo clippy
```

如果报错，Harness 会把错误信息再喂给模型。

例如：

```text
error[E0277]: `Result<User, AppError>` is not a future
```

模型看到后继续判断：

```text
这里漏了 .await
这里函数还没有改成 async
这里 trait 签名不一致
```

然后再修改。

这就是 agent loop。

---

## 6. Agent Harness 最核心的概念：循环

AI agent 不是一次性生成答案，而是反复循环：

```text
观察 -> 思考 -> 行动 -> 观察结果 -> 再思考 -> 再行动
```

在编程场景里就是：

```text
读取项目
-> 搜索代码
-> 阅读文件
-> 修改文件
-> 运行测试
-> 看错误
-> 再修改
-> 再测试
-> 输出结果
```

可以写成：

```text
while task_not_done:
    context = observe_project()
    plan = model.think(context)
    action = model.choose_tool(plan)
    result = run_tool(action)
    update_context(result)
    verify_progress()
```

这整个循环的管理者，就是 Agent Harness。

---

## 7. Codex CLI / Claude Code / Copilot CLI 为什么能改项目？

因为它们不是简单地调用模型 API。

它们大概都有类似的 Agent Harness：

```text
模型接口
文件系统工具
shell 执行工具
搜索工具
patch 工具
git 工具
权限系统
上下文管理
项目说明文件
任务循环
验证机制
```

比如它们可以做：

```bash
ls
rg "login"
cat src/auth.rs
apply_patch
cargo check
cargo test
git diff
```

模型负责决定下一步做什么，Harness 负责真的执行。

所以更准确地说：

```text
Codex CLI / Claude Code / Copilot CLI
= LLM + Agent Harness + 本地开发工具链
```

---

## 8. Agent Harness 和 IDE 的关系

IDE 是一个很大的开发环境。

Agent Harness 是让 AI 代理工作的执行框架。

它们可以组合，但不是一回事。

### CLI Agent Harness

例如 Claude Code、Codex CLI：

```text
在终端运行
主要靠 shell、git、grep、编译器、测试工具
UI 简单
自动化能力强
```

### IDE Agent Harness

例如 VS Code Copilot Agent：

```text
在编辑器里运行
可以使用打开的文件、LSP、诊断信息、代码跳转、UI diff
交互体验更好
```

两者底层思想类似：

```text
让模型拥有工具
让模型能读写项目
让模型能执行命令
让模型能验证结果
```

区别只是工具来源不同：

```text
CLI agent 借用 shell 工具链
IDE agent 借用 IDE / LSP / 编辑器上下文
```

---

## 9. 你们自己做 AI 代理 IDE 时，Agent Harness 是核心

如果你们要做自己的 AI 代理 IDE，真正应该先做的不是 UI，而是 Agent Harness。

最小版本可以这样设计：

```text
Agent Harness
├── Model Adapter
│   ├── OpenAI
│   ├── Claude
│   ├── Gemini
│   └── Local Model
│
├── Context Manager
│   ├── repo map
│   ├── 文件摘要
│   ├── 当前任务历史
│   ├── token 预算
│   └── 上下文压缩
│
├── Tool Registry
│   ├── read_file
│   ├── write_file
│   ├── apply_patch
│   ├── search_code
│   ├── run_command
│   ├── git_diff
│   └── run_tests
│
├── Permission Manager
│   ├── 只读模式
│   ├── 修改文件审批
│   ├── 命令白名单
│   ├── 禁止访问 secrets
│   └── 沙箱执行
│
├── Agent Loop
│   ├── plan
│   ├── act
│   ├── observe
│   ├── verify
│   └── repair
│
└── Result Reporter
    ├── 修改总结
    ├── git diff
    ├── 测试结果
    └── 风险提示
```

这就是你们产品的“发动机”。

IDE UI 只是外壳。

---

## 10. 一个简单公式

你可以这样记：

```text
LLM = 大脑
Tools = 手脚和工具
Context = 记忆和资料
Agent Loop = 工作流程
Permission = 安全边界
Harness = 把这些东西组织起来的运行框架
```

最终：

```text
Agent Harness
= 模型调用
+ 上下文管理
+ 工具调用
+ 状态管理
+ 权限控制
+ 执行循环
+ 结果验证
```

在 AI 编程产品里：

```text
Agent Harness
= 让模型能够像工程师一样阅读、修改、运行、验证代码的系统
```

所以你可以把前面的问题统一成一句话：

> **Codex、Claude Code、Copilot CLI 这类工具真正值钱的地方，不只是模型，而是它们背后的 Agent Harness。它们把模型接到了文件系统、shell、git、编译器、测试框架和权限系统上，所以模型才从“会写代码的聊天机器人”变成了“能修改项目的 AI 工程师”。**
