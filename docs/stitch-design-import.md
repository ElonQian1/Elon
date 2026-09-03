# Stitch 设计导入合同

本合同规定如何把 Google Stitch 导出稳定地应用到一龙 Android APK 与移动 PWA。它补充 `docs/Design.md`，不改变业务行为、导航语义或发布规则。

## 可接受输入与权威顺序

完整 `.zip` 是首选输入。导入前运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\inspect-stitch-export.ps1 `
  -ZipPath <导出文件.zip> `
  -OutputPath .ai-tmp\stitch-export-inspection.json `
  -RequireFull
```

工具必须至少确认 `screen.png`；同时存在 `code.html` 与 `DESIGN.md` 时为 `FULL`，缺少任一结构化文件时为 `PARTIAL`，只能把缺失参数标成推断值。

检查器使用稳定的 `elon.stitch_export_inspection.v1` JSON 回执；`-OutputPath` 以无 BOM UTF-8 原子写入，标准输出也保留同一份 JSON。退出合同如下：

- `exit 0`：导出可用于当前模式；使用 `-RequireFull` 时只接受 `FULL`；
- `exit 2`：`INSUFFICIENT`，缺少目标截图，停止实现；
- `exit 3`：存在截图但不是 `FULL`，被 `-RequireFull` 精确导入门禁拒绝；
- `exit 1`：ZIP、重复必需文件或 PNG 结构无效。

回执中的 `claimPolicy.oneToOneClaimFromExportAlone` 永远是 `false`：`FULL` 只证明精确参数来源可用，不证明 APK/PWA 的运行时视觉已经一致。

同一属性发生冲突时按以下顺序裁决：

1. 用户对当前页面的明确要求；
2. `code.html` 中目标组件的计算后几何与样式；
3. `DESIGN.md` 中的设计 token 和组件说明；
4. `screen.png` 的像素测量；
5. 项目现有通用 token；
6. AI 推断。

截图负责确认最终视觉，不应覆盖 HTML 中明确的组件尺寸。项目旧布局只能保留业务语义和用户明确要求保留的素材，不能覆盖目标设计几何。

## 导入记录

开始实现前记录以下事实，临时记录放在 `.ai-tmp/`，不得提交下载路径：

- ZIP 的 SHA-256；
- `code.html`、`DESIGN.md`、`screen.png` 是否存在及各自 SHA-256；
- 目标图像宽高，即设计画布尺寸；
- 目标页面、目标状态和需要保留的现有交互/图标；
- 每项实现参数的来源：`EXACT_CODE`、`EXACT_DESIGN`、`MEASURED_IMAGE` 或 `INFERRED`。

`INFERRED` 参数不能被描述为“1:1”。

## 项目约束怎样参与导入

项目约束分为两类，不能混用：

- 业务、导航、状态、无障碍、触控安全区和 APK/PWA 同步约束必须保留，它们帮助导出页面成为可用产品；
- 旧页面的宽高、padding、weight、透明度、背景、圆角和视觉 token 只是兜底。与当前 Stitch 目标冲突时，必须在目标页面作用域内移除或覆盖，不能反向限制设计稿。

Stitch 导出的 HTML/CSS 可以直接提供布局关系、精确数值和响应模式，但不是 Android 源码，也不能整页覆盖带业务逻辑的移动 PWA。实施时必须建立并保存参数映射：

| 目标组件/属性 | 证据来源 | Stitch 值 | Android 实现 | PWA 实现 | 状态 |
|---|---|---:|---|---|---|
| 示例：底部导航总宽 | `code.html` | `337px` | 固定内在宽度，窄屏统一缩放 | `width: 337px` | `EXACT_CODE` |

状态只能是 `EXACT_CODE`、`EXACT_DESIGN`、`MEASURED_IMAGE` 或 `INFERRED`。关键参数仍为 `INFERRED` 时，应继续补证据或明确交付为近似实现。

## 画布、单位与响应式换算

Stitch 的 CSS 像素不能无条件当作 Android `dp`，也不能无条件按截图宽度缩放。先从导出 HTML 判定组件的响应模式：

