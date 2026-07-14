---
name: cloud-apk-dev
description: >
  一龙云端 APK 平台的代码修改、验证、Git、后端部署、PC 节点和 APK 发布工作流。
---

# Cloud APK Development Skill

先读 `AGENTS.md`，按其中路由只加载当前任务需要的专项文档。共享生命周期由 `.github/copilot-instructions.md` 的 `WF-START` 至 `WF-REPORT` 定义，本 Skill 不复制命令清单。

## 适用范围

- Android、Rust 服务端、PC 前端或 Win 节点代码修改。
- 提交、push、部署、APK 发布、版本和线上验证。
- `/elon-dev-task`、`/elon-apk-release` 及对应 planner/implementer/reviewer 工作流。

## 执行要求

- 使用预检脚本给出的 `EDIT_ROOT` 和 `FINISH_COMMAND_*`，脚本信号优先。
- 按 `WF-FILES` 在创建文件时确定“提交、临时、精确忽略或保留报告”，不在收尾时凭文件名猜测。
- 只 stage 当前任务交付物；push 冲突只按 `WF-REBASE` 处理。
- Rust 日常验证走 `cargo-dev.*`；不要并行写同一 target。
- 后端版本由 `publish-server.*` 的 release claim 注入，不改 `server/Cargo.toml` 版本。
- APK 版本由 `publish-apk.*` claim，脚本会还原临时 `build.gradle` 变化，不生成版本提交。
- Win 节点用户可见变更使用 `publish-node-agent.ps1` 并验证节点版本接口。
- 新机器 Android 构建异常才读取 `docs/android-setup.md`，不要把机器级 Gradle 配置提交到仓库。
- 发布被更新主线或服务器版本超越时停止追车，保留代码已进入远端主线的业务结论。

## 完成

根据共享完成类型选择 `CodePushed`、`DocsOnly`、`Server`、`PcFrontend`、`NodeAgent` 或 `AndroidFeature`。统一收尾必须输出 `FINALIZABLE=true`；最终报告分别说明业务完成、主基线同步、未知未跟踪文件告警和 worktree 回收状态。
