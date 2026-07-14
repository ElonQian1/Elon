# Android 真渲染与双端样式工作流

本项目的 UI Tuner 采用三层渲染策略，优先级不可颠倒：

1. **真实 Android Renderer**：优先使用 Compose Preview / Layoutlib、Preview Host 或已运行 APK 的真帧，作为视觉权威结果。
2. **React 数字孪生**：仅在页面无法 Preview、Android 正在重连或需要零延迟草稿时使用；它可以即时编辑，但不是最终验收结果。
3. **真帧校准**：任何数字孪生草稿都要回到 Android 构建结果校准；清空 Runtime Patch 后仍一致，才算完成。

PC 画布中的绿色或透明几何框只允许承担点选命中，不得覆盖真实像素或伪装成组件外观。

## APK 与 PWA 共用一份 Token

在目标项目创建 `.elon/ui-style-targets.json`：

```json
{
  "version": 1,
  "tokenFile": ".elon/ui-standards/tokens.json",
  "androidValuesFile": "android/app/src/main/res/values/elon_ui_tokens.xml",
  "webCssFile": "pc-frontend/src/styles/elon-ui-tokens.css",
  "androidNamePrefix": "elon_ui",
  "webVariablePrefix": "elon-ui",
  "webSelector": ":root"
}
```

Token 文件可以使用嵌套 JSON：

```json
{
  "color": {
    "primary": "#FF4F46E5",
    "surface": "#FF101216"
  },
  "radius": {
    "button": 12
  },
  "spacing": {
    "medium": "16dp"
  },
  "typography": {
    "bodySize": "14sp"
  }
}
```

用户在 UI Tuner 确认绑定到 Token/Style JSON 的 LIVE 修改后，节点会：

1. 确定性写回 Token；
2. 自动生成 Android `values.xml`；
3. 自动生成 PWA CSS Variables；
4. 重新渲染可用的 Compose Preview；
5. 最终通过构建、安装、清空 Runtime Patch 和真帧对比确认源码效果。

生成文件带来源哈希，**不要手工修改**。Android 与 PWA 组件必须消费生成的资源或 CSS 变量，不要再复制一套硬编码数值。

## 渲染方式的产品含义

| 画布状态 | 适用场景 | 是否权威 | 能否即时编辑 |
|---|---|---:|---:|
| Android Layoutlib / Compose Preview | 可以稳定 Preview 的 Composable | 是 | 写回后重新渲染 |
| Preview Host / 真机 Runtime | 依赖 DI、导航、真实状态的页面 | 是 | 支持 Runtime Patch |
| React 数字孪生 | 无 Preview、断线、草稿设计 | 否 | 是，PC 本地立即更新 |

React 数字孪生不是“另写一个网页冒充 APK”。它消费统一 UI IR、节点几何、绑定和 Token，并由真实 Android 截图持续校准。对于无法从 Android 反射出的圆角、字体、阴影和 Modifier 顺序，必须由组件适配器显式声明，不能凭 XML 猜测。

## 完成标准

一次 UI 修改只有同时满足下列条件才完成：

- PC 草稿可即时预览；
- Android 权威渲染结果达到目标；
- Token/Style/Kotlin 已写回源码；
- Runtime Patch 已清空；
- 重新构建后的 APK 仍与确认画面一致；
- PWA 使用同源 Token 时没有产生非预期视觉偏差。

