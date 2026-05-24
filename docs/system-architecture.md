# 系统架构详细设计

> 本文档供 AI 代理按需读取，描述云端APK开发平台的完整系统架构。

---

## 1. 整体架构图

```
┌─────────────────────────────────────────────────────────────┐
│                        用户手机                               │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  一龙 APK                                              │  │
│  │  - AI 对话界面（自然语言输入）                          │  │
│  │  - 任务进度展示（编译中/部署中/完成）                   │  │
│  │  - APK 下载/更新入口                                   │  │
│  │  - 后端服务器版本展示                                  │  │
│  └────────────────┬──────────────────────────────────────┘  │
└───────────────────┼─────────────────────────────────────────┘
                    │ HTTPS / WebSocket
                    ▼
┌─────────────────────────────────────────────────────────────┐
│                      服务器                                   │
│                                                             │
│  ┌─────────────────┐    ┌──────────────────────────────┐   │
│  │  Rust API 服务   │    │     AI 对话处理模块           │   │
│  │  - 接收用户请求  │───►│  - 调用 LLM 理解需求          │   │
│  │  - 任务队列管理  │    │  - 生成代码修改方案            │   │
│  │  - 状态推送      │    │  - 规划执行步骤               │   │
│  └────────┬────────┘    └──────────────┬───────────────┘   │
│           │                            │                    │
│           ▼                            ▼                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                代码修改执行层                         │   │
│  │                                                     │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │   │
│  │  │ Rust代码  │  │ Android  │  │   前端代码        │  │   │
│  │  │ 修改模块  │  │ 代码修改  │  │   修改模块        │  │   │
│  │  └──────────┘  └──────────┘  └──────────────────┘  │   │
│  └─────────────────────────┬───────────────────────────┘   │
│                            │                               │
│                            ▼                               │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                 自动化流水线                          │   │
│  │                                                     │   │
│  │  git commit → 本地开发机编译 → 上传产物到服务器     │   │
│  │       → 重启后端 / 分发APK → 生成下载链接            │   │
│  └─────────────────────────┬───────────────────────────┘   │
│                            │                               │
└────────────────────────────┼───────────────────────────────┘
                             │
                             ▼
                    ┌────────────────┐
                    │  APK 下载链接   │
                    │  → 推送回用户  │
                    └────────────────┘
```

---

## 2. 数据流详解

### 2.1 用户发起需求

```
用户输入: "帮我在首页加一个红色按钮，点击后显示'你好'"
    │
    ▼
APK 客户端 → POST /api/v1/conversation
    {
      "user_id": "xxx",
      "session_id": "yyy",
      "message": "帮我在首页加一个红色按钮，点击后显示'你好'"
    }
```

### 2.2 AI 理解需求，规划修改

```
AI 分析结果:
    {
      "type": "android_ui_change",
      "files_to_modify": ["android/app/src/main/res/layout/activity_main.xml"],
      "description": "在 activity_main.xml 中添加红色按钮",
      "git_message": "feat: 用户xxx请求 - 首页添加红色按钮"
    }
```

### 2.3 执行修改 → 编译 → 部署

```
Step 1: 修改 Android 布局文件
Step 2: git add . && git commit -m "feat: 用户xxx请求 - 首页添加红色按钮"
Step 3: 在本地开发机触发 Android 编译 (./gradlew assembleRelease)
Step 4: 本地签名 APK (apksigner / Gradle signingConfig)
Step 5: 上传 APK 产物到分发服务器
Step 6: 生成唯一下载链接
Step 7: 通过 WebSocket 推送给用户
```

---

## 3. 代码仓库结构（目标结构）

```
d:\一龙\
├── .github\
│   └── copilot-instructions.md   ← AI全局指令（永远加载）
├── docs\
│   ├── system-architecture.md    ← 本文件
│   └── ai-agent-workflow.md      ← AI工作流详细步骤
├── server\                        ← Rust 服务端
│   ├── src\
│   │   ├── main.rs
│   │   ├── api\                   ← HTTP API 路由
│   │   ├── conversation\          ← AI 对话处理
│   │   ├── pipeline\              ← 编译部署流水线
│   │   └── models\                ← 数据模型
│   └── Cargo.toml
├── android\                       ← Android APK
│   ├── app\src\main\
│   │   ├── kotlin\                ← Kotlin 源码
│   │   ├── res\                   ← 布局/资源
│   │   └── AndroidManifest.xml
│   └── build.gradle
├── frontend\                      ← Web 前端（如有）
│   ├── src\
│   └── package.json
└── scripts\                       ← 自动化脚本
    ├── publish-server.ps1         ← Windows 本地交叉编译后端并上传 binary
    ├── publish-server.sh          ← Linux/macOS 本地交叉编译后端并上传 binary
    └── publish-apk.ps1            ← 本地构建、签名并上传 APK
```

---

