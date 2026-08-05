# 一龙项目知识中心

本文是面向开发者、项目管理者和各类 AI 代理的项目知识首页。它回答“项目是什么、先读什么、到哪里继续找”，不复制具体实现细节或工作流硬规则。

判断项目当前已实现、建设中、提案或已否决状态时，先读根目录 `AI_CURRENT.md`。

## 推荐阅读顺序

1. `AI_CURRENT.md`：当前事实、提案和已禁用路线。
2. `AI_PROJECT.md`：项目定位、能力边界和任务入口。
3. `AI_ARCHITECTURE.md`：稳定的模块地图与依赖方向。
4. `docs/system-architecture.md`：后端、PC、节点和客户端的数据流。
5. `AI_INDEX.md`：从功能定位到源码、测试与专项文档。
6. `AGENTS.md`：所有 AI 供应商共享的最小路由入口；真正的写任务硬规则位于 `.github/copilot-instructions.md`。

## 知识主题地图

| 主题 | 入口 | 适合解决的问题 |
|---|---|---|
| 项目总览 | `AI_PROJECT.md` | 项目目标、范围、任务如何开始 |
| 平台架构 | `AI_ARCHITECTURE.md`、`docs/system-architecture.md` | 模块边界、数据流、改动落点 |
| 代码归属与迁移 | `docs/architecture/source-of-truth.md`、`docs/architecture/legacy-inventory.md`、`docs/architecture/feature-parity-matrix.md` | 新旧入口、兼容代码和功能迁移判定 |
| 后端与 API | `AI_INDEX.md` | 路由、接口、服务端实现与验证入口 |
| PC Web 工作台 | `docs/pc-frontend-migration.md` | `/pc`、React/Vite、项目文档工作台 |
| 后台多端 UI 设计 | `docs/headless-ui-design-mcp.md` | AI 无需打开画布即可发现、捕获和读取 Web/PWA/Tauri/Android 页面 |
| Windows 节点 | `docs/node-agent-upgrade-compatibility.md` | 本机节点、数据目录、升级和离线行为 |
| Android 客户端 | `docs/android-setup.md` | APK、Compose、真机渲染与发布 |
| 分布式算力联邦 | `docs/distributed-compute/README.md` | 用户节点、外部矿池、按需插件、任务调度和期货锁价结算 |
| AI 与上下文 | `AGENTS.md` | 跨供应商入口、按需指令、Prompt/Agent/Skill |
| 功能需求与代理任务 | `docs/project-feature-registry.md` | 正式需求登记、低 token 发现、认领、状态机、证据与漂移治理 |
| 讨论知识与脑图演化 | `docs/discussion-knowledge-compiler.md` | 长聊天拆分、分叉、节点晋升、版本回看与修正 |
| 用户与项目系统 | `docs/user-project-system.md` | 用户项目、频道、Git 工作区与文档知识库 |
| 监督式 PC 项目开发 | `docs/supervised-pc-project-development.md` | Codex 桌面监督、PC 节点执行、证据验收和平台改进续跑 |
| Codex 桌面工作流效率 | `docs/codex-desktop-workflow-efficiency.md` | 增量 Wait、权威 Resume、终态、Runtime、收尾契约与 A/B 度量 |
| 开放商业与 AI 经济 | `docs/open-commerce/README.md` | 当前能力、商户 AI、开放商业 V1、共享资源与 Sui 提案边界 |
| 发布与运维 | `docs/ai-agent-workflow.md` | 验证、发布、故障恢复和任务收尾 |
| 安全与权限 | `docs/windows-client-defender.md` | 权限、凭据、密钥和安全边界 |
| fb2 AI 中心 | `docs/fb2-ai-center/README.md` | 子项目契约、SDK、计费和验收 |

## 两条互不冲突的浏览轴

- “知识架构”按业务主题回答“这份文档讲什么”。目录树、项目模板和推荐阅读属于这一轴。
- “治理视图”按权威性与生命周期回答“这份文档能否作为当前事实”。必须、按需、当前知识、草稿、证据、归档和等待整理属于这一轴。

同一份文档会同时拥有一个知识主题和一个治理属性。旧讨论可以位于“AI 与上下文 / 工作区”，同时在治理视图中标记为“草稿”；主题位置不会提升它的权威性。

## 路径权重

AI 检索时先看路径，再看元数据，最后才按需读取正文：

1. 根目录入口与 `.github/copilot-instructions.md` 是共享路由和硬规则。
2. `.github/instructions/` 是按任务加载的领域规则。
3. `docs/` 中的当前架构、规范和运行手册是专题知识。
4. 名称含 `讨论`、`draft`、`PLAN`、`PROGRESS`、`report` 的材料默认不是当前事实。
5. `archive`、被替代文档和历史证据默认不进入当前实现检索。

详细判定规则见 `.github/instructions/document-authority.instructions.md`。

## 维护方式

PC 项目文档工作台用 `.elon/document-sections.json` 保存项目模板、知识首页、分区树、治理覆盖、文档关系，以及独立的产品功能图和技术架构图。网页和供应商无关 MCP 都消费 Rust 后端生成的同一项目图谱；AI 先查局部节点、实现证据和 token 阅读计划，只有冲突、歧义和缺失入口才少量按需阅读正文。

AI 整理默认使用 Git 双提交事务：整理前提交原始文档，完成分区、重命名或移动后再提交整理结果。正文不会因虚拟分类而被改写；实体移动禁止覆盖、禁止越出项目，且必须验证目录 revision。
