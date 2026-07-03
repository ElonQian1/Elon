# 查询聊天记录与项目状态 — AI 代理操作手册

> **适用对象**：Copilot CLI、Codex CLI、VS Code Copilot Agent  
> **用途**：诊断用户投诉"为什么这么慢"、"中间说了什么"、"APK地址在哪"等问题时，快速定位数据库和日志中的真实聊天记录。  
> **加载时机**：用户要求"查看聊天记录"、"分析对话时间线"、"找到某个会话的历史"时按需加载本文档。

**完整对话内容 = SQLite messages + task_events + `/root/.codex/sessions/**/*.jsonl`（Codex 会话事件记录）**
**最快路径**：用 `conversation_timeline` VIEW 一条 SQL 得到合并时间线，再通过 `codex_thread_id` 找 Codex JSONL 文件核对公开执行过程、工具调用、工具结果和最终回复；不要把它描述成可展示的“内心推理”。

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
| `codex_thread_id` | ⭐ Codex/CopilotCLI 的 thread UUID，可用来定位 `/root/.codex/sessions/**/*<uuid>.jsonl` |

### `task_events` — Codex 执行步骤流水

| 字段 | 说明 |
|---|---|
| `task_id` | 关联任务 |
| `event_json` | JSON 格式的事件，`type` 字段常见值：`progress`、`done`、`error` |
| `created_at` | UTC 时间戳 |

> 注：`task_events` 每个任务最多保留最近 200 条，超出后滚动删除。完整 Codex 内部日志（含 LLM 对话）需查 JSONL 文件。

### `agent_native_sessions` — Codex/CopilotCLI 会话管理

| 字段 | 说明 |
|---|---|
| `project_id` | 所属项目 |
| `user_id` | 所属用户 |
| `conversation_id` | 所属会话 |
| `provider` | 目前固定为 `"codex"` |
| `native_session_id` | ⭐ Codex thread UUID（与 `tasks.codex_thread_id` 同一个值） |
| `status` | `"active"` / `"inactive"` |
| `updated_at` | 最后活跃时间 |

> 用途：找到某个 project+conversation 对应的 Codex thread UUID，进而定位 JSONL 文件。

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

## 5. `conversation_timeline` VIEW — 一条 SQL 查完整时间线

**这是诊断时最快的入口**，把 `messages` 和 `task_events` 合并为按时间排序的统一视图，并附带 `codex_thread_id`。

```sql
-- 查某个会话的完整时间线（含用户消息、AI回复、Codex工具调用步骤）
SELECT time, kind, role, event_type, codex_thread_id,
       substr(content, 1, 200) AS content_preview
FROM conversation_timeline
WHERE conversation_id = 'YOUR_CONVERSATION_ID'
ORDER BY time;
```

字段说明：

| 字段 | 说明 |
|---|---|
| `time` | UTC 时间戳（需+8小时换北京时间） |
| `kind` | `"message"`（用户/AI 消息）或 `"task_event"`（Codex 执行步骤） |
| `role` | `"user"` / `"assistant"`（task_event 行此字段为 NULL） |
| `event_type` | task_event 类型，如 `"progress"`、`"done"`（message 行为 NULL） |
| `codex_thread_id` | Codex thread UUID，用来找 JSONL 文件 |
| `project_name` | 项目名称（无需额外 JOIN） |
| `task_id` | 关联任务 ID |

### 5.1 按项目ID查所有会话的合并时间线

```sql
SELECT time, kind, role, event_type, conversation_id,
       codex_thread_id, substr(content, 1, 150) AS preview
FROM conversation_timeline
WHERE project_id = 'prj_5530f285db7646b9bb18ab3bcbab1cae'
ORDER BY time;
```

### 5.2 只看用户和 AI 的对话（过滤掉执行步骤）

```sql
SELECT time, role, substr(content, 1, 300) AS content
FROM conversation_timeline
WHERE conversation_id = 'YOUR_CONVERSATION_ID'
  AND kind = 'message'
ORDER BY time;
```

### 5.3 只看 Codex 工具调用步骤（诊断卡顿/超时）

