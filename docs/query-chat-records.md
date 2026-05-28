# 查询聊天记录与项目状态 — AI 代理操作手册

> **适用对象**：Copilot CLI、Codex CLI、VS Code Copilot Agent  
> **用途**：诊断用户投诉"为什么这么慢"、"中间说了什么"、"APK地址在哪"等问题时，快速定位数据库和日志中的真实聊天记录。  
> **加载时机**：用户要求"查看聊天记录"、"分析对话时间线"、"找到某个会话的历史"时按需加载本文档。

---

## 1. 数据存储位置速查

### 1.1 服务端数据库（最重要）

```
服务器：root@43.139.149.158（需加 -o ProxyCommand=none 绕代理）
数据库：/opt/elon/data/elon.db  ← 所有聊天消息都在这里
日志：  /root/elon-server.log   ← 只有系统事件，没有消息文本
```

> ⚠️ 注意：`/root/workspaces/elon.db` 是空文件，是旧路径遗留，**不要用它**。

### 1.2 Android 设备侧日志（补充参考）

```
ADB 设备 ID：e0d909c3（有线）或 192.168.31.171:5555（无线）
ADB 路径：D:\Android\sdk\platform-tools\adb.exe
App 包名：com.elon.app
Logcat 命令：
  D:\Android\sdk\platform-tools\adb.exe -s e0d909c3 logcat -d com.elon.app:V *:S
```

Android 端日志只包含 UI 渲染和网络状态，**不含服务器消息文本**，聊天内容必须查数据库。

---

## 2. 数据库核心表结构

### `projects` — 项目

| 字段 | 说明 |
|---|---|
| `id` | 格式：`4390d6d0-xxxx`（UUID）或 `prj_xxxx`（新格式） |
| `name` | 用户起的名字，如"杀蟑螂"。**同名项目可能有多个！** |
| `created_at` | UTC 时间，需+8小时换算为北京时间 |
| `workspace_key` | 服务器工作区目录名 |

### `conversations` — 会话

| 字段 | 说明 |
|---|---|
| `id` | UUID 或 `"default"`（首条会话固定为 default） |
| `project_id` | 外键 → projects.id |
| `title` | 会话标题，如"新会话 2" |

### `messages` — 消息（最核心）

| 字段 | 说明 |
|---|---|
| `id` | 格式：`msg_xxxx` |
| `project_id` | 所属项目 |
| `conversation_id` | 所属会话，可为 `"default"` |
| `role` | `"user"` 或 `"assistant"` |
| `content` | 完整消息文本 |
| `created_at` | UTC 时间戳（ISO 8601） |

### `tasks` — 任务执行记录

| 字段 | 说明 |
|---|---|
| `id` | 格式：`tsk_xxxx` |
| `project_id` | 所属项目 |
| `conversation_id` | 所属会话 |
| `message` | 用户发送的原始消息 |
| `status` | `"done"` / `"failed"` / `"running"` |
| `apk_url` | 生成的 APK 公网下载地址（如有） |
| `error` | 错误信息（如有） |

---

## 3. SSH 连接与查询方法

### 3.1 Windows 下正确的 SSH 命令格式

```powershell
# Windows PowerShell 下，用单引号包裹整个远程命令
& "C:\Windows\System32\OpenSSH\ssh.exe" -o ProxyCommand=none root@43.139.149.158 'echo "SQL语句" | sqlite3 /opt/elon/data/elon.db'
```

> **坑记录**：
> - PowerShell 里不能用 `''` 在双引号内转义单引号，应用 `echo "..." | sqlite3` 管道方式
> - SQL 中字符串用 `''` 两个单引号转义，如 `WHERE id=''38745c35-...''`
> - `sqlite3 /opt/elon/data/elon.db "SQL"` 方式在 PowerShell 里引号容易出错，用 `echo` 管道更可靠

### 3.2 Linux / Codex CLI 服务端

```bash
sqlite3 /opt/elon/data/elon.db "SQL语句"
# 或交互模式
sqlite3 /opt/elon/data/elon.db
```

---

## 4. 常用查询模板

### 4.1 按项目名查所有同名项目

```sql
SELECT id, name, created_at FROM projects WHERE name LIKE '%杀蟑螂%' ORDER BY created_at ASC;
```

> ⚠️ **同名项目陷阱**：用户可能创建了多个同名项目。必须列出所有，再按 `created_at` 和消息内容判断哪个是用户所指。

### 4.2 按项目ID查所有会话

```sql
SELECT id, title, created_at FROM conversations
WHERE project_id = 'prj_5530f285db7646b9bb18ab3bcbab1cae'
ORDER BY created_at ASC;
```

### 4.3 查某项目全部消息（时间线）

```sql
SELECT role, created_at, conversation_id, substr(content, 1, 200)
FROM messages
WHERE project_id = 'prj_5530f285db7646b9bb18ab3bcbab1cae'
ORDER BY created_at ASC;
```

### 4.4 按日期过滤（北京时间换算）

```sql
-- 北京时间 5月27日晚 = UTC 5月27日 15:00 之后（UTC+8，晚23点=UTC15点）
SELECT role, created_at, substr(content, 1, 150)
FROM messages
WHERE project_id = 'prj_5530f285...'
  AND created_at >= '2026-05-27T15:00'
ORDER BY created_at ASC;
```

### 4.5 查某会话的任务执行结果（含 APK URL）

```sql
SELECT id, status, apk_url, substr(message, 1, 80), created_at
FROM tasks
WHERE project_id = 'prj_5530f285db7646b9bb18ab3bcbab1cae'
  AND created_at >= '2026-05-27T15:00'
ORDER BY created_at ASC;
```

