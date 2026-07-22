# Elon AI Project Guide

本文件给 AI 代理快速理解 elon 自项目。规则权威仍是 `.github/copilot-instructions.md`，本文件只记录项目事实、架构入口和项目理解/RAG现状。

## 项目是什么

elon 是一个通过持续讨论把模糊想法变成可交付应用的 AI 开发平台。用户在手机 APK 或 PC 工作台里描述目标，一龙 AI 先帮助用户澄清产品方向；对不确定性较高的需求，可由低成本“预言家 AI（Demo Oracle）”生成可讨论的 demo、页面草图或交互预演。方向确认后，总调度 AI 再从平台的 AI-to-AI Skill 能力库中选择和组合合适 Skill，把任务交给 AI CLI/Codex/Copilot 等代理，在真实 Git 工作区修改代码、验证、构建、发布，最后把 APK、项目或其他产物回传给用户。

这里的 Skill 是平台内部、面向 AI 调用的官方工程能力，不要求普通用户手动挑选提示词，也不作为可上架、购买或交易的商品。用户负责表达目标和判断 demo 是否符合预期；总调度 AI 负责需求理解、Skill 路由、Matter 计划、执行编排和验收汇报。

本仓库同时包含：

- `server/`：Rust 后端、项目管理、AI CLI 调度、发布接口、context compiler、RAG/符号索引能力。
- `android/`：一龙 APK 客户端，负责对话、项目入口、任务进度、APK 更新。
- `scripts/`：后端发布、APK 发布、预检、worktree 清理等自动化脚本。
- `default-project-docs/`：给用户新项目种下的 AI 工作入口和默认规则模板。
- `.github/`：本仓库 AI 代理的规则权威、专项 instructions、skills。

## 产品演进方向

当前长期产品链路定义为：

```text
用户讨论需求
  -> 一龙 AI 持续澄清目标
  -> 预言家 AI 低成本生成 demo（按不确定性触发）
  -> 用户确认、修改或放弃方向
  -> 总调度 AI 生成 Matter 并选择 AI-to-AI Skill
  -> Skill Agent / Worker Bot 在隔离工作区执行
  -> Reviewer / Verifier 验收
  -> 构建、发布和分发
  -> 将成功流程、偏好和质量数据沉淀为 Context / Taste / Skill
```

短期先验证“官方 Skill + 预言家 demo + Matter 执行”的闭环。长期把官方 Skill 做成平台内部可版本化、可审核、可组合、可升级的能力仓库，并把生成的应用接入独立的版本分发和二次创作体系。产品不建设 Skill 市场，不接收第三方 Skill 上架，也不为 Skill 提供买卖、订阅或创作者结算。详细路线见 `docs/ai-to-ai-skill-oracle-roadmap.md`。

## 当前项目理解能力

项目已经不只是基础 repo map。当前已有分层上下文系统：

- 项目规则层：`AGENTS.md`、`.github/copilot-instructions.md`、`.github/instructions/*.instructions.md`。
- Repo map / context compiler：`server/src/context_compiler/` 生成文件树、摘要、repo map、context pack 和任务上下文包。
- Rust 代码结构层：Rust 符号扫描、rust-analyzer/LSP 事实、`semantic_facts.jsonl`、`lsp_locations.jsonl`。
- 符号索引层：`symbol_index.sqlite`、符号/边/lookup、impact pack、task pack、graph query、retrieval eval。
- 混合检索层：关键词/符号/chunk/vector 多路召回，`repo_context_task_pack` 和 `repo_symbol_search` 面向 agent 使用。
- 向量层：已有本地 `local-hash-v1` embedding provider 和 `embeddings` 表；schema 支持同一 chunk 存多个模型向量。
- 验证闭环：patch plan、dry run、review、verification、repair context、目标测试建议。

## 当前共享 Android 真机（项目记忆）

`elon-self` 项目当前共享真机如下。PC 节点和 AI 排查无线 ADB 时，以硬件序列号作为稳定身份，不要把局域网 IP、无线 ADB 端口或 mDNS 冲突后缀当成永久身份。网页切换手机时必须按稳定身份隔离画面、草稿和 LIVE 会话，不能继续显示上一台手机。

| 显示名 | 厂商 / 型号 | Android | 硬件序列号（稳定身份） | 最近可用无线端点 | 当前状态 |
|---|---|---|---|---|---|
| `Xiaomi 23116PN5BC` | `Xiaomi` / `23116PN5BC` | Android 16（SDK 36） | `e0d909c3` | `192.168.31.171:5555`（2026-07-14） | 当前真机调试主设备 |
| `HONOR AAK-AN00` | `HONOR` / `AAK-AN00` | Android 16（SDK 36） | `ASUJ6R6324002425` | `192.168.31.83:36115`（2026-07-13） | 已记忆，离线时不得阻塞在线设备 |

