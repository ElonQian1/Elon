<#
.SYNOPSIS
    Audit public process events for one PC Codex task.

.DESCRIPTION
    Query the production SQLite database over SSH and report whether a task has
    the public process signals the PC conversation UI can display: dispatch,
    waiting heartbeat, command/tool events, file changes, usage, and final reply.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-pc-task-process-events.ps1 -TaskId tsk_xxx

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts\check-pc-task-process-events.ps1 -TaskId tsk_xxx -Expect dispatch,command,tool_result,usage,final_reply
#>
param(
    [Parameter(Mandatory = $true)]
    [string]$TaskId,

    [string[]]$Expect = @("dispatch", "command", "tool_result", "final_reply"),

    [string]$Server = "root@43.139.149.158",
    [string]$DbPath = "/opt/elon/data/elon.db"
)

$ErrorActionPreference = "Stop"

$validExpect = @(
    "dispatch",
    "heartbeat",
    "tool_call",
    "command",
    "file_change",
    "tool_result",
    "usage",
    "assistant_event",
    "final_reply",
    "error"
)
$Expect = @(
    $Expect |
        ForEach-Object { ($_ -split ",") } |
        ForEach-Object { $_.Trim() } |
        Where-Object { $_ }
)
foreach ($name in $Expect) {
    if ($validExpect -notcontains $name) {
        Write-Error "Invalid -Expect value '$name'. Valid values: $($validExpect -join ', ')"
        exit 1
    }
}

$repoRoot = git -C $PSScriptRoot rev-parse --show-toplevel
. (Join-Path $repoRoot "scripts\direct-network.ps1")
Set-ElonProjectDirectNetwork

$ssh = Join-Path $env:WINDIR "System32\OpenSSH\ssh.exe"
if (-not (Test-Path $ssh)) {
    $ssh = "ssh"
}

$python = @'
import json
import sqlite3
import sys

task_id = sys.argv[1]
db_path = sys.argv[2]

coverage_keys = [
    "dispatch",
    "heartbeat",
    "tool_call",
    "command",
    "file_change",
    "tool_result",
    "usage",
    "assistant_event",
    "final_reply",
    "error",
]
heartbeat_markers = [
    "\u6b63\u5728\u5904\u7406\u4e2d",
    "\u5df2\u7b49\u5f85",
    "AI \u8fd8\u5728",
]

def short(text, limit=180):
    text = (text or "").strip().replace("\n", " ")
    return text if len(text) <= limit else text[:limit] + "..."

def event_message(value):
    return str(value.get("message") or value.get("text") or "").strip()

conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row

task = conn.execute(
    """
    SELECT id, project_id, user_id, conversation_id, message, status, error,
           apk_url, codex_thread_id, created_at, updated_at
    FROM tasks
    WHERE id = ?
    """,
    (task_id,),
).fetchone()

if task is None:
    print(json.dumps({"ok": False, "error": f"task not found: {task_id}"}, ensure_ascii=False))
    sys.exit(0)

events = conn.execute(
    """
    SELECT rowid AS seq, created_at, event_json
    FROM task_events
    WHERE task_id = ?
    ORDER BY rowid ASC
    """,
    (task_id,),
).fetchall()

messages = conn.execute(
    """
    SELECT role, content, created_at
    FROM messages
    WHERE task_id = ?
    ORDER BY created_at ASC
    """,
    (task_id,),
).fetchall()

coverage = {key: False for key in coverage_keys}
event_type_counts = {}
tool_counts = {}
samples = []

