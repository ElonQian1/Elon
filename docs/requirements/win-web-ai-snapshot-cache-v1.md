---
version_status: current
requirement_status: accepted
reviewed_at: 2026-08-14
---

# Win 网页 AI 快照缓存 V1

## 目标

Win 客户端在 ChatGPT 与 Google AI 模式之间切换、刷新一龙 PC 页面或重启客户端时，
先回显当前一龙账号和厂商对应的最近一次安全语义快照，再在后台连接官方 WebView2 页面并更新。
会话列表、项目列表和当前聊天不能因为官方页面重新加载而先变成空白。

## 用户主路径

1. 用户在一龙 Chat 模式选择 ChatGPT 或 Google AI。
2. 原生 UI 同步读取按“一龙账号 + 厂商”隔离的最近快照并立即呈现。
3. 后台 WebView2 继续加载官方页面；缓存内容保持可读，但发送等写操作仍以实时页面能力为准。
4. 适配器收到新的可见语义后原子替换缓存，原生 UI 自动更新。
5. ChatGPT 侧栏每次重新激活都后台同步一次官网会话与项目目录，不能因为已有缓存而跳过刷新。

## 必须实现

- 前端提供有界进程内热缓存，厂商切换的首帧不得先清空后等待 IPC。
- Rust 宿主继续保留进程内快照，并在 Windows 使用当前用户 DPAPI 加密持久化最后一份完整快照。
- 持久快照按不可逆 owner 指纹和 provider 隔离，缓存键不得使用原始一龙 ownerKey。
- 状态合同必须区分 `empty`、`cached` 和 `live`，并提供快照更新时间；缓存不能冒充实时官网状态。
- 页面导航或重启恢复期间使用 stale-while-revalidate；新适配器事件到达后切换为 `live`。
- 流式生成中的半成品、输入框草稿、命令结果、Cookie、token、请求头和原始响应不得写入持久缓存。
- “清除本地网页会话”必须同时清除对应 WebView2 浏览数据、内存快照和 DPAPI 快照文件。
- 缓存损坏、版本未知、解密失败或体积超限必须忽略并继续官方页回退，不能阻止会话创建。

## 非目标

- 不缓存或重放厂商私有 API，不复制官方 Cookie 到一龙前端。
- 不让缓存内容解锁发送、停止、登录、模型选择等实时动作。
- 不修改服务器、APK 或 PWA；PWA 不获得本机 WebView2 快照。
- 不在本轮真实读取、发送或截图用户私密会话。

## 验收标准

1. 前端定向测试覆盖账号/厂商隔离、容量淘汰、缓存命中和清除。
2. 控制器合同证明厂商变化时先读取热缓存，随后才后台打开和轮询官方会话。
3. ChatGPT 侧栏合同证明缓存目录立即可见，同时每次厂商重新激活仍触发一次目录同步。
4. Rust 测试覆盖缓存来源转换、流式/草稿持久化过滤、损坏缓存降级和清除。
5. PC TypeScript、ESLint、Vite 生产构建与 Tauri Rust 定向测试通过。
6. Windows 发布工件绑定唯一 Git SHA；真实账号下的切换时延和官网 DOM 同步单独保留为现场验收。

## 实现范围

- `pc-frontend/src/features/user-browser/`
- `pc-frontend/scripts/test-local-ai-snapshot-cache.cjs`
- `desktop-shell/src-tauri/src/local_ai_browser*`
- `desktop-shell/src-tauri/Cargo.toml`
- `docs/user-browser-module-integration.md`