### 4.6 按消息内容搜索（关键词定位）

```sql
SELECT role, created_at, conversation_id, substr(content, 1, 200)
FROM messages
WHERE project_id = 'prj_5530f285...'
  AND content LIKE '%下载地址%'
ORDER BY created_at ASC;
```

### 4.7 统计各会话消息数量和时间范围

```sql
SELECT conversation_id, COUNT(*) as cnt, MIN(created_at) as first, MAX(created_at) as last
FROM messages
WHERE project_id = 'prj_5530f285...'
GROUP BY conversation_id
ORDER BY first ASC;
```

---

## 5. 日志与数据库结合分析

### 5.1 服务器日志能告诉你的

```bash
# 按项目ID搜索某天的日志
grep -n "prj_5530f285\|4390d6d0" /root/elon-server.log | grep "2026-05-27"
```

日志包含：
- 工作区创建时间（`[工具] 创建项目工作区`）
- Codex CLI 预热结果（成功/失败/耗时）
- 意图路由决策（`intent routing decision`）
- AI 请求耗时（`local AI CLI request completed ... elapsed_ms=XXX`）
- 服务器重启时间（`elon server listening on`）
- 健康探针结果（网络中断/恢复）

日志**不包含**：用户消息文本、AI 回复内容。

### 5.2 时间换算

| 日志时间（UTC） | 北京时间（CST = UTC+8） |
|---|---|
| `T06:00` | 14:00 |
| `T11:00` | 19:00 |
| `T15:00` | 23:00 |

---

## 6. 诊断工作流（AI 代理标准步骤）

当用户说"为什么没有回复"、"中间说了什么"、"聊天记录在哪"时，按以下顺序操作：

```
Step 1. 按项目名找项目ID（注意同名多项目）
  ↓
Step 2. 查该项目的所有会话（conversations）
  ↓
Step 3. 查该项目的全部消息（messages），按时间排序
  ↓
Step 4. 查该项目的任务列表（tasks），确认 apk_url 和 status
  ↓
Step 5. 对照服务器日志，找重启/网络中断/耗时事件
  ↓
Step 6. 用数据库时间换算北京时间，呈现完整时间线
```

---

## 7. 典型陷阱与注意事项

### 7.1 同名项目问题
用户可能在不同时间创建了多个同名项目。**必须先列出所有同名项目及其创建时间**，再询问或根据消息内容判断用户所指的是哪一个。

### 7.2 "本轮开发任务已完成"的来源
这句话**不是服务器发送的**，是 Android 端本地生成的兜底文案：
- 触发条件：服务器返回的 `done` 消息中 `apk_url` 不为空，且消息内容被 `cleanFinalReplyForUser()` 过滤后为空白
- 过滤规则：包含 `/root/workspaces/` 路径、`apksigner`、`sha256sum` 等行会被过滤掉
- 相关代码：`android/app/src/main/kotlin/com/elon/app/MainWorkflowText.kt` → `finalReplyMessage()`

### 7.3 `conversation_id` 陷阱
- 每次用户在不同"聊天气泡"里开新会话，都会产生新的 `conversation_id`
- 日志里 `prewarm` 事件的 `conversation_id` 对应数据库里的会话 ID
- 如果 `prewarm` 失败（超时），该轮消息可能不会写入数据库

### 7.4 APK 下载链接格式
任务里的 `apk_url` 是带鉴权的公网地址：
```
http://43.139.149.158:8080/api/user/{user_id}/projects/{project_id}/download/latest.apk
```
AI 回复文本里的 `/root/workspaces/...` 是服务器本地路径，**手机无法直接访问**。

---

## 8. 快速参考命令集

```powershell
# Windows SSH 连接（绕代理）
$SSH = "C:\Windows\System32\OpenSSH\ssh.exe"

# 查所有同名项目
& $SSH -o ProxyCommand=none root@43.139.149.158 'echo "SELECT id, name, created_at FROM projects WHERE name LIKE ''%蟑螂%'' ORDER BY created_at;" | sqlite3 /opt/elon/data/elon.db'

# 查项目全部消息
& $SSH -o ProxyCommand=none root@43.139.149.158 'echo "SELECT role, created_at, conversation_id, substr(content,1,200) FROM messages WHERE project_id=''PROJECT_ID'' ORDER BY created_at ASC;" | sqlite3 /opt/elon/data/elon.db'

# 查任务列表（含 APK URL）
& $SSH -o ProxyCommand=none root@43.139.149.158 'echo "SELECT id, status, apk_url, substr(message,1,60), created_at FROM tasks WHERE project_id=''PROJECT_ID'' ORDER BY created_at ASC;" | sqlite3 /opt/elon/data/elon.db'

# 查当天服务器日志（按项目ID）
& $SSH -o ProxyCommand=none root@43.139.149.158 'grep "PROJECT_ID" /root/elon-server.log | grep "2026-05-27"'

# 查服务器重启时间
& $SSH -o ProxyCommand=none root@43.139.149.158 'grep "elon server listening on" /root/elon-server.log'

# 查 Codex 网络中断事件
& $SSH -o ProxyCommand=none root@43.139.149.158 'grep "health probe failed" /root/elon-server.log | grep "2026-05-27"'
```

```bash
# Linux / Codex CLI 服务端直接执行
sqlite3 /opt/elon/data/elon.db "SELECT id, name FROM projects WHERE name LIKE '%蟑螂%';"
grep "PROJECT_ID" /root/elon-server.log | tail -50
```