for row in events:
    try:
        value = json.loads(row["event_json"])
    except Exception:
        continue
    event_type = str(value.get("type") or "")
    if not event_type:
        continue
    event_type_counts[event_type] = event_type_counts.get(event_type, 0) + 1
    tool = str(value.get("tool") or "")
    if tool:
        tool_counts[tool] = tool_counts.get(tool, 0) + 1

    if event_type == "pc_dispatch_started":
        coverage["dispatch"] = True
    if event_type == "runtime_status" and str(value.get("phase") or "") == "pc_dispatched":
        coverage["dispatch"] = True
    if event_type == "progress":
        message = event_message(value)
        if any(marker in message for marker in heartbeat_markers):
            coverage["heartbeat"] = True
    if event_type == "tool_call":
        coverage["tool_call"] = True
        if tool == "shell":
            coverage["command"] = True
        if tool == "file_change":
            coverage["file_change"] = True
    if event_type == "tool_result":
        coverage["tool_result"] = True
        if tool == "shell":
            coverage["command"] = True
        if tool == "file_change":
            coverage["file_change"] = True
    if event_type == "usage":
        coverage["usage"] = True
    if event_type in ("assistant_message", "assistant_chunk"):
        coverage["assistant_event"] = True
    if event_type == "done":
        coverage["final_reply"] = bool(event_message(value)) or coverage["final_reply"]
    if event_type == "error":
        coverage["error"] = True

    if event_type in ("pc_dispatch_started", "runtime_status", "progress", "tool_call", "tool_result", "usage", "assistant_message", "done", "error"):
        samples.append({
            "seq": row["seq"],
            "time": row["created_at"],
            "type": event_type,
            "tool": tool,
            "preview": short(event_message(value) or json.dumps(value, ensure_ascii=False)),
        })

for message in messages:
    if str(message["role"] or "") == "assistant" and str(message["content"] or "").strip():
        coverage["final_reply"] = True

if str(task["status"] or "") in ("failed", "error"):
    coverage["error"] = True

thread_id = (task["codex_thread_id"] or "").strip()
report = {
    "ok": True,
    "task": {key: task[key] for key in task.keys()},
    "codex_thread_uri": f"codex://threads/{thread_id}" if thread_id else "",
    "coverage": coverage,
    "event_type_counts": event_type_counts,
    "tool_counts": tool_counts,
    "message_count": len(messages),
    "event_count": len(events),
    "samples": samples[-30:],
}
print(json.dumps(report, ensure_ascii=False))
'@

$jsonText = $python | & $ssh -o ProxyCommand=none $Server "python3 - '$TaskId' '$DbPath'"
$report = $jsonText | ConvertFrom-Json

if (-not $report.ok) {
    Write-Error $report.error
    exit 1
}

$missing = @()
foreach ($name in $Expect) {
    if (-not [bool]$report.coverage.$name) {
        $missing += $name
    }
}

if ($missing.Count -eq 0) {
    Write-Host "PC_TASK_PROCESS_AUDIT=passed"
} else {
    Write-Host "PC_TASK_PROCESS_AUDIT=failed"
}

Write-Host ("TASK_ID={0}" -f $report.task.id)
Write-Host ("STATUS={0}" -f $report.task.status)
Write-Host ("PROJECT_ID={0}" -f $report.task.project_id)
Write-Host ("CONVERSATION_ID={0}" -f $report.task.conversation_id)
if ($report.codex_thread_uri) {
    Write-Host ("CODEX_THREAD_URI={0}" -f $report.codex_thread_uri)
} else {
    Write-Host "CODEX_THREAD_URI="
}
Write-Host ("EVENT_COUNT={0}" -f $report.event_count)
Write-Host ("MESSAGE_COUNT={0}" -f $report.message_count)
Write-Host ""
Write-Host "Coverage:"
foreach ($name in @("dispatch", "heartbeat", "tool_call", "command", "file_change", "tool_result", "usage", "assistant_event", "final_reply", "error")) {
    $mark = if ([bool]$report.coverage.$name) { "[x]" } else { "[ ]" }
    Write-Host ("  {0} {1}" -f $mark, $name)
}

Write-Host ""
Write-Host "Event types:"
$report.event_type_counts.PSObject.Properties |
    Sort-Object Name |
    ForEach-Object { Write-Host ("  {0}={1}" -f $_.Name, $_.Value) }

Write-Host ""
Write-Host "Tools:"
if ($report.tool_counts.PSObject.Properties.Count -eq 0) {
    Write-Host "  none"
} else {
    $report.tool_counts.PSObject.Properties |
        Sort-Object Name |
        ForEach-Object { Write-Host ("  {0}={1}" -f $_.Name, $_.Value) }
}

Write-Host ""
Write-Host "Recent public samples:"
foreach ($sample in $report.samples) {
    $tool = if ($sample.tool) { " tool=$($sample.tool)" } else { "" }
    Write-Host ("  #{0} {1}{2} {3}" -f $sample.seq, $sample.type, $tool, $sample.preview)
}

if ($missing.Count -gt 0) {
    Write-Host ""
    Write-Host ("MISSING_EXPECTED={0}" -f ($missing -join ","))
    exit 1
}
