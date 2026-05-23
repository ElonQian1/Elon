# 一龙云端开发平台 — AI 代理全局指令

> 本文件被所有 AI 代理自动加载。请在回答任何编码问题前先理解本文件内容。

## 项目定位

**云端APK开发平台**：用户在手机APK上用自然语言和AI对话，描述自己想要的功能；AI在服务器上修改代码、自动编译、部署，最后把新的APK下载链接发回用户手机。用户无需任何编程知识即可定制自己的移动应用。

---

## 系统组件一览

```
[用户手机 APK]
    │ 自然语言对话
    ▼
[AI 对话后端]  ←─── 理解需求，规划代码修改
    │
    ├─► [Rust 服务端代码]   (server/)
    ├─► [Android APK 代码]  (android/)
    └─► [前端 Web 代码]      (frontend/)
    │
    ▼
[自动化流水线]
    git commit → 编译构建 → 部署上线 → APK 打包签名
    │
    ▼
[推送下载链接] → 用户手机
```

---

## 技术栈

| 层 | 技术 | 说明 |
|---|---|---|
| 移动端 | Android (Kotlin/Java) | 用户使用的 APK，含 AI 对话界面 |
| 服务端 | Rust | 核心业务逻辑、API 接口 |
| 前端 | (待定) | Web 管理界面或 H5 内嵌页 |
| AI 对话 | LLM API | 理解用户需求，生成代码修改方案 |
| CI/CD | 自动化脚本 | git → 编译 → 部署 → APK签名分发 |
| 版本控制 | Git | 所有代码变更走 git 管理 |

---

## 核心工作流（AI 代理必须理解）

1. **需求理解**：用户在APK内用自然语言描述需求
2. **代码定位**：AI确定需要修改哪些文件（Rust/Android/前端）
3. **安全修改**：在服务器上修改对应代码，保持代码风格一致
4. **提交构建**：`git add → git commit → 触发构建流水线`
5. **编译部署**：Rust编译、Android打包签名、前端构建，部署到服务器
6. **反馈用户**：将新APK下载链接通过对话界面发回用户手机

> 详细流程见：`docs/ai-agent-workflow.md`

---

## VS Code Copilot 工作方式记忆

- 把 VS Code Copilot 理解为 agent loop：先组装上下文，再用工具读取/编辑/运行命令，工具结果回到上下文后继续迭代，最后验证和交付。
- 上下文来自系统指令、customizations、用户消息、会话历史、隐式编辑器/Git 状态、显式 `#` 引用和工具输出；没有进入上下文的内容对模型不可见。
- 项目级稳定规则放在 `.github/copilot-instructions.md`；局部规则放在 `.github/instructions/*.instructions.md`；重复任务放在 `.github/prompts/*.prompt.md`；角色和工具受限的流程放在 `.github/agents/*.agent.md`。
- 复杂任务先用 Plan/规划思路做 discovery、alignment、design、refinement；计划确认后再进入实现。
- 修改 AI customization 时，保持规则短、自包含、可版本化；需要完整背景时引用 `docs/vscode-copilot-working-model.md`，不要在多个文件重复长规则。

---

## 关键原则（AI 代理必须遵守）

- **每次修改都要 git commit**，commit message 用中文描述用户的需求
- **修改代码前先读懂上下文**，不随意删除已有功能
- **编译失败必须回滚或修复**，不允许将编译错误的代码部署
- **APK 签名密钥不得泄露**，相关操作只走自动化脚本
- **每个用户的修改是隔离的**，不能让一个用户的操作影响其他用户
- **代码变更记录用户身份**，commit 信息中包含用户标识

---

## 参考文档（按需读取）

| 文档 | 内容 |
|---|---|
| `docs/system-architecture.md` | 系统架构详细设计、组件交互、数据流 |
| `docs/ai-agent-workflow.md` | AI代理如何执行代码修改→编译→部署的完整流程 |
| `docs/vscode-copilot-working-model.md` | VS Code Copilot 最新 agent / instructions / prompt files / custom agents 工作方式速记 |

---

## 当前开发状态

- [ ] 项目整体架构设计
- [ ] Rust 服务端基础框架
- [ ] Android APK 基础框架
- [ ] AI 对话后端集成
- [ ] 自动化编译部署流水线
- [ ] APK 分发机制

> AI 代理在修改任何代码时，请先确认该模块的当前完成状态。
