---
applyTo: "pc-frontend/**/*,server/src/web.rs,server/src/router.rs,scripts/publish-server.*,scripts/start-pc-frontend-dev.ps1"
---

# PC 工作台前端规则（收尾期 / 新功能期）

> **当前状态（2026-06-26）：迁移已完成。**
> - `/pc` 指向 `pc-frontend/`（Vite + React + TypeScript），由 `$DATA_DIR/pc-next-dist/` 服务。
> - 所有旧 `server/src/assets/pc_*.{html,js,css}` 文件**已删除**，禁止重新创建。
> - 所有旧 `pc_app_*` Rust 处理器函数**已删除**，禁止重新添加。
> - **新 PC 功能只能进入 `pc-frontend/src/features/`**，不允许写回旧 HTML/JS 模式。
> - `/pc-legacy` 只允许作为发布脚本从历史提交 `d1f89950` 导出的静态对照快照，不允许把旧源码重新提交回仓库。

## ⛔ 不允许的操作（给所有 AI 代理的硬规则）

1. **禁止在 `server/src/assets/` 下新建任何 `pc_*.{html,js,css}` 文件**
2. **禁止在 `server/src/web.rs` 里重新添加 `pc_app_*` 系列函数**
3. **禁止在 `server/src/router.rs` 里重新注册 `/assets/pc_*` 路由**
4. **禁止把 PC 业务逻辑写成内嵌的字符串 HTML**

允许的例外：`scripts/publish-server.*` 可从历史提交 `d1f89950` 导出只读静态包到服务器 `$DATA_DIR/pc-legacy-dist/`，供甲方和开发者通过 `/pc-legacy` 与新版 `/pc` 对比；该静态包不得进入 git。

---


## 迁移方向

- 目标技术栈：`Vite + React + TypeScript`，目录为 `pc-frontend/`。
- 旧 PC 工作台资产：`server/src/assets/pc_app.html`、`pc_app.js`、`pc_app.css`、`pc_app_*.js`、`pc_app_*.css`、`pc_project_landing.*`、`pc_voice_project.*` 等 `pc_*` 静态文件。
- 旧移动网页版：`server/src/assets/web_page.html` 仍按 APK/Web 同步规则维护，不纳入本次 PC 框架迁移，除非任务明确要求移动网页版。
- 服务端入口：`server/src/web.rs` 和 `server/src/router.rs` 负责 `/pc`、`/pc-next`、静态资源和构建产物托管。

## 硬规则

1. **新 PC 功能默认进入 `pc-frontend/`**：涉及 PC 工作台的新页面、新复杂交互、新状态管理、新 API 面板，默认使用新前端工程实现。
2. **旧 `pc_*` 静态资产进入收缩期**：旧原生资产只允许 bugfix、小型兼容桥接、临时入口跳转和迁移删除。超过 30 行的新逻辑必须先说明为什么不能进入 `pc-frontend/`。
3. **禁止长期双写**：同一业务能力不能在旧 PC 和新 PC 两套实现里长期并行扩展。迁移某个模块后，必须记录旧模块的关闭或删除计划。
4. **`/pc` 必须稳定可用**：迁移期先使用 `/pc-next` 承载新前端验证；功能覆盖并验证通过后，再把 `/pc` 切到新前端，旧版临时保留为 `/pc-legacy`。
5. **构建链路必须接入发布流程**：新增 `pc-frontend/` 后，必须定义 `npm run build` 或等价命令，并让发布脚本/服务端托管方式能交付构建产物。当前服务端主要上传 Rust binary；若前端产物不嵌入 binary，必须同步更新发布脚本上传并验证。
6. **迁移任务要减少旧代码体量**：迁移完成一个模块后，优先删除或瘦身对应旧 `pc_*` 代码，不允许只复制一份新实现而让旧实现永久留存。
7. **多 AI 并行先看迁移状态**：开始 PC 前端任务前，先读 `docs/pc-frontend-migration.md`，确认模块归属、迁移阶段和最近提交，避免多个 AI 同时迁移同一模块。

