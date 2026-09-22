---
version_status: current
reviewed_at: 2026-09-23
---

# 每群 ChatGPT 项目与长期会话

## 当前边界

本批补上群聊执行器的项目绑定路径，不把离线通过当作手机验收。

- `capability_id`: `android_chatgpt_group_project_v1`
- 代码：项目传输、服务器绑定租约、Android 长期执行、显式选区加入记忆已接线。
- 验证：官网公开脚本契约、Node 定向测试、生产 SQL 测试和后端编译通过；APK 最终测试、发布及手机真实链路另记。
- 未完成：Windows 执行器使用同一项目绑定、项目内多个主题的选择界面、官方分享链接导入/分叉、群图片附件分析仍不在本批完成范围。
- 不能据此宣称整个历史 Goal 已完成，或“训练记忆”等于训练模型。

## 实现

1. 一龙用户、不可变 `group_id`、ChatGPT 用户与 workspace 的 SHA-256 指纹组成绑定主键。群名只用于首次项目标题，改名不重建。
2. Cookie、Bearer 和设备凭证留在 WebView。指纹由已审核官网运行时的 user/account 双重一致性校验产生，不包含临时令牌或设备 ID。
3. V306 表 `group_chatgpt_projects` 保存 generation、project ID、conversation ID 和有界操作租约。事务中检查群成员，跨设备不能并发抢同一绑定。
4. 创建先登记 `creating`，再发一次官网 POST。创建超时保留未知结果，后续先按 instructions 内的稳定绑定标记查找，绝不自动重发创建。
5. 只允许 `project_v2` 项目范围记忆，不默认使用 global 记忆。读取真实资源，核对 ID、写权限和 memory scope 后才导航。首次创建和未知恢复额外核验绑定标记；已有绑定按稳定 ID 读取，用户修改项目说明不会打断使用。
6. 既有会话通过私有正文接口核验 `conversation_id`、`gizmo_id` 和非临时状态。发送前再次确认账号范围和准确的当前路由。
7. 普通群分析使用长期项目；ChatGPT 多选确认页默认勾选“加入本群 ChatGPT 项目”，明确提示会结合本群已有记忆。用户取消勾选后进入严格选区临时模式，不读取旧会话。Google 和工作 AI 不显示该选择。
8. 发送前记录历史消息 ID，长期请求带唯一请求标记，避免把旧的同文问题回答当成本次结果。
9. 完整回答与绑定写回分离：后者失败不扣留回答；本机先保存无正文的会话 ID 回执，下次取得租约后重新核验并恢复。最长等待绑定写回 20 秒，不无限阻塞群回复。
10. 原项目连续读取返回 404 后，需要用户明确确认重建，新 generation 不复用旧会话。401、403、429、超时和 DOM 未就绪均不触发重建。404 也可能表示不可访问，UI 不断言是用户删除。

## 协议证据

2026-09-23 从已登录手机的 `runtime_assets` 只读探针获得当前公开资源地址，再读取公开 CDN JS；未导出用户请求、凭证或聊天正文。

- shared: `4813494d-oyru480whcw6vje0.js`
- 创建弹窗: `ee0c200a-jqrfo2gcauarts2t.js`
- 公开资源 SHA-256：shared `011f7cfa6034a3f41692762815a08b86f07b0bd34c4070651a0cd7527cdcf975`；创建弹窗 `ea24e05828137ed720a1d9d53d8fa033d9fd65cba9d5449e88e83a378f6ab761`。
- `wRt` / `fm` 的 mutation 使用 `safePost('/projects', {requestBody})`。
- 弹窗提交字段：`instructions`、`name`、可选 `emoji/theme`、`memory_scope`。
- 返回对象由 `kZ` 读取 `resource.gizmo.id`；正式传输再 GET 资源确认，不能仅凭 HTTP 200 绑定。
- 旧 `CRt` / `/gizmos/snorlax/upsert` 先要求已有 ID 并读取已有资源，是更新而不是创建。
- 项目目录为 `/gizmos/snorlax/sidebar`，仅查询 owned 项目并有界分页；未找到不等于可以重发未知创建。

## 验证记录

- `scripts/test-chatgpt-web-group-project.js`：9 项通过，含账户/文档切换、未知写不重试、权限与记忆范围、唯一标记恢复及已有项目说明编辑兼容用例。
- 同批现有项目附件兼容测试：25 项通过。
- `group-ai-selection-harness` 引用生产 SQL 模块：3 项通过，覆盖成员/账号/群隔离、租约过期、创建未知、会话去重、generation 和明确重建。
- `cargo check --manifest-path server/Cargo.toml`：通过。
- 最终 Android 定向测试：36 项，XML 核验失败/错误/跳过均为 0，包含线程回执恢复及绑定失败仍交付回答。日志：`group-project-final-unit-20260923-034525-110`。
- 首次 Gradle 调用误用了多模块 `testDebugUnitTest`，辅助模块因没有匹配用例报错；改为 `:app:testDebugUnitTest`。不是将失败日志当作通过。

## 发布协调

V305 群 AI 助手由另一任务维护。本批 V306 与 V305 必须都进入发布源码后再部署，避免迁移版本乱序。APK 由合并后的已提交源码完整构建，不使用 `SkipBuild`。

手机验收至少核对：首次创建并回群、第二次复用同一个项目/会话、严格选区不进入项目、取消及网络失败不重复创建。没有真实回执前，保留验收未完成状态。
