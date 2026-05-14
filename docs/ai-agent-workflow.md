# AI 代理工作流 — 代码修改·编译·部署完整流程

> 本文档描述 AI 代理在接收到用户需求后，如何安全地修改代码、触发编译、部署，并将结果反馈给用户。
> AI 代理在执行任何代码操作前，必须先阅读本文档。

---

## 总体流程

```
用户需求
  │
  ▼
Step 1: 需求分析与分类
  │
  ▼
Step 2: 定位需要修改的代码
  │
  ▼
Step 3: 生成代码修改方案（先规划，后执行）
  │
  ▼
Step 4: 执行代码修改
  │
  ▼
Step 5: 本地验证（语法检查/lint）
  │
  ▼
Step 6: git commit 提交
  │
  ▼
Step 7: 触发编译流水线
  │
  ├── 编译成功 ──► Step 8: 部署 + 打包 APK → 发送下载链接给用户
  │
  └── 编译失败 ──► Step 9: 自动修复 or 回滚 → 反馈用户
```

---

## Step 1：需求分析与分类

AI 代理接收用户消息后，必须先判断需求类型：

| 需求类型 | 示例 | 涉及代码 |
|---|---|---|
| UI 变更 | "首页加个按钮" | Android 布局 XML / 前端 HTML |
| 业务逻辑 | "点击按钮发送消息给服务器" | Android Kotlin + Rust API |
| 服务端逻辑 | "添加一个查询天气的接口" | Rust server |
| 全栈功能 | "做一个用户登录功能" | Android + Rust + 数据库 |
| 配置/文本 | "把应用名改成'我的APP'" | Android res/strings.xml |

**分析输出格式**（内部使用）：
```json
{
  "need_type": "ui_change",
  "affected_modules": ["android"],
  "affected_files": ["android/app/src/main/res/layout/activity_main.xml"],
  "estimated_complexity": "simple",
  "user_friendly_description": "在首页添加一个红色按钮"
}
```

---

## Step 2：定位需要修改的代码

### 2.1 Android 代码定位规则

| 修改内容 | 文件位置 |
|---|---|
| UI 布局 | `android/app/src/main/res/layout/*.xml` |
| 字符串/文本 | `android/app/src/main/res/values/strings.xml` |
| 颜色/样式 | `android/app/src/main/res/values/colors.xml`, `styles.xml` |
| 页面逻辑 | `android/app/src/main/kotlin/**/MainActivity.kt` 等 |
| 网络请求 | `android/app/src/main/kotlin/**/network/` |
| 权限配置 | `android/app/src/main/AndroidManifest.xml` |

### 2.2 Rust 服务端代码定位规则

| 修改内容 | 文件位置 |
|---|---|
| 新增 API 接口 | `server/src/api/` |
| 业务逻辑 | `server/src/services/` 或 `server/src/handlers/` |
| 数据模型 | `server/src/models/` |
| 配置 | `server/src/config.rs` |

### 2.3 前端代码定位规则

| 修改内容 | 文件位置 |
|---|---|
| 页面组件 | `frontend/src/components/` |
| 页面路由 | `frontend/src/pages/` |
| API 调用 | `frontend/src/api/` |
| 样式 | `frontend/src/styles/` |

---

## Step 3：生成代码修改方案

**在修改任何文件之前**，AI 代理必须：

1. **读取原始文件内容**，理解现有结构
2. **规划完整修改方案**，包括：
   - 修改哪些文件（列表）
   - 每个文件改什么（描述）
   - 是否有依赖关系（先改哪个）
3. **评估风险**：该修改是否可能破坏已有功能

```
修改前自检清单：
  □ 是否读取了要修改的文件？
  □ 修改是否局限在用户要求的范围内？
  □ 修改后代码语法是否正确？
  □ 是否需要同步修改多个文件（如接口+调用方）？
```

---

## Step 4：执行代码修改

- 使用 `replace_string_in_file` 或 `multi_replace_string_in_file` 精确修改
- **不允许**整个文件重写，除非是新建文件
- **不允许**删除用户未明确要求删除的功能
- 保持代码缩进、风格与原文件一致

---

## Step 5：本地验证

### Rust 代码验证
```powershell
cd server
cargo check   # 只检查语法，不完整编译，速度快
```

### Android 代码验证
```powershell
cd android
./gradlew lint   # 静态检查
```

### 前端代码验证
```powershell
cd frontend
npm run lint
```

> 如果验证失败，**立即修复，不允许带错误提交**。

---

## Step 6：git commit 提交

```powershell
git add <修改的文件列表>
git commit -m "feat(用户需求): <用中文简洁描述本次修改内容>

用户ID: {user_id}
需求原文: {original_request}
修改文件: {file_list}"
```

**commit message 规范**：
- 前缀：`feat` 新功能 / `fix` 修复 / `style` 样式 / `refactor` 重构
- 主体：中文，一句话描述用户看到的变化
- 必须包含：用户ID、需求原文

---

## Step 7：触发编译流水线

### 7.1 Rust 服务端编译
```powershell
cd server
cargo build --release
# 编译产物: server/target/release/server.exe (Windows) 或 server (Linux)
```

### 7.2 Android APK 编译打包
```powershell
cd android
./gradlew assembleRelease
# 编译产物: android/app/build/outputs/apk/release/app-release-unsigned.apk
```

### 7.3 APK 签名
```powershell
# 签名密钥从环境变量注入，不要硬编码
apksigner sign --ks $env:APK_KEYSTORE --ks-pass pass:$env:APK_KEYSTORE_PASS `
  --out app-release-signed.apk `
  android/app/build/outputs/apk/release/app-release-unsigned.apk
```

### 7.4 前端构建
```powershell
cd frontend
npm run build
# 构建产物: frontend/dist/
```

---

## Step 8：部署 + 发送结果

### 8.1 部署服务端
```powershell
# 重启 Rust 服务（具体命令根据服务器配置确定）
scripts/deploy.sh server
```

### 8.2 分发 APK
```powershell
# 上传 APK 并获取下载链接
scripts/deploy.sh apk
# 脚本返回: https://download.example.com/apk/v{version}/app.apk
```

### 8.3 推送结果给用户

通过 WebSocket 发送：
```json
{
  "type": "task_complete",
  "message": "已完成！你要的功能做好了。",
  "apk_download_url": "https://download.example.com/apk/v1.2.3/app.apk",
  "changes_summary": "在首页添加了一个红色按钮，点击后显示'你好'",
  "version": "1.2.3"
}
```

---

## Step 9：编译失败处理

```
编译失败
  │
  ▼
分析错误信息
  │
  ├── 是代码逻辑错误 ──► 修复代码 → 重新提交 → 重新编译
  │                     （最多尝试 3 次）
  │
  ├── 是依赖/配置问题 ──► 修复配置 → 重试一次
  │
  └── 无法自动修复 ──► git revert 回滚到修改前的状态
                       → 告知用户: "这个需求遇到了技术问题，正在人工处理"
```

---

## 重要约束

1. **禁止**将编译失败的代码推送到主分支
2. **禁止**在 commit 中包含 APK 签名密钥、数据库密码等敏感信息
3. **必须**在每次代码修改后更新 `copilot-instructions.md` 中的"当前开发状态"
4. **每个用户任务**必须有完整的 git 提交记录，可溯源
5. **不允许**一次修改范围过大（超过5个文件应拆分为多次任务）