## 新旧边界

| 场景 | 应该改哪里 |
|---|---|
| 旧 PC 页面小 bug、文字错误、兼容跳转 | `server/src/assets/pc_*` |
| PC 新页面、新复杂组件、新表单、新状态流 | `pc-frontend/` |
| 新旧入口切换、静态资源托管 | `server/src/web.rs`、`server/src/router.rs` |
| 新前端构建、上传、验证 | `scripts/publish-server.*` 和相关文档 |
| APK UI 同步到移动网页版 | `server/src/assets/web_page.html`，并遵守 `apk-web-ui-sync.instructions.md` |

## 迁移阶段

1. **准备期**：新增 `pc-frontend/`、构建命令、`/pc-next` 路由和构建产物托管方式；旧 `/pc` 不变。
2. **并行期**：新功能优先进新前端；旧 PC 只做修复和入口桥接；每迁移一个模块就在 `docs/pc-frontend-migration.md` 更新状态。
3. **切换期**：新前端覆盖核心工作流后，将 `/pc` 指向新前端，旧版改为 `/pc-legacy`。
4. **收尾期**：删除旧 `pc_*` 资产、旧 asset route 和遗留桥接，确保旧原生 PC 前端归零。

## 文件计划要求

PC 前端任务开始写代码前，必须在普通模块化文件计划 JSON 外，额外说明：

```json
{
  "pc_frontend_migration": {
    "target": "pc-frontend | legacy-pc-assets | routing | build-pipeline",
    "route_impact": ["/pc-next"],
    "legacy_assets_touched": ["server/src/assets/pc_app.js"],
    "legacy_shrink_plan": "迁移完成后删除旧任务入口渲染逻辑"
  }
}
```

## 验证要求

- 修改旧静态 PC 资产：至少运行与服务端静态资源相关的最小 Rust 检查或对应测试；必要时用浏览器打开 `/pc` 验证。
- 修改新前端工程：运行新前端的类型检查、lint/build；若还没有完整检查命令，至少运行 `npm run build`。
- 修改 `pc-frontend/`、`/pc`、`/pc-next` 或用户可见 PC 工作台 UI 时，`CodePushed` 只代表代码同步，不代表用户页面已经更新。除非用户明确要求只同步代码或暂不发布，否则必须在 push 后运行 `scripts\publish-server.ps1`（Linux/macOS：`bash scripts/publish-server.sh`），让服务器 `$DATA_DIR/pc-next-dist/` 指向本次构建，并用 `scripts\check-task-complete.ps1 -Kind Server`、`/api/server/version` 和 `/pc` 可访问性收尾。
- 截图、遮挡、错位、层级、弹窗或按图修复类 UI 问题，必须先把截图里的区域定位到真实组件/样式文件，再用本地预览、浏览器截图、DOM/坐标/层级检查之一做视觉验收；无法截图时必须在最终回复说明替代证据，不能只凭 `npm run build` 宣称用户问题已解决。
- Windows 本地启动 `/pc` 前端预览时，使用 `powershell -ExecutionPolicy Bypass -File scripts\start-pc-frontend-dev.ps1`。如必须手写后台启动命令，`Start-Process -FilePath` 必须传 `(Get-Command npm.cmd).Source` 或等价的 `npm.cmd` 绝对路径，禁止传裸 `npm`，避免 PowerShell 解析到 `npm.ps1` 后被 Windows 文件关联打开成记事本。
- 修改入口或发布链路：验证 `/pc`、`/pc-next`、静态资源路径和 `scripts/publish-server.*` 相关流程说明一致。

## 提交说明

- 迁移提交使用 `refactor(pc): 迁移 <模块> 到新前端`。
- 新能力提交使用 `feat(pc): <用户可见能力>`，并说明是否进入 `pc-frontend/`。
- 删除旧资产使用 `refactor(pc): 删除旧 <模块> 原生实现`。
- 提交说明中要写清楚旧资产是否缩减，以及 `docs/pc-frontend-migration.md` 是否已更新。