```sql
SELECT time, event_type, task_id, substr(content, 1, 200) AS detail
FROM conversation_timeline
WHERE conversation_id = 'YOUR_CONVERSATION_ID'
  AND kind = 'task_event'
ORDER BY time;
```

---

## 6. 查阅 Codex 内部完整对话（JSONL 文件）

SQLite 里存的是服务端收发的消息摘要，**Codex 内部每一轮 LLM 的 input/output、工具调用、系统 prompt** 都存在服务器的 JSONL 文件里，不在数据库中。

### 6.1 获取 thread UUID

方法一：从 `conversation_timeline` 里直接读（最方便）
```sql
SELECT DISTINCT codex_thread_id, task_id
FROM conversation_timeline
WHERE conversation_id = 'YOUR_CONVERSATION_ID'
  AND codex_thread_id IS NOT NULL;
```

方法二：查 `agent_native_sessions`
```sql
SELECT native_session_id, conversation_id, updated_at
FROM agent_native_sessions
WHERE project_id = 'YOUR_PROJECT_ID'
ORDER BY updated_at DESC;
```

### 6.2 找到并读取 JSONL 文件

文件路径格式：
```
/root/.codex/sessions/YYYY/MM/DD/rollout-<日期时间>-<thread_uuid>.jsonl
```

```bash
# 服务端：用 thread UUID 找文件
find /root/.codex/sessions -name "*019e6a19-d623-7d83-af6e-3bef562500ea*"

# 读取文件（每行一个 JSON 事件）
cat /root/.codex/sessions/2026/05/27/rollout-...-<thread_uuid>.jsonl | python3 -c "
import sys, json
for line in sys.stdin:
    if not line.strip(): continue
    ev = json.loads(line)
    if ev['type'] in ('response_item', 'tool_call', 'tool_result'):
        p = ev.get('payload', {})
        role = p.get('role', ev['type'])
        for item in p.get('content', []):
            if 'text' in item:
                print(f'[{ev[\"timestamp\"]}] [{role}] {item[\"text\"][:300]}')
"
```

PowerShell 查询：
```powershell
$SSH = "C:\Windows\System32\OpenSSH\ssh.exe"
$tid = "019e6a19-d623-7d83-af6e-3bef562500ea"  # 替换为实际 thread UUID
& $SSH -o ProxyCommand=none root@43.139.149.158 "find /root/.codex/sessions -name '*$tid*'"
```

### 6.3 JSONL 关键 type 说明

| type | 说明 |
|---|---|
| `session_meta` | 会话元数据（cwd 含 project_id + conversation_id、model） |
| `event_msg` | 任务开始/结束 |
| `response_item` | ⭐ LLM 对话内容（role: developer/user/assistant） |
| `tool_call` | ⭐ 工具调用（shell 命令、文件读写） |
| `tool_result` | 工具执行结果（包含 stdout/stderr） |
| `turn_context` | 当前工作目录、时区等上下文 |

### 6.4 数据库丢失时如何恢复

如果 `/opt/elon/data/elon.db` 损坏：
1. `/root/.codex/sessions/` 保留了所有 Codex 内部记录（LLM 对话、工具调用）
2. JSONL 里 `session_meta.payload.cwd` 包含 `project_id` 和 `conversation_id`，可重建对应关系
3. `agent_native_sessions` 存有 `native_session_id`，已知的 thread UUID 仍然可以找到文件

---

## 7. 日志与数据库结合分析

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

## 8. 诊断工作流（AI 代理标准步骤）

当用户说"为什么没有回复"、"中间说了什么"、"聊天记录在哪"时，按以下顺序操作：

```
Step 1. 按项目名找项目ID（注意同名多项目）
  ↓
Step 2. 查该项目的所有会话（conversations），确认 conversation_id
  ↓
★ Step 3. 用 conversation_timeline VIEW 一条 SQL 查完整时间线
          → 包含用户消息、AI回复、Codex 执行步骤，全部按时间排列
  ↓
Step 4. 从时间线里找到 codex_thread_id（有卡顿/异常时才需要往下）
  ↓
Step 5. SSH 到服务器，find /root/.codex/sessions -name "*<thread_id>*" 找 JSONL
  ↓
Step 6. 读 JSONL 获得 Codex 内部完整 LLM 对话和工具调用
  ↓
Step 7. 对照服务器日志，找重启/网络中断/耗时事件
  ↓
Step 8. 用数据库时间换算北京时间（UTC+8），呈现完整时间线
```