## 4. 多用户隔离方案

每个用户的修改需要隔离，防止冲突：

| 方案 | 说明 | 推荐场景 |
|---|---|---|
| **Git 分支隔离** | 每个用户在独立分支修改，合并到主分支前测试 | 生产推荐 |
| **代码沙箱** | 每个用户有独立的代码副本 | 高并发场景 |
| **任务队列** | 串行处理，同一时刻只有一个修改在执行 | 简单起步阶段 |

> 初期推荐：**任务队列** + **Git 分支**组合

---

## 5. APK 分发方案

当前一龙 APK 分发不是第三方托管方案，而是固定直链 + 版本信息 + 同 WiFi P2P mirror：

```
scripts/publish-apk.ps1
    │
    ├── 上传 /opt/elon/data/app/ElonSpeed-latest.apk
    ├── 上传 /opt/elon/data/app/version.json
    ├── POST /api/app/update/broadcast 通知在线客户端
    │
    ▼
服务器
    ├── GET /app/version.json
    │     ├── 读取磁盘 version.json
    │     ├── 重写 downloadUrl/downloadPageUrl 为公网地址
    │     └── 动态注入在线 seeder mirrors
    ├── GET /app/ElonSpeed-latest.apk
    ├── GET /app/peer/ws?version_code=N
    └── GET /app/relay/peer/{peer_id}/apk
```

相关实现：

| 模块 | 职责 |
|---|---|
| `server/src/app_update.rs` | 读取最新 `version.json` 并广播在线更新事件 |
| `server/src/peer_relay.rs` | 注册同 WiFi seeder、动态注入 `mirrors`、中继 APK 下载 |
| `android/app/src/main/kotlin/com/elon/app/update/AppUpdateManager.kt` | 拉取 `version.json`、尝试 mirrors、失败后回退服务器直链 |
| `android/app/src/main/kotlin/com/elon/app/update/PeerSeederManager.kt` | 已安装 APK 的手机连接 WebSocket，收到 `SEND_APK` 后发送本机安装包 |

P2P 分发维护规则：

- `version.json` 是 APK 分发事实来源；发布后必须校验公网 `/app/version.json`，不能只看本地生成文件。
- 当前 mirror 字段由 `server/src/peer_relay.rs` 动态注入，仅包含 `version_code >= 当前发布 versionCode` 的在线 seeder。
- 当前 Android 端按 `priority` 降序尝试 mirror；如果后续引入 dev-mirror 并希望采用“数字越小越优先”，必须同步修改 Android 排序、服务器注入规则和文档，然后按 APK 发布闭环发布。
- WebSocket 长连接必须使用无读超时或足够长读超时；服务器端遇到 Ping/Pong 等控制帧不能当作传输失败。
- 大 APK 中继要关注背压和内存占用。当前服务器会收集完整 APK 后再返回 HTTP 响应；若 APK 增大或并发增加，应改为流式转发，并在 Android WebSocket 发送端按队列大小节流，避免 OkHttp 写缓冲撑满导致截断。
- mirror 全部失败时必须保留 `downloadUrl` 直链兜底，避免 P2P 节点不在线影响普通更新。

历史备选方案：

```
编译完成的 APK
    │
    ├── 方案A: 自建文件服务器
    │         APK 存到 /var/www/apk/{version}/app.apk
    │         生成链接: https://download.example.com/apk/v1.2.3/app.apk
    │
    ├── 方案B: 对象存储 (OSS/S3)
    │         上传到 OSS，生成预签名 URL（有时效性）
    │
    └── 方案C: pgyer / 蒲公英 等第三方分发
              调用 API 上传，返回下载页面链接
```

---

## 5.1 版本信息通道

一龙区分 APK 版本和后端服务器版本：

| 类型 | 来源 | 接口 | 用途 |
|---|---|---|---|
| APK 版本 | `android/app/build.gradle` + 发布脚本生成的 `version.json` | `/app/version.json` | Android 自更新、下载页、P2P APK mirrors |
| 后端版本 | `server/Cargo.toml` 的 `package.version` + 构建时注入的 git SHA | `/api/server/version` | APK 个人页展示服务器版本，用户可见后端已更新 |

后端运行代码变更必须递增 `server/Cargo.toml` 版本号并走服务端部署脚本；部署脚本负责注入 git SHA，重启后验证 `/health` 和 `/api/server/version`。

---

## 6. 安全考虑

- **代码执行沙箱**：AI 生成的代码变更必须经过人工确认规则或自动安全扫描才能执行
- **APK 签名密钥**：存储在服务器安全存储，不随代码提交，只在 CI 步骤中注入
- **用户鉴权**：所有 API 需要用户 Token，对话内容不混用
- **代码审计**：保留所有 git commit 历史，可溯源每个修改的用户和时间
- **速率限制**：限制每个用户每天触发编译的次数，防止资源滥用
