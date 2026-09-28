---
version_status: draft
reviewed_at: 2026-09-28
---

# 独立子项目 Runtime 持久绑定评审

本文件记录量化子项目的诊断和待实施方案，不修改现行身份规则，不代表持久重绑已通过。

## 失败层与证据

业务回执的源码为 `6a9e83d0c8896e7db8f006a636f18a34611d8a34`，gen 3，
隔离包 `com.elon.quant.uitest_398d0d9d`。构建、安装和 Runtime 连接已经成功，
CAPTURE 阶段失败为 `RUNTIME_REBIND_PERSIST_FAILED / RUNTIME_BINDING_MISSING_ROOT`。
业务随后从真实 Runtime 读回 Launcher 23 个语义节点。这些证据只能证明其各自成功阶段。

`build_verify.rs` 在 Runtime 准备完成且需保留会话后调用 `runtime_binding::persist_verified`。
后者通过 `project_identity` 读取当前源码 fingerprint 和 `resolve_root_task_id`。
当前身份解析的可信来源仅为：

1. 同一规范 workspace 的持久 task，且 supervision contract/lineage 校验通过；
2. 没有上述候选时，使用 `node_agent_codex_task_contract_identity` 的带锁主工作流
   finish contract，或它明确支持的历史 Codex task 分支约定。

`scripts/templates/subproject-lite-workflow/README.md` 明确规定轻量子项目不建 worktree、
不上锁、不依赖主项目工作流。因此本次是身份来源覆盖缺口；不能把独立项目没有主流程合同
解释为 APK 构建失败。当前解析器没有接入已登记项目身份的分支，单纯重新登记项目也不会
使本版本的 `persist_verified` 自动通过。

## 当前可做与不可宣称的事项

保留现有 LIVE 会话及真实截图/语义证据，持久重绑能力仍标记失败。
不为消除报错伪造 supervision lineage、手改绑定 JSON，或替子项目补造主流程锁/合同。
也不以忽略 persist 错误的方式把整个 prepare 标记完成。

业务已明确占用模拟器，本诊断未操作其设备、会话或节点生命周期。

## 建议的正式接入

服务端现有 `store/project_identities.rs` 和 `store/pc_project_binding.rs` 已有
owner/project/node/workspace 登记模型，可作为新增可信来源的接入点。
这些模块证明“有登记模型”，并不证明当前节点已经持有足以授权重绑的凭证。

建议新增独立 `REGISTERED_PROJECT_TASK` 身份类型，与监督 root/主流程 contract 区分：

1. 经已认证节点通道向服务端核实 owner、projectId、nodeId 及绑定 workspace；
   验证规范项目根、Git common dir/origin、当前任务所有者与工作台设计 task 的关联。
2. 服务端签发或通过现有可信通道保存有版本、作用域和吊销语义的绑定凭据。
   不能把本机路径哈希、任意 taskId 或 `.elon` 中调用者可写的声明视作授权证明。
3. 构建/连接后再核对真实 package/device/source revision/generation，持久记录
   `identityKind`、可信 issuer、project/task scope 和 provenance，再允许同一身份恢复。
4. 身份签发先于昂贵构建，缺失时给出明确的登记/授权操作；恢复阶段仍重新验证来源。
   不改变已具备合法 root 的历史路径，也不把未知身份默认降为可信。
5. 绑定 store 采用新 schema，并显式读取旧 schema；source、workspace、owner 或任务范围
   变化即拒绝复用。一个项目有多个身份候选时必须报歧义，不任选一个。

需先确定节点侧如何读取和验证服务端登记，补齐工具入口及凭据失效规则，再实施。
这是待实现的正式路径，当前没有可安全执行的单条 root 回填命令。

## 最小回归与后续验收

- 合法已登记 lite 项目首次准备、节点重启后恢复、同任务重复调用。
- 未登记/错误 owner、错误 node/workspace、跨项目 task、伪造凭据、吊销/过期凭据。
- 多个候选、Git 根漂移、源码 revision/generation 不符、控制 Runtime、活跃 patch。
- 旧 supervision/finish-contract 绑定继续有效；历史 schema 可读且不扩大信任范围。
- 模拟器空闲后，通过正式工具完成真实构建、捕获、持久化和新连接重绑，分别保存阶段证据。

后续源码实现已接入现有账号/节点项目登记接口，逐次在线校验登记 checkout 与当前 worktree
的 Git common dir/origin；不增加离线凭据签发，也不伪造 task root。绑定记录使用
`REGISTERED_PROJECT_WORKSPACE` 独立来源，保存已验证 build/generation，旧 schema 1 可读。
首次绑定要求当前无 Patch 的源码证明；恢复需要同源码、同登记身份和同 build 的历史记录。
登记缺失、歧义、失联或身份变化均失败。源码专项测试和节点编译通过，尚待发布及真实重绑验收。