> 绝大多数诊断问题在 Step 3 就能解决。只有需要看 AI 内部思考、具体工具调用失败原因时才需要 Step 5-6。

---

## 9. 典型陷阱与注意事项

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

## 10. 快速参考命令集

```powershell
# Windows SSH 连接（绕代理）
$SSH = "C:\Windows\System32\OpenSSH\ssh.exe"

# ★ 最常用：conversation_timeline 一条查完整会话（用 python3 heredoc 避免引号问题）
$q = @'
import sqlite3
c = sqlite3.connect('/opt/elon/data/elon.db')
rows = c.execute("""
    SELECT time, kind, role, event_type, codex_thread_id,
           substr(content, 1, 200) AS preview
    FROM conversation_timeline
    WHERE conversation_id = 'YOUR_CONVERSATION_ID'
    ORDER BY time
""").fetchall()
for r in rows:
    print(r)
'@
$q | & $SSH -o ProxyCommand=none root@43.139.149.158 "python3"

# 查所有同名项目
& $SSH -o ProxyCommand=none root@43.139.149.158 'echo "SELECT id, name, created_at FROM projects WHERE name LIKE ''%蟑螂%'' ORDER BY created_at;" | sqlite3 /opt/elon/data/elon.db'

# 查项目全部会话
& $SSH -o ProxyCommand=none root@43.139.149.158 'echo "SELECT id, title, created_at FROM conversations WHERE project_id=''PROJECT_ID'' ORDER BY created_at;" | sqlite3 /opt/elon/data/elon.db'

# 查任务列表（含 APK URL 和 codex_thread_id）
& $SSH -o ProxyCommand=none root@43.139.149.158 'echo "SELECT id, status, codex_thread_id, apk_url, substr(message,1,60), created_at FROM tasks WHERE project_id=''PROJECT_ID'' ORDER BY created_at ASC;" | sqlite3 /opt/elon/data/elon.db'

# 通过 conversation 找 Codex thread UUID
& $SSH -o ProxyCommand=none root@43.139.149.158 'echo "SELECT native_session_id, conversation_id, updated_at FROM agent_native_sessions WHERE project_id=''PROJECT_ID'' ORDER BY updated_at DESC;" | sqlite3 /opt/elon/data/elon.db'

# 通过 thread UUID 找 Codex JSONL 文件路径
$tid = "019e6a19-d623-7d83-af6e-3bef562500ea"  # 替换为实际值
& $SSH -o ProxyCommand=none root@43.139.149.158 "find /root/.codex/sessions -name '*$tid*'"

# 查当天服务器日志（按项目ID）
& $SSH -o ProxyCommand=none root@43.139.149.158 'grep "PROJECT_ID" /root/elon-server.log | grep "2026-05-27"'

# 查服务器重启时间
& $SSH -o ProxyCommand=none root@43.139.149.158 'grep "elon server listening on" /root/elon-server.log'

# 查 Codex 网络中断事件
& $SSH -o ProxyCommand=none root@43.139.149.158 'grep "health probe failed" /root/elon-server.log | grep "2026-05-27"'
```

```bash
# Linux / Codex CLI 服务端直接执行（最简洁）
sqlite3 /opt/elon/data/elon.db <<'EOF'
SELECT time, kind, role, event_type, codex_thread_id, substr(content,1,200)
FROM conversation_timeline
WHERE project_id = 'prj_5530f285db7646b9bb18ab3bcbab1cae'
ORDER BY time;
EOF

# 找 Codex JSONL 文件（替换 thread UUID）
find /root/.codex/sessions -name "*019e6a19*"

# 查服务日志
grep "PROJECT_ID" /root/elon-server.log | tail -50
```
