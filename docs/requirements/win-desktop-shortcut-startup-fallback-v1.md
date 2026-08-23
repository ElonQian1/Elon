---
version_status: current
requirement_status: accepted
owner: win-platform
---

# Win 桌面壳快捷键启动降级 V1

## 目标

Windows 桌面壳启动时，如果 `Ctrl+Alt+E` 已被系统或其他应用占用，主窗口仍必须正常创建并显示；全局快捷键只作为可选能力，不能成为启动前置条件。

## 非目标

- 不更换现有快捷键组合。
- 不静默抢占其他应用已经注册的系统快捷键。
- 不改变托盘、窗口、WebView2 或官方 AI 网页会话行为。

## 验收标准

1. 全局快捷键注册失败时记录诊断信息并继续完成桌面壳启动。
2. 快捷键可用时保持现有 `Ctrl+Alt+E` 呼出/隐藏行为。
3. 源码合同回归明确禁止用 `?` 把快捷键注册失败传播为整个 Tauri 启动失败。
4. Windows 发布后桌面进程保持运行，主窗口标题为“一龙工作台”，节点发布身份与 `origin/main` 一致。

## 实现范围

- `desktop-shell/src-tauri/src/main.rs`
- `server/src/node_client_launcher/launcher_tests.rs`