- `INTRINSIC_FIXED`：固定按钮、padding 与 gap 形成的内在尺寸；在可用宽度足够时保持该尺寸；
- `FLUID`：使用百分比、两侧约束、`flex: 1` 或 `max-width`；按 Android 父约束重新布局；
- `CANVAS_SCALED`：明确要求整组保持比例的插画、预览或特殊组合；才允许统一缩放。

只有 `CANVAS_SCALED`，或 `INTRINSIC_FIXED` 在安全宽度中放不下时，才使用：

```text
scale = min(1, W_available / W_component)
rendered_component_width = W_component * scale
```

`W_available` 必须扣除导出代码要求的左右安全留白。对确实需要缩放的固定内部比例组合组件，外框、内边距、间隙、按钮、分隔线、圆形操作按钮和图标必须使用同一个 `scale`；禁止只缩外框。文本字号是否缩放由导出代码决定，不能从容器缩放自动推导。

例如导出的底部导航由固定 `56px/48px` 按钮、padding、gap 和 divider 组成，且在目标 `390px` 视口内可以完整容纳，应保持其约 `337px` 内在宽度；不能再除以目标画布宽度进行二次缩放。

Android 还必须区分：

- CSS `px`：设计画布坐标；
- `dp`：Android 密度无关布局单位；
- `sp`：受系统字体缩放影响的文字单位；
- 系统栏与安全区域：不属于 Stitch 内容画布，必须单独处理。

若目标图包含设备外框、编辑器画布或缩放预览，先裁出真实页面边界，再计算画布尺寸。

## 组件替换规则

按图还原时先建立“目标组件 → Android View → PWA 元素”映射，再修改源码。

- 目标设计改变组件几何时，移除旧的 `match_parent`、weight、最小宽高、父容器 padding、透明叠层和重复背景等冲突约束。
- 用户要求保留图标时，只保留图标语义与素材；图标容器、选中底板、间距和缩放仍服从目标设计。
- 导出的位图或矢量素材优先复用；重新绘制时必须记录原因并做视觉比对。
- `backdrop-filter`、外发光等网页效果在 Android 不可直接等价时，使用视觉等价实现，并分别记录源参数与平台实现方式。
- 点击区域可为无障碍扩大，但不得改变可见几何；视觉尺寸与触控尺寸应分离。

## 实现与验收门禁

完成条件不是“代码中写入了相同数字”，而是以下证据同时成立：

1. Android 与移动 PWA 使用同一套已记录参数和缩放规则；
2. 契约测试覆盖画布宽度、关键组件尺寸、响应式比例和必须保留的素材；
3. 构建通过；
4. 使用与目标图相同的视口、页面状态、字体缩放和内容夹具捕获运行时截图；
5. FitRun 或等价像素比较通过项目阈值；未取得真帧时只能报告“已实现/已发布，视觉验证延期”；
6. 用户报告刚交付结果不正确时，设置 `realDeviceRequired=true`，但仍遵守发布与验证分离规则。

以下情况必须阻止“1:1 完成”的结论：导出不完整、画布边界未知、字体或素材缺失、目标状态无法复现、运行时截图缺失，或关键参数仍为 `INFERRED`。

自动化由 `scripts/test-stitch-design-import-workflow.ps1` 覆盖，并接入本地静态质量入口与 CI。回归至少验证 `FULL`、`PARTIAL`、`INSUFFICIENT`、损坏 PNG、重复必需文件、无 BOM 回执和项目路由；修改检查器或本合同时必须同步更新该测试。

## 与现有项目规范的关系

- `docs/Design.md` 管视觉语言和组件原则；
- 本合同管 Stitch 证据如何变成跨端实现参数；
- `.github/instructions/apk-web-ui-sync.instructions.md` 管 APK 与 PWA 同步范围；
- `docs/app-ui-fast-lane.md` 管验证、推送和发布顺序；
- `.agents/skills/yilong-ui-design/SKILL.md` 管运行时捕获、FitRun 与视觉证据。
