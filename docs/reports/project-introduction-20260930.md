---
version_status: current
decision_status: evidence
reviewed_at: 2026-09-30
implementation_status: pc_published_mobile_runtime_deferred
---

# 通用项目介绍框架交付记录

## 范围与行为

- 主项目和子项目复用定位摘要、团队协作、能力清单、适用人群、使用与更新详情、频道资料及交付入口。
- 加入方式分别说明邀请、开放加入、申请审批和只读体验；参与范围始终以角色权限为准。
- Windows 团队区连接既有成员管理页面，讨论与开发按钮连接既有频道，不创建任务或扩大权限。
- 所有子项目能力条目保留，取消四项截断；没有介绍配置时明确提示，不补造能力。
- 没有配置开发或构建频道的项目不再显示对应空步骤。量化专用启动与成员下载保持原入口和权限。
- 主项目文案强调团队共同开发、自然语言任务、多端接力、已有项目接入、项目知识与交付追踪。
- Windows 官方主项目介绍随前端读取 `.elon/project-landing.json`，修复线上旧节点快照继续覆盖新文案的问题；子项目仍使用自己的 landing。
- Android 新介绍组件在截图区之前呈现，使用现有 Material 语义颜色，并支持详细内容展开；移动 PWA 使用同一组 landing 字段、安全文本节点和可展开详情。

## 已执行验证

| 验证 | 结果与边界 |
|---|---|
| PC 类型检查、生产构建、lint | 通过 |
| 现有项目首页完整回归 | 通过，包括量化下载和专用启动合同 |
| 真实浏览器离线回归 | 通过：主项目旧快照、子项目完整内容、四种加入方式、成员和频道操作、详情展开、空配置、320/390/768/1280 宽度无横向溢出 |
| PWA 实际介绍渲染器 | 浏览器回归通过：文本转义、重新渲染、清理与展开；完整生产登录页面未新增验收 |
| PWA 源码检查 | 通过 |
| V2 规范检查 | 自测 12 项和正式治理检查通过；不等于原生视觉验收 |
| Android 内部构建 | `:app:assembleDebug` 通过，含新的原生离线 Preview 场景 |
| 原生运行验收 | 延期；工作台运行准备两次超时，设备列表为空，关闭自动启动后明确返回没有空闲模拟器 |
| 用户验收 | 未执行 |

截图由离线生产组件生成，保存在本次桌面会话附件；示例数据不代表真实项目任务已完成。
可重复验证脚本：`scripts/test-project-introduction-browser.mjs`。
PC 预览：开发环境 `/pc/project-introduction-preview.html`，不进入生产构建。
原生 Preview：`elon.project.introduction`，场景为 `main/child/empty/readonly`。

## 发布与待续工作

Windows 通用框架已通过独立前端发布流程；最终前端身份、兼容后端身份和发布时间以
`/pc/assets/release.json` 的回读为准。后端 API 沿用现有兼容版本。

Android 和移动 PWA 改动已提交、推送，尚未发布。本次信息层级改动不适用低风险视觉快速通道；
后续应绑定本任务源码，准备空闲隔离模拟器，验证浅深主题、字体放大、展开和成员入口，
补原生运行与跨端证据后，再走既有正式发布和装机流程。
主项目移动端介绍内容还需通过既有项目首页同步路径确认当前 landing 快照。

功能工作流工具在本次会话不可调用；未手工改写 `.elon/project-features.json`。

| UI 收尾字段 | 当前值 |
|---|---|
| FIT_RUN_STATUS | NOT_RUN；没有目标截图，也没有原生运行证据 |
| FINAL_VISUAL_LOSS | NOT_MEASURED |
| VISUAL_ACCEPTANCE_THRESHOLD | NOT_EVALUATED |
| CROSS_PLATFORM_VISUAL_PARITY | NOT_VERIFIED |
| BUSINESS_DELIVERY_READY | false，指移动 UI 工作台闭环；不否定独立 PC 发布 |
| PLATFORM_EVOLUTION_PENDING | false |
| EVOLUTION_THREAD | NONE |
| REAL_DEVICE_STATUS | NOT_REQUESTED |
| ANDROID_RENDERER | UNAVAILABLE |
