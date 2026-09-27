---
version_status: current
decision_status: evidence
reviewed_at: 2026-09-28
implementation_status: partial_not_released
---

# 双 APK 桌面接管与验证记录

本文件记录证据，不替代需求和设计权威。用户要求撤销旧视觉约束、学习移动技能并系统重构两个 APK；当前已接入规范和公共基础，并改造代表页面，尚未完成全应用验收。

## 来源与范围

接管 ChatGPT 的主仓草稿 #6、量化草稿 #212，保留其 V2 规范治理工作。桌面端核对当前代码、AI_CURRENT、项目规则、命中领域文件和本地技能后实施。没有假定云端任务具有本机未提交文件或全部本地记忆；本机也没有可读取的 `.codex/memories` 目录。

新审查入口：[主项目 #7](https://github.com/ElonQian1/Elon/pull/7)、[量化 #213](https://github.com/ElonQian1/yilong-quant/pull/213)。均为草稿，旧 PR 未自动关闭或合并。

8 个外部技能已安装，版本、选择与不适用规则见 [采用矩阵](../design/mobile-skill-adoption.md) 和 `mobile-skills.lock.json`。安装不等于视觉验收；两个生产 APK 均为 View/XML/WebView，不以学习 Compose/Expo 技能为由重写框架。

## 已实施

- 两仓共享同版 V2 规范与语义 token，生成浅深 Android 资源；主仓同时生成 PWA 主题，清除内联旧颜色覆盖。旧名称保留兼容映射，不再作为视觉权威。
- 两仓主题跟随系统并提供手动选择。量化统一 Material 按钮、48dp 交互、反馈层、主题色；机器 ID 与人类读屏语义分离，检查工具和测试同步迁移。
- 主仓项目列表移除按截图缩放的几何规则，增加大字体重排；聊天气泡与文字成对使用语义色。广场卡片使用内容高度，操作区独立排列；加入状态不再伪装成运行状态。
- 主仓新增离线调试预览场景，复用生产项目列表和消息 XML；旧配色/装饰锁定测试改为语义和行为检查。保留业务回调及权限边界。
- 系统级主题/公共组件改造不能走先发布后视觉验收的快速通道。补齐 Windows CI 长路径、Android SDK 包声明和既有导入路由；删除一处失效的 PC lint 禁用注释。

## 验证证据与限制

量化提交 `c243e7647f2239b684e61cc7f46d578bedfa3e56`：最终 Android 回归 696 项，0 失败、0 错误、10 跳过，调试 APK 成功。SHA256 `D956128BD842DDEA39472D7597AC6F727D5979C527DC5FC11DA9FC604B663D59`。治理 12/12、生成检查和收尾契约 14/14 通过。完整验证在 Rust 缓存磁盘门禁停止；远程工作流 steps 为空，不能推断是费用、额度或代码问题。

主仓：治理 12/12、生成资源、PWA 主题行为、快速通道、Stitch 导入、预检/收尾工作流及提示资产审计通过。整仓 Android 最近一次完整回归执行 2381 项，21 项失败；后续针对视觉改动补跑的结果以 PR 更新为准。失败包含旧实现/文案断言、Windows 路径/符号链接差异及聊天集成契约，尚未在原始基线全部复现，不能统称历史问题。Rust 格式检查通过，实际 Rust 测试尚未完成。

主仓 `6a9402d` 的远程治理、PC 前端、Sui Move 通过；Android SDK 设置和 Rust 作业的导入路由检查失败已有修复，后续运行以 PR 为准。不能把一次构建或选定测试通过说成全仓通过。

运行工作台返回模拟器资源无空闲，状态 `VERIFICATION_DEFERRED`，建议 `WAIT_FOR_IDLE_RENDERER`；没有确认的能力缺失。已停止重复分配，不接触用户真机或绕过租约。没有新版本截图、像素匹配、键盘、性能和完整导航验收。

## 接续顺序

1. 保留两仓独立工作区和 PR。主仓基于 `b9dc0be39`，本机同步主线已前进至 `e569070b1`；已知交集为 `server/src/assets/web_page.html`，合并前必须审查新聊天/分享相关改动，不能覆盖。不要自动追赶重置主线。
2. 在稳定提交上恢复隔离模拟器，检查浅深主题、320/411/600dp、长中文、200% 字体、键盘、加载/离线/错误状态。按主仓“项目→对话→结果”、量化“网格→详情→离线草稿”逐条保留能力证据。
3. 主仓尚有 126 个生产 Kotlin/XML 文件含颜色字面量，包含合法图标值；需按使用语义逐项审查，不能以数量直接判定缺陷。继续检查广场、个人、会话附件等页面及 PWA 跨端状态。
4. 修复相关测试并确认其余失败的基线归属；解决量化 CI 未启动及 Rust 验证环境阻塞。所有必要检查与原生验收完成后再考虑合并、正式构建、发布和安装。

## 收尾状态

`FIT_RUN_STATUS=not_run`；`FINAL_VISUAL_LOSS=unavailable`；`VISUAL_ACCEPTANCE_THRESHOLD=unavailable`；`CROSS_PLATFORM_VISUAL_PARITY=unverified`；`BUSINESS_DELIVERY_READY=false`；`PLATFORM_EVOLUTION_PENDING=false`；`EVOLUTION_THREAD=none`。

主仓统一收尾返回 `FINALIZABLE=false`（提交尚未进入 origin/main），量化返回 `head_not_in_origin_main`。两仓均未合并、未正式发布、未装机；草稿源码交付与系统重构完成分开记录。最终工作区清洁度与最新检查结果见 PR 和任务回复。
