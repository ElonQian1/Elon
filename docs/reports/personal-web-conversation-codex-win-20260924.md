---
version_status: current
reviewed_at: 2026-09-24
---

# Codex 接入与 Win 会话读取恢复

[使用说明](../personal-web-conversation-reader.md) · [需求](../requirements/personal-web-conversation-reader-v1.md)

## 现场基线与修改

本轮开始时正式节点运行 `522a246`，`read_conversation` 未加载；`c82dfc5` 候选等待桌面壳退出。通过现有精确更新重启动作后，正式节点回读 `c82dfc5`，读取动作实际创建了 ChatGPT WebView。随后发现完整 UI 适配器初始化失败，原读取器因依赖该适配器的 transport 而返回 `reader_unavailable`。

新增 Win 启动模块，仅启动正式安装目录内的一龙启动器，保留其更新锁、单实例和节点恢复逻辑；节点或宿主缺失时有界等待，多实例不随机选择，旧版本明确报告需升级。读取独立加载既有页面内认证和 JSON 请求模块，登录仍在官方 WebView 完成，登录失效时显示该窗口。此修改不代表完整聊天 UI 的所有适配器已经恢复。

Codex 与 Claude 任务均按明确会话链接注入同一个 stdio MCP；Codex 使用进程级 TOML 配置，保留其他 MCP。桌面端注册入口保存内容寻址代码副本，通过官方 CLI 登记授权范围，避免任务 worktree 回收后断链。工具新增不返回正文的 `web_conversation_connect`。

## 验证与边界

- 17 项 Node 测试覆盖真实 stdio/HTTP 替身、冷启动、超时、旧版本、多实例、断线重连、只读认证独立于 UI、授权及分页、不可变注册资产。
- 生产 CLI 配置与研究队列模块的 Rust harness 测试通过；桌面原生生产编译检查通过。
- 本机 Codex CLI 已能回读 `yilong_web_conversations` 配置及 150 秒超时；当前授权仅包含用户指定会话。配置存在不等于当前 Codex 任务已热加载工具。
- 本文在新版发布前编写；新版正式身份、完整会话分页、真实冷启动、Claude 模型调用和 APK 现场读取须分别补证，不能用本记录代替。
- 图片/PDF 字节、API 模型自动接入仍未覆盖；不宣称已读懂截图。

真实会话正文、登录凭据和附件内容不写入本报告或测试夹具。