HONOR 最近发现的 TLS mDNS 选择器为 `adb-ASUJ6R6324002425-ZDy0od (3)._adb-tls-connect._tcp`；该选择器的冲突后缀会变化。

连接约定：网页先使用本机已授权节点；本机没有目标手机时，自动发现项目内已授权且在线的“共享设备主机”，通过项目鉴权的云端 WebSocket 隧道使用其真机捕获和 Live Runtime，不再要求每台浏览电脑重复 ADB 授权。设备主机仍按硬件序列号同步共享档案，先通过 `_adb-tls-connect._tcp` 发现当前端点，再尝试最近端点；发现新端点后回写共享档案。`(3)`、IP 和端口都可能变化。至少一台托管电脑必须完成手机端无线调试配对/ADB 授权、运行最新版 Windows 节点并开启公共开发共享；禁止在项目文档、云端档案或 Git 中共享 `adbkey` 等私钥。配对、忘记设备、切换 `tcpip` 等高权限操作只允许在托管电脑本机执行。PC 本地节点管理端口也可能位于 `7799-7819`，网页应使用 `node_admin` 或自动探测结果，不能写死 `7799`。完整运行边界见 `docs/shared-android-device-host.md`。

并发约定：同一台公共真机允许多台 PC 保持无线 ADB 连接和只读查看，但安装/启动调试 APK、实时拖动、撤销、构建验证等写操作必须先取得项目设备租约。同一硬件序列号同一时刻只允许一个浏览器标签页持有租约；页面每 10 秒续租，断线或关闭后最迟 45 秒自动释放。单个 PC 节点内的 Codex 会话仍在各自 conversation worktree 并行开发；只有明确 ready、工作区干净且有 Git 提交身份的候选，才能按审计顺序进入节点托管的临时集成 worktree。合并、构建和安装按“项目 + 硬件序列号 + 节点”固定槽串行，并用构建代次淘汰旧候选。单台手机只有一个前台画面；其他用户必须看到占用者、共享预览所有者和当前代次，不能静默抢占。

调试包约定：每个稳定 Windows 节点在真机上只有一个 Launcher 调试包：`com.elon.app.uituner_<8位节点指纹>`，指纹只从持久 `install_id` 派生并跨客户端更新、重启保持不变。PC 工作台、HTTP、MCP 和旧调用即使传入 `.uituner`、`.uitest` 或 `.uitest_anim`，真机也必须归一到该固定包，禁止以新后缀作为失败兜底；显式隔离后缀只允许模拟器。正式包 `com.elon.app` 和名称“一龙ai”不变。构建输出保留在代次 worktree 中，协调器只把校验过包名、标签、签名、版本和 SHA-256 的当前 APK 原子暂存到内容寻址目录，再覆盖安装。最近成功版本默认关闭，只有任务显式启用才记录、推进和校验；关闭时不阻塞构建、ADB 安装或收尾。冲突、离线或安装失败时手机当前版本保持不变；历史杂包只报告，不自动卸载用户手机应用。

## 还没完全做到的部分

这些是后续最有价值的完善方向：

1. 真正的远程 embedding provider：接入用户自带 API key 和模型配置，把云端 embedding 写入现有 `embeddings(chunk_id, model)`；设计边界见 `AI_ARCHITECTURE.md`。
2. 多模型检索策略：按项目/用户选择 embedding 模型，并在 task pack 中记录模型来源、维度、召回质量。
3. 索引增量更新：文件变更后只重算受影响 chunks、symbols、embeddings，而不是全量重建。
4. 用户项目默认 AI 文档：新项目应默认具备 `AI_PROJECT.md`、`AI_INDEX.md`、`AI_RULES.md`、`AI_TASK_TEMPLATE.md` 和 `.aiignore`。
5. 检索质量回归集：把真实任务沉淀为 retrieval eval cases，防止后续改动降低召回质量。

## 常用完成标准

所有写任务遵守 `.github/copilot-instructions.md` 的 `WF-START` 至 `WF-REPORT`。后端运行代码默认通过 `publish-server.*` 发布并以 `Server` 收尾；用户明确只同步代码时才用 `CodePushed`。Android、PC 前端和 Win 节点的默认发布边界也以共享契约的“完成类型”为准，不在本项目说明中复制步骤。
