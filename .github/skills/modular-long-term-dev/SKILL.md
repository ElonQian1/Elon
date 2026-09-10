---
name: modular-long-term-dev
description: Prevent giant source files and sprawling documents during software development. Use when adding features, refactoring, reviewing module boundaries, auditing file growth, or adopting incremental size gates across repositories. 中文：模块化开发、避免巨型文件、按职责拆分、历史文件止增。Does not manage binary assets or build-cache cleanup.
metadata:
  version: "2.0.0"
---

# 模块化长期开发

让新增功能进入清晰的职责模块，用可执行的增量门禁防止巨型文件继续增长。项目自身的明确规则优先；本 Skill 不携带一龙项目的部署、分支、服务器或认领流程。

## 写代码前

1. 读取目标项目的 `AGENTS.md` 和命中的架构文档，检查 Git 状态及目标文件的近期变动；保留其他任务的修改。需要远端基线时按项目实际 remote/branch fetch，不假定 `origin/main`，不自动 rebase、发布或派发代理。
2. 找到本次行为所属领域及已有模块。新增功能或实质重构时，用简短表格或 JSON 列出目标文件、职责、当前行数、预计新增和上限；简单局部修复只核对目标文件即可。
3. 超出预算先调整模块方案。入口负责生命周期、路由和依赖组装，业务规则、协议、持久化、UI 与外部副作用进入相应领域模块。

## 默认预算

以下为共享工具的默认值；项目可在 `.modularity.json` 显式配置更严格或有理由的角色规则。

| 角色 | 建议目标 | 硬上限（物理行） |
|---|---:|---:|
| 入口/组装 | 300 | 500 |
| 普通业务模块、脚本、样式 | 500 | 800 |
| 工具模块 | 400 | 600 |
| 协议/类型定义 | 600 | 1000 |
| 测试 | 600 | 1000 |
| Markdown 文档 | 500 | 800，且不超过 50000 UTF-8 字节 |

- 源码同时有 128000 字节上限；不要用压行、删除说明、改文件后缀或移到忽略目录绕过门禁。
- 普通新文件超过 500 行需检查单一职责，超过角色硬上限必须拆分。函数超过约 80 行或同时承担两类职责时进行人工审查；脚本不假装能跨语言理解函数和依赖。
- 历史超限文件可通过初次接入基线记录，之后不得增长；拆小后运行 `ratchet` 收紧基线。不能重建基线掩盖本次新增债务。
- 行数过关不是模块化验收。避免机械分成 `part1/part2`、跨模块访问内部状态、循环依赖和为了拆文件而制造空转发层。

## 实现与验收

- 优先沿已有领域边界扩展；新模块用业务名称。抽取混杂职责时先做可验证的行为等价搬迁，再改变行为；复杂任务拆成可独立验证的步骤，不按文件数量机械切分。
- 保持入口、公开接口、数据所有权和依赖方向清晰；同步 import/export、模块声明、路由、构建入口和测试调用方。
- 文档按主题分拆，入口只保留当前事实、约束和链接。生成物需有可复现生成入口及精确排除理由，不能把手写源码当生成物排除。
- 并行工作仅在任务已授权并行时按模块协调，避免共同修改同一符号；不因加载本 Skill 自动创建代理或新任务。
- 跑与修改有关的编译/测试，再执行项目大小检查。提交时只纳入本任务文件；报告目标文件变化和测试证据，不声称存量债务已全部解决。

关于状态、依赖、纯搬迁及重构验收的具体判断，按需读 [模块边界原则](references/module-boundaries.md)。

## 可移植检查工具

需要 Git 和 Node.js 18+，无 npm 依赖。`<skill-dir>` 指本 `SKILL.md` 所在目录；相对路径均以 `--root` 指定的仓库根为准。

```text
node <skill-dir>/scripts/modularity.mjs check --root <repo>
node <skill-dir>/scripts/modularity.mjs check --root <repo> --staged
node <skill-dir>/scripts/modularity.mjs check --root <repo> --base <commit>
node <skill-dir>/scripts/modularity.mjs audit --root <repo> --json
```

默认检查工作区已跟踪及未被 Git 忽略的新文件；`--staged` 只检查实际暂存版本。`--base` 检查基线未被放宽，并在已接入的仓库收紧历史文件预算；不存在的 ref 直接报错。

首次接入、基线治理、CI 与跨电脑安装时读 [跨项目接入](references/adoption.md)，使用 [配置模板](assets/modularity.config.json)。工具只统计尺寸，不替代职责与依赖审查，也不限制模型、纹理、音视频等二进制资产。
