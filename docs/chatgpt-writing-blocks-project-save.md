# Writing Blocks 项目会话写回

能力 ID：`android_chatgpt_project_writing_block_save_v1`。

- `code_status=implemented`
- `verification_status=offline_verified`
- `production_status=pending_grouped_build_acceptance`

这是既有原生编辑器与写作块保存的范围扩展，不新增编辑器或复制另一套传输。普通会话已通过 `1692` 的能力继续复用；本批不能继承它的项目验收结论。

## 支持范围

- 个人账号下已加载的、自有项目会话中的完整 `:::writing` 块。
- `/g/g-p-<32hex>[-slug]/c/<uuid>` 路由，以及官网以 `/c/<uuid>` 展示同一项目会话的情况。
- 保存票据同时绑定页面文档、账号、会话、项目 ID、当前分支和原块正文。
- 原生编辑/复制/导出仍立即可用；保存准备与确认不要求输入框或 DOM 发送按钮就绪。
- 项目 ID 来自官网当前已加载会话状态，并与私有历史响应 `gizmo_id` 逐次核对；不是从项目名或 UI 标签推断。
- 保存一次 POST，回读一致后通过既有官网单节点更新动作更新正文，再刷新原生缓存。项目路由的结构化回显只读内存，不导入模块、不请求网络、不重载页面。

临时会话、共享会话副本、跨工作区、未知 GPT 类型、库文件联动与无明确源身份的块仍拒绝写回，保留本机副本编辑与导出。后续 [typed widget 扩展](chatgpt-writing-blocks-widget-save.md)已离线验证，真机同样待验收。项目所属账号、分支或项目关系在异步期间改变时不继续写；POST 后结果不确定时保留核对状态，禁止重发。

## 协议证据

复用保留的 `web_20260912` 官方公开源码，不执行下载的 bundle：

- `a965fc59-fzrm5l4zirdbhwph.js` / `nc`：唯一消息写作块保存体是 conversation / message / block ID、content、index、variant、title、metadata 与更新时间，没有额外项目字段，也没有项目专属写端点。
- `2120deb9-jv4295pp9oyi96ww.js` / `Gr`：用当前会话对象及消息 ID 更新单节点写作块 metadata，不切回普通会话或整段重载。
- 哈希钉住与字段核对复用 `scripts/test-chatgpt-writing-block-public-evidence.cjs`。这些证据只支持客户端合同，不能替代真实账号下项目写入及原生 UI 的验收。

保留原接口的并发限制：`updated_at` 不是服务端 compare-and-swap 条件，前后核对不能承诺跨设备同时编辑完全无竞态。

## 本批验证

- `writing-project-contract-20260913-111333-663`：54 项通过、无跳过，含实际保留官网源码检查。
- `writing-project-shared-regression-20260913-111637-268`：19 项通过、无跳过，覆盖历史、流式和块清点共享路径。
- 新增项目写回测试串联真实策略、上下文、私有请求协调与结构化回显，覆盖项目路径、普通路径承载项目、唯一写入、项目迁移、分支/身份变化、临时及共享范围拒绝、POST 后只读恢复。
- Android 路径与回执测试增加项目路径正反例，验证原生层不会提前拒绝合法请求，也不接受项目首页、共享页、查询参数或伪造路径。
- `writing-project-android-20260913-111502-877`：Release Kotlin/Java 编译通过，318.7 秒。首次测试筛选仅运行到 5 项写回协议测试；随后修正模型测试的完整包名，复用编译产物补齐模型、连续性和 UI 契约测试，没有重新打包。
- `writing-project-native-model-20260913-112120-804`：19 项原生测试通过、无失败/错误/跳过，23.4 秒。源码提交 `070a43fff`；主线文档更新后的 rebase 经 range-diff 确认源码补丁不变，复用原验证。

## 集中验收

沿生产“一龙 AI → ChatGPT”进入受控项目会话，打开已有 Writing Block，修改副本并导出，再显式保存一次。核对服务端回读、原生重开正文、项目归属和无页面重载；恢复原页面及状态。通过后才将本能力标记 `completed / production_verified`，不重测已完成的普通代码块导出与普通会话写回。

本批不单独打包、发布或操作用户数据。仅改源码并提交主线，和其他待验收能力统一编译发布。
