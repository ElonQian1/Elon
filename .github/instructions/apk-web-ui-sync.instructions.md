---
applyTo: >
  android/app/src/main/res/layout/**,
  android/app/src/main/java/**Activity.kt,
  android/app/src/main/java/**Fragment.kt,
  android/app/src/main/java/**View.kt,
  android/app/src/main/res/values/colors.xml,
  android/app/src/main/res/values/themes.xml,
  server/src/assets/web_page.html
---

# APK UI ↔ 网页 UI 同步规则

Ripple、颜色、间距、圆角、字号和轻量动画等低风险纯视觉同步，优先按 `docs/app-ui-fast-lane.md` 的单命令验证与发布流程执行；默认不启动真机、模拟器或 Renderer。

## 核心原则

**每次修改 APK UI，必须同步更新 `server/src/assets/web_page.html`**。
网页版是 APK 在电脑/浏览器上的镜像，两者共享同一套 API 和 WebSocket，视觉风格必须保持一致。

AI 代理修改任一侧 UI 时，必须在同一 commit 中完成两侧的同步修改。

---

## APK 文件 → 网页对照表

### 布局文件

| APK 布局文件 | 网页 HTML 区域 | 网页 CSS 区域 |
|---|---|---|
| `activity_main.xml` - Toolbar | `<header class="toolbar">` 内的 `h1` 和 `.icon-btn` | `.toolbar`, `.icon-btn` |
| `activity_main.xml` - BottomNavigationView | `<nav class="tabs-bar">` 内的 `.tab` 按钮 | `.tabs-bar`, `.tab` |
| `activity_main.xml` - 内容区 | `<section class="content">` 内各 `.page` | `.content`, `.page` |
| `item_message_user.xml` | `.bubble.user` 气泡 | `.bubble.user` |
| `item_message_ai.xml` | `.bubble.ai` 气泡 | `.bubble.ai` |
| `item_message_progress.xml` | `.bubble.progress` 气泡 | `.bubble.progress` |
| `item_message_error.xml` | `.bubble.error` 气泡 | `.bubble.error` |
| `activity_settings.xml` - 行 | `.profile-row` 按钮行 | `.profile-row` |
| `page_agent.xml` | 项目页 `.project-page` | `.project-page`, `.project-block` |

### 颜色/主题

| APK 来源 | 网页对应 CSS 变量 |
|---|---|
| `colors.xml` 背景色 `#101010` | `--bg: #101010` |
| 主品牌绿 `#07c160` | `--brand: #07c160` |
| 用户气泡绿 `#95ec69` | `--bubble-user: #95ec69` |
| 面板背景 `#1e1e1e` | `--panel: #1e1e1e` |
| 文字色 `#d0d0d0` | `--ink: #d0d0d0` |

---

## 常见改动的同步规则

### 1. 新增 Toolbar 按钮

APK `activity_main.xml` 中加了 `ImageButton`：
```xml
<ImageButton android:id="@+id/newBtn" ... android:contentDescription="功能名" />
```

网页必须在 `<header class="toolbar">` 中加同位置的 `.icon-btn`：
```html
<button class="icon-btn" id="newBtn" title="功能名">
  <svg ...><!-- 对应图标的 SVG --></svg>
</button>
```
并在 `switchTab()` 函数里控制该按钮的显隐逻辑（与 APK 逻辑对齐）。

### 2. 新增底部 Tab

APK 新增 Tab 项：
- 网页 `<nav class="tabs-bar">` 加对应 `<button class="tab" data-tab="xxxPage" data-title="标题">` 
- `<section class="content">` 加对应 `<div id="xxxPage" class="page">` 结构
- JS 的 `switchTab()` 里加对应逻辑

### 3. 消息气泡样式变化

APK `item_message_*.xml` 调整了颜色、内边距、圆角：
- 对应修改 `web_page.html` 中 `.bubble.*` 的 CSS
- 圆角/内边距值在两侧尽量用相同数值（注意 dp vs px 差异，1dp ≈ 1px）

### 4. 新增设置项/功能行

APK `activity_settings.xml` 新增一行：
- 网页"我的"页中增加对应 `<button class="profile-row" id="xxxRow">` HTML
- JS 里加对应点击事件处理

### 5. 新增对话框/弹窗

APK 新增了 AlertDialog 或 BottomSheetDialog：
- 网页加对应 `<div class="modal-mask" id="xxxMask"><form class="modal">` 结构
- JS 里加 open/close 逻辑

### 6. 颜色主题变更

APK 修改 `colors.xml` 或 `themes.xml`：
- 同步修改 `web_page.html` `<style>` 头部的 `:root { }` CSS 变量

### 7. 新增底部 UI 元素（底部栏、输入栏、按钮行等）

**所有位于底部的 UI 元素**（Tab 栏、输入栏、浮动按钮行等），网页端必须加 iOS 安全区域 padding：

```css
/* Tab 栏 / 底部导航 */
.tabs-bar {
  padding-bottom: env(safe-area-inset-bottom);
}

/* 输入栏（兼顾原有内边距和安全区域，取较大值）*/
.input-bar {
  padding-bottom: max(8px, env(safe-area-inset-bottom));
}
```

原因：iOS 全面屏手机在 PWA / Safari 全屏模式下，Home 条会遮挡页面底部。
不加此规则会导致底部按钮在 iPhone 上被系统 Home 条遮住。

---

## ❌ 以下 APK 功能无需在网页实现

AI 代理不要尝试在网页端"模拟"以下原生能力，标注说明即可：

- **语音输入/输出** (ASR/TTS) — 网页可显示"语音功能需使用 APK"提示
- **悬浮球 / 浮层窗口** — 原生 Android 特权，网页不适用
- **无障碍服务 / 屏幕捕获** — 网页不适用
- **后台 AgentService** — 网页通过 WebSocket 实现，无需背景服务
- **相机 / 麦克风原生流** — 浏览器 WebRTC 另行设计，不强制同步
- **生物识别 / 系统锁屏** — 网页用密码登录即可

---

## 检查清单（提交前必看）

改动 APK UI 后，提交前检查：

- [ ] `activity_main.xml` Toolbar 改动 → 网页 `<header class="toolbar">` 已同步
- [ ] 底部 Tab 改动 → 网页 `<nav class="tabs-bar">` + `<section class="content">` 已同步
- [ ] 消息气泡样式改动 → 网页 `.bubble.*` CSS 已同步
- [ ] 颜色/主题改动 → 网页 `:root` CSS 变量已同步
- [ ] 新增设置项 → 网页"我的"页已同步
- [ ] 新增对话框 → 网页 `.modal-mask` 已同步
- [ ] **新增底部 UI 元素 → 已加 `padding-bottom: env(safe-area-inset-bottom)`（iOS 安全区域）**
- [ ] 同一 commit 包含 APK 和网页两侧的修改

---

## 提交消息规范

同步改动时，commit message 明确说明两侧都修改了：

```
feat: APK+网页 同步新增XXX功能/页面/按钮
```

或单独提交时用：
```
sync(web): 跟进 APK XXX 改动，更新网页端 UI
```
