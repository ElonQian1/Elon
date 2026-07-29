# Android 真渲染与双端样式工作流

本项目的 UI Tuner 采用四层渲染策略，优先级不可颠倒：

1. **真实 Android Renderer**：优先使用 Compose Preview / Layoutlib、Preview Host 或已运行 APK 的真帧，作为视觉权威结果。
2. **项目真实 PWA 交互草稿**：项目同时维护移动网页时，PC 直接加载真实 `/web` DOM。设计师可以切换“选择组件 / 操作页面”，临时样式直接作用于真实 PWA 组件。
3. **React 通用数字孪生**：只在项目没有 PWA、页面无法 Preview、Android 正在重连或需要零延迟草稿时使用；它可以即时编辑，但不是最终验收结果。
4. **真帧校准**：任何 PWA 或数字孪生草稿都要回到 Android 构建结果校准；清空 Runtime Patch 后仍一致，才算完成。

PC 画布中的绿色或透明几何框只允许承担点选命中，不得覆盖真实像素或伪装成组件外观。

## PWA 交互草稿发现与安全边界

一龙自身项目会自动发现 `server/src/assets/web_page.html`，并使用 `/web?ui_tuner_preview=1`。其他同时拥有移动网页的项目，可以在项目中声明：

```json
// .elon/ui-pwa-preview.json
{
  "url": "/mobile-preview?ui_tuner_preview=1"
}
```

当前只接受同源相对 URL。PWA 设计桥只在 `ui_tuner_preview=1` 时启用；普通手机访问 `/web` 不注册点选和样式消息。PC 与 PWA 的 `postMessage` 双方都校验来源、协议版本和同源关系，样式修改只作用于当前设计会话。

PWA 点选后，工作台按稳定 `data-ui-node`、DOM `id`、无障碍名称和短文本匹配 Android 源码节点。匹配失败时必须明确显示“尚未建立跨端绑定”，不得把不确定节点静默写回 Android 源码。

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

## FitRun 学习工件

FitRun 的接受与拒绝结果会写入项目级学习真源：

- `.elon/ui-standards/fit-cases.v1.json` 保存经过路径脱敏的正负案例；
- `.elon/ui-standards/fit-priors.v1.json` 保存达到晋升门槛的稳定先验；
- 同目录 `*.json.bak` 仅用于本机原子写入恢复，由 Git 精确忽略。

任务提交前应检查两份学习真源不含密钥或本机绝对路径，并把本轮变化随任务提交。不得为了让工作树变干净而删除已记录的接受/拒绝案例，也不得提交恢复备份。

## 渲染方式的产品含义

| 画布状态 | 适用场景 | 是否权威 | 能否即时编辑 |
|---|---|---:|---:|
| Android Layoutlib / Compose Preview | 可以稳定 Preview 的 Composable | 是 | 写回后重新渲染 |
| Preview Host / 真机 Runtime | 依赖 DI、导航、真实状态的页面 | 是 | 支持 Runtime Patch |
| 项目真实 PWA | APK/PWA 双端项目的低延迟交互设计 | 否 | 是，真实 DOM 即时更新 |
| React 数字孪生 | 无 Preview、断线、草稿设计 | 否 | 是，PC 本地立即更新 |

React 数字孪生不是“另写一个网页冒充 APK”。它消费统一 UI IR、节点几何、绑定和 Token，并由真实 Android 截图持续校准。对于无法从 Android 反射出的圆角、字体、阴影和 Modifier 顺序，必须由组件适配器显式声明，不能凭 XML 猜测。

## 60 秒真机预算与模拟器接管

普通视觉 UI 验收默认使用 `ui_verify_with_fallback`。真机探测从 Android 准备开始计时，预算为 60 秒；一次设备离线、被其他会话占用、锁屏/AOD/通知栏遮挡、授权或安装确认、Runtime 准备失败，或预算到期，都立即结束真机路径并申请空闲模拟器槽。后续构建、安装、启动、点击、取帧和 FitRun 必须始终携带返回的会话、设备身份和 Renderer 资源证据，不能再次按“第一个在线设备”猜测。报告至少包含：

- `REAL_DEVICE_STATUS`：`READY`、`DEFERRED_USER_CONFIRMATION`、`REQUIRED_FOLLOWUP` 或 `NOT_REQUIRED`；
- `ANDROID_RENDERER`、`rendererResourceId`、lease owner 与 `sourceSha`；
- 真机失败原因、模拟器槽和最终无 Patch/source proof。

OEM、权限弹窗、软键盘、Launcher、摄像头、蓝牙、传感器、硬件和性能专项设置 `realDeviceRequired=true`，禁止模拟器替代。其他纯视觉任务在模拟器已证明精确 package、generation、业务/集成 revision、workspace 指纹、runtimeBuildId 和零 Patch 时可以报告“视觉已验收”；真机复核作为用户设备恢复后的后续项，不阻塞提交、正式发布或统一收尾。

同一 NodeAgent 内，一台物理设备或一个 `emulator-*` 实例同时只允许一个写入链路。准备阶段排除其他 Live Runtime 和正在准备的设备，部署锁按设备而不是包名串行；同一项目绑定多个已连接 Renderer 时，未带明确 `sessionId` 的 bootstrap 调用拒绝猜测。模拟器池默认最多两个并行实例，可用 `ELON_ANDROID_EMULATOR_MAX_SLOTS` 调整；无空闲槽时明确等待，不抢占已有实例。

当前本机闭环的 lease 证据作用域是 `NODE_LOCAL_OPERATION_SESSION`：owner 包含 `pcInstallId/taskId/sessionId/projectId/sourceSha`，generation 作为 fencing token。跨 PC 真机全局互斥、持久 TTL/heartbeat、所有点击/取帧/FitRun 副作用前的云端 fencing 复核，以及预克隆独立 AVD 数据目录与 FIFO 冷启动队列仍是下一阶段强制项；在这些能力发布前，跨 PC 共用同一真机必须由现有物理设备 lease 串行，MCP 入口不得宣称已获得全局 Renderer lease。

## 发布顺序与完成状态

普通 APP UI 开发固定采用 `push → 发布 Server/PWA → 发布 APK → 有资源时补做 Renderer 验收`。Renderer 资源忙碌、无空闲模拟器、真机离线或 Runtime 准备超时只产生 `VERIFICATION_DEFERRED`，不得把已经通过源码、契约、构建和发布门禁的业务状态回退为未交付。

最终报告必须拆开：

- 业务交付状态：是否已推送、Server/PWA 是否发布、APK 是否发布、统一收尾是否完成；
- 视觉验收状态：`VISUAL_ACCEPTED` 或 `VERIFICATION_DEFERRED`，以及真实设备和 Renderer 证据。

只有用户明确要求发布前验收，或 `realDeviceRequired=true` 专项，视觉验收才是发布前置门禁。没有真帧时不得宣称视觉通过。

## 视觉验收闭环标准

一次 UI 修改只有同时满足下列条件，才能宣称“视觉验收闭环完成”：

- PC 草稿可即时预览；
- Android 权威渲染结果达到目标；
- Token/Style/Kotlin 已写回源码；
- Runtime Patch 已清空；
- 重新构建后的 APK 仍与确认画面一致；
- PWA 使用同源 Token 时没有产生非预期视觉偏差。

