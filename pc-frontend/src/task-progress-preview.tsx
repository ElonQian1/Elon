import { useMemo, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { LivePreview, livePreviewConfigFromLocation } from './task-progress-live-preview'
import type { ChatMessage, ToolEvent } from './features/dev/types'
import { previewStartedAt as startedAt, previewTaskId as taskId, ScenarioPreview } from './task-progress-preview/ScenarioPreview'

type ScenarioId =
  | 'queued'
  | 'dispatch'
  | 'heartbeat'
  | 'recovery'
  | 'server-update'
  | 'win-update'
  | 'recovery-timeout'
  | 'thinking'
  | 'command-running'
  | 'tool-timeout'
  | 'tools'
  | 'tool-failed'
  | 'approval'
  | 'resume-required'
  | 'timeout'
  | 'done'
  | 'incomplete'
  | 'failed'
  | 'canceled'
  | 'connection-interrupted'
type ViewId = 'all' | ScenarioId

interface Scenario {
  id: ScenarioId
  label: string
  width: number
  messages: ChatMessage[]
  localNodeReady?: boolean
  localNodeRequired?: boolean
}

const scenarios: Scenario[] = [
  {
    id: 'queued',
    label: '排队准备',
    width: 599,
    messages: [
      task('请做一次只读诊断，不要修改文件、不要提交、不要发布。', 'queued'),
    ],
  },
  {
    id: 'dispatch',
    label: '连接节点',
    width: 720,
    messages: [
      task('请读取项目入口规则，并说明接下来会做什么。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      event({ type: 'runtime_status', status: 'running', phase: 'pc_dispatched', runtime: 'codex', message: 'PC 节点已接收任务，等待 Codex CLI 输出。' }),
    ],
  },
  {
    id: 'heartbeat',
    label: '等待输出',
    width: 720,
    messages: [
      task('请继续处理这个任务。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      timelineText('AI CLI 正在处理中，已等待 35s。'),
    ],
  },
  {
    id: 'recovery',
    label: '恢复连接',
    width: 599,
    messages: [
      task('请做一次只读诊断，不要修改文件、不要提交、不要发布。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给节点执行。' }),
      event({ type: 'runtime_status', status: 'running', phase: 'connection_recovering', runtime: 'codex', message: '正在恢复本轮任务连接。' }),
      progress('我正在恢复本轮任务连接。\n\n先确认本地会话状态，再接上后续步骤。'),
    ],
  },
  {
    id: 'server-update',
    label: '服务器更新',
    width: 720,
    messages: [
      task('请继续上一轮任务。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      event({ type: 'runtime_status', status: 'running', phase: 'server_updating', runtime: 'codex', message: '服务器正在更新升级，通信临时中断，会自动恢复。' }),
    ],
  },
  {
    id: 'win-update',
    label: 'Win端更新',
    width: 720,
    messages: [
      task('请继续上一轮任务。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      event({ type: 'runtime_status', status: 'running', phase: 'win_client_updating', runtime: 'codex', message: 'Win 端正在更新升级，重启后会自动恢复连接。' }),
    ],
  },
  {
    id: 'recovery-timeout',
    label: '恢复超时',
    width: 860,
    messages: [
      task('请继续上一轮任务。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      event({ type: 'runtime_status', status: 'running', phase: 'pc_cli_communication_recovering', runtime: 'codex', message: '通信临时中断，正在等待自动恢复。' }),
      event({ type: 'runtime_status', status: 'error', phase: 'pc_cli_recovery_timeout', runtime: 'codex', message: '自动恢复超时，仍未收到新的公开输出或完成事件。' }),
    ],
  },
  {
    id: 'thinking',
    label: '公开说明',
    width: 960,
    messages: [
      task('请读取最近 5 次 Git 提交、当前服务器版本、最近一次 Codex 会话记录，然后按格式总结。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      progress('我会先按项目入口规则重新读取权威说明，确认这轮仍然只做只读诊断，不进入修改、提交或发布流程。'),
      progress('当前工作区是干净的会话分支；接下来我会读取聊天记录查询文档，然后分别拉取 Git、线上版本和最近会话证据。'),
      event({ type: 'runtime_status', status: 'running', phase: 'thinking', runtime: 'codex', message: '正在整理下一步计划。' }),
    ],
  },
  {
    id: 'tools',
    label: '命令/文件/测试',
    width: 1040,
    messages: [
      task('请实现 UI 修复，运行 lint/build，不要发布。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      progress('我已经定位到任务过程组件；接下来会先调整信息分层，再验证展开态样式。'),
      toolCall('git log --oneline -5'),
      toolResult('33850f10 fix(pc): 钱一龙弱化任务停止按钮\nc91db0f1 fix(pc): 钱一龙优化任务开始态展示', 0),
      progress('命令结果回来后，我继续检查文件变更和验证步骤，展开过程应该按执行顺序继续往下读。'),
      event({
        type: 'tool_call',
        tool: 'file_change',
        status: 'running',
        args: { path: 'pc-frontend/src/features/dev/DevTaskGroup.tsx' },
        diff: {
          files: ['pc-frontend/src/features/dev/DevTaskGroup.tsx', 'pc-frontend/src/features/dev/TaskTimeline.tsx'],
          preview: '- 旧的过程折叠逻辑\n+ 公开 AI 进展常显，技术过程保留在展开区',
        },
      }),
      event({
        type: 'tool_result',
        tool: 'file_change',
        status: 'done',
        diff: {
          files: ['pc-frontend/src/features/dev/DevTaskGroup.tsx', 'pc-frontend/src/features/dev/TaskTimeline.tsx'],
          preview: '+ hideAssistantReplies\n+ expandAll\n+ TaskProgressNotes',
        },
        result: '2 files changed, 64 insertions, 9 deletions',
      }),
      toolCall('npm.cmd run lint'),
      toolResult('> elon-pc-frontend@0.1.0 lint\n> eslint src --ext ts,tsx --max-warnings 0\n\n', 0),
      toolCall('npm.cmd run build'),
      toolResult('vite v5.4.21 building for production...\n✓ 2288 modules transformed.\n✓ built in 3.54s', 0),
    ],
  },
  {
    id: 'command-running',
    label: '命令执行中',
    width: 860,
    messages: [
      task('请运行构建并告诉我结果。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      progress('我已经开始运行构建命令，等待命令返回后会继续判断是否需要修复。'),
      toolCall('npm.cmd run build'),
    ],
  },
  {
    id: 'tool-timeout',
    label: '工具超时',
    width: 860,
    messages: [
      task('请运行构建并告诉我结果。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      progress('我已经开始运行构建命令，等待命令返回后会继续判断是否需要修复。'),
      toolCall('npm.cmd run build'),
      event({ type: 'runtime_status', status: 'error', phase: 'pc_tool_result_timeout', runtime: 'codex', message: '构建命令长时间没有返回工具结果或完成事件。' }),
    ],
  },
  {
    id: 'tool-failed',
    label: '命令失败',
    width: 960,
    messages: [
      task('请运行前端检查并修复失败项。', 'running'),
      progress('我会先复现失败，再按报错定位到具体文件。'),
      toolCall('npm.cmd run build'),
      toolResult('exit=1\nsrc/task-progress-preview.tsx(1,8): error TS6133: React is declared but its value is never read.', 1),
      event({ type: 'runtime_status', status: 'running', phase: 'thinking', runtime: 'codex', message: '正在根据 TypeScript 报错定位修复点。' }),
    ],
  },
  {
    id: 'approval',
    label: '等待审批',
    width: 860,
    messages: [
      task('请提交并发布这个修复。', 'running'),
      progress('我已经完成本地验证；下一步涉及推送和发布，需要先等待工具审批。'),
      event({ type: 'tool_approval_required', tool: 'shell', status: 'pending', approval_id: 'apr_preview_publish', message: '需要运行 git push origin HEAD:main。' }),
    ],
  },
  {
    id: 'resume-required',
    label: '需要继续',
    width: 860,
    messages: [
      task('请继续上一轮任务。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      event({ type: 'runtime_status', status: 'error', phase: 'resume_required', runtime: 'codex', message: '自动恢复没有完成，请点击继续让 AI 检查当前工作区后接着处理。' }),
    ],
  },
  {
    id: 'timeout',
    label: '超时卡住',
    width: 860,
    messages: [
      task('请继续上一轮任务。', 'running'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      event({ type: 'runtime_status', status: 'running', phase: 'pc_cli_no_output_timeout', runtime: 'codex', message: 'PC 节点没有返回命令、工具结果或最终完成事件。' }),
    ],
  },
  {
    id: 'done',
    label: '任务完成',
    width: 960,
    messages: [
      task('请读取最近 5 次 Git 提交、当前服务器版本、最近一次 Codex 会话记录，然后按格式总结。', 'done'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      progress('我会先按项目入口规则重新读取权威说明，确认这轮仍然只做只读诊断，不进入修改、提交或发布流程。'),
      toolCall('git log --oneline -5'),
      toolResult('33850f10 fix(pc): 钱一龙弱化任务停止按钮\nc91db0f1 fix(pc): 钱一龙优化任务开始态展示', 0),
      progress('当前工作区是干净的会话分支；接下来我会读取聊天记录查询文档，然后分别拉取 Git、线上版本和最近会话证据。'),
      toolCall('Invoke-WebRequest http://43.139.149.158:8080/api/server/version'),
      toolResult('{"service":"elon-server","status":"ok","versionName":"0.3.1344","gitSha":"33850f104597198ea1d3ece941b06bd4dd139e89"}', 0),
      event({ type: 'usage', status: 'done', model: 'gpt-5.5', message: '输入 18k tokens，输出 2k tokens。' }),
      event({ type: 'runtime_summary', status: 'done', total_tools: 4, failed_tools: 0, message: '公开过程已结束，最终回复已生成。' }),
      result('我现在会取证这三块信息：本地 Git 最近 5 个提交、线上 `/api/server/version`，以及数据库里这个会话对应的最近 Codex thread。\n\n总结：最近一次发布是 `v0.3.1344`，对应提交 `33850f10`，主要修复任务恢复态停止按钮过重和换行问题。', 'done'),
    ],
  },
  {
    id: 'incomplete',
    label: '回复未完成',
    width: 960,
    messages: [
      task('请读取项目规则并完成只读诊断。', 'failed'),
      progress('我先读取项目规则，再继续完成诊断。'),
      toolCall('Get-Content -Raw CODEX.md'),
      toolResult('CODEX Project Entry', 0),
      event({ type: 'runtime_status', status: 'error', phase: 'final_reply_missing', runtime: 'codex', message: '最后一条公开说明之后仍有工具执行，但没有收到收尾回复。' }),
      result('PC CLI 执行未完成：Codex 没有返回收尾回复；本轮结果无法确认完成。请点击“重试处理”继续。', 'failed'),
    ],
  },
  {
    id: 'failed',
    label: '平台异常',
    width: 960,
    messages: [
      task('请做一次只读诊断，不要修改文件、不要提交、不要发布。', 'failed'),
      event({ type: 'runtime_status', status: 'running', phase: 'thinking', runtime: 'server-runtime', message: '正在调用模型生成下一步计划。' }),
      event({ type: 'runtime_status', status: 'error', phase: 'failed', runtime: 'server-runtime', message: '调用 server-runtime 失败：服务器 AI runtime 返回 502 Bad Gateway。' }),
      event({ type: 'runtime_summary', status: 'error', failed_tools: 0, total_tools: 0, message: '服务商返回错误，本轮没有生成有效回复。' }),
      result('任务遇到问题：平台 AI runtime 返回 502 Bad Gateway，本轮没有生成有效诊断。', 'failed'),
    ],
  },
  {
    id: 'canceled',
    label: '用户取消',
    width: 720,
    messages: [
      task('请继续上一轮任务。', 'canceled'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      progress('我已经接到任务，正在确认本机节点状态。'),
      event({ type: 'runtime_status', status: 'canceled', phase: 'canceled', runtime: 'codex', message: '用户已停止本轮任务。' }),
      result('任务已停止。', 'canceled'),
    ],
  },
  {
    id: 'connection-interrupted',
    label: '通信中断',
    width: 960,
    localNodeReady: false,
    localNodeRequired: true,
    messages: [
      task('请做一次只读诊断，不要修改文件、不要提交、不要发布。', 'canceled'),
      event({ type: 'pc_dispatch_started', status: 'running', message: '已获得 PC 会话执行权，开始交给 PC 节点执行。' }),
      progress('我已经接到任务，正在读取最近提交和线上版本。'),
      event({
        type: 'runtime_status',
        status: 'canceled',
        phase: 'connection_interrupted',
        runtime: 'codex',
        message: 'PC 节点通信中断；服务器正在更新升级，或 Win 端正在更新升级/重启。',
      }),
      result('AI 开发任务通信中断。任务已停止以避免重复执行；Win 端恢复后可以直接继续原任务。', 'canceled'),
    ],
  },
]

function Preview() {
  const liveConfig = useMemo(() => livePreviewConfigFromLocation(), [])
  const [activeId, setActiveId] = useState<ViewId>(() => initialViewFromLocation())
  const [expandAll, setExpandAll] = useState(() => initialExpandFromLocation())
  const activeScenario = scenarios.find((item) => item.id === activeId)
  const visibleScenarios = activeId === 'all' ? scenarios : activeScenario ? [activeScenario] : scenarios
  const toggleExpand = () => setExpandAll((value) => {
    const next = !value
    updatePreviewLocation(undefined, next)
    return next
  })
  const selectScenario = (view: ViewId) => {
    setActiveId(view)
    updatePreviewLocation(view, undefined)
  }
  if (liveConfig.enabled) {
    return (
      <>
        <style>{previewCss}</style>
        <LivePreview config={liveConfig} expandAll={expandAll} onToggleExpand={toggleExpand} />
      </>
    )
  }

  return (
    <main className="previewPage">
      <style>{previewCss}</style>
      <aside className="previewRail">
        <strong>任务交互生命周期</strong>
        <button
          type="button"
          className="expandToggle"
          data-active={expandAll ? 'true' : undefined}
          onClick={toggleExpand}
        >
          {expandAll ? '收起工具详情' : '展开工具详情'}
        </button>
        <div className="scenarioTabs">
          <button type="button" data-active={activeId === 'all' ? 'true' : undefined} onClick={() => selectScenario('all')}>
            全部生命周期
          </button>
          {scenarios.map((item) => (
            <button
              key={item.id}
              type="button"
              data-active={item.id === activeId ? 'true' : undefined}
              onClick={() => selectScenario(item.id)}
            >
              {item.label}
            </button>
          ))}
        </div>
      </aside>
      <section className={activeId === 'all' ? 'previewMatrix' : 'previewSingle'}>
        {visibleScenarios.map((scenario) => (
          <ScenarioPreview key={scenario.id} scenario={scenario} expandAll={expandAll} />
        ))}
      </section>
    </main>
  )
}

function initialViewFromLocation(): ViewId {
  const value = new URLSearchParams(window.location.search).get('view') ?? ''
  if (value === 'all') return 'all'
  return scenarios.some((item) => item.id === value) ? value as ScenarioId : 'all'
}

function initialExpandFromLocation(): boolean {
  const value = new URLSearchParams(window.location.search).get('expand')
  return value != null && value !== '0'
}
function updatePreviewLocation(view: ViewId | undefined, expand: boolean | undefined) {
  const url = new URL(window.location.href)
  if (view !== undefined) url.searchParams.set('view', view)
  if (expand !== undefined) url.searchParams.set('expand', expand ? '1' : '0')
  window.history.replaceState(null, '', `${url.pathname}${url.search}${url.hash}`)
}
function task(content: string, status: string): ChatMessage {
  return {
    id: `${taskId}-request-${status}`,
    kind: 'ai_task',
    task_id: taskId,
    task_status: status,
    sender_name: '钱一龙',
    content,
    created_at: startedAt,
  }
}

function progress(content: string): ChatMessage {
  return {
    id: `${taskId}-progress-${content.slice(0, 12)}`,
    kind: 'ai_progress',
    task_id: taskId,
    content,
    assistant_progress_event: true,
    cli_name: 'codex',
    created_at: startedAt,
  }
}

function timelineText(content: string): ChatMessage {
  return {
    id: `${taskId}-timeline-${content.slice(0, 12)}`,
    kind: 'ai_progress',
    task_id: taskId,
    content,
    cli_name: 'codex',
    created_at: startedAt,
  }
}

function result(content: string, status: string): ChatMessage {
  return {
    id: `${taskId}-result-${status}`,
    kind: 'ai_result',
    task_id: taskId,
    task_status: status,
    content,
    created_at: startedAt,
  }
}

function event(value: ToolEvent): ChatMessage {
  return {
    id: `${taskId}-event-${value.type}-${value.phase ?? value.tool ?? value.status ?? 'item'}-${value.approval_id ?? ''}`,
    kind: 'ai_progress',
    task_id: taskId,
    content: JSON.stringify(value),
    created_at: startedAt,
  }
}

function toolCall(command: string): ChatMessage {
  return event({ type: 'tool_call', tool: 'shell', status: 'running', args: { command } })
}

function toolResult(output: string, exitCode: number): ChatMessage {
  return event({ type: 'tool_result', tool: 'shell', status: exitCode === 0 ? 'done' : 'error', result: output, exit_code: exitCode })
}

const previewCss = `
:root {
  --surface: #111318;
  --surface-2: #151922;
  --border: rgba(148, 163, 184, .16);
  --text: #eef3fb;
  --text-soft: #d8e1ee;
  --text-muted: #7f8ca3;
  --accent: #5865f2;
}

html,
body,
#root {
  min-height: 100%;
}

body {
  margin: 0;
  background: #050607;
  color: var(--text);
  font-family: Inter, "Microsoft YaHei", system-ui, sans-serif;
}

button {
  font-family: inherit;
}

.previewPage {
  box-sizing: border-box;
  min-height: 100vh;
  display: grid;
  grid-template-columns: 188px minmax(0, 1fr);
  gap: 28px;
  padding: 22px 28px;
  background: #050607;
}

.previewRail {
  position: sticky;
  top: 22px;
  align-self: start;
  display: grid;
  gap: 14px;
}

.previewRail > strong {
  color: #dce5f3;
  font-size: 13px;
}

.expandToggle,
.scenarioTabs button {
  min-height: 30px;
  border: 0;
  border-radius: 0;
  background: transparent;
  color: #8d99aa;
  padding: 0;
  text-align: left;
  font-size: 12px;
  font-weight: 750;
  cursor: pointer;
}

.expandToggle {
  color: #cbd6ff;
  background: transparent;
}

.expandToggle[data-active="true"] {
  color: #b8f2cc;
  background: transparent;
}

.scenarioTabs {
  display: grid;
  gap: 6px;
}

.scenarioTabs button:hover,
.scenarioTabs button[data-active="true"] {
  border-color: transparent;
  background: transparent;
  color: #eef3fb;
}

.scenarioTabs button[data-active="true"] {
  text-decoration: underline;
  text-underline-offset: 5px;
  text-decoration-color: rgba(148, 163, 184, .45);
}

.previewMatrix {
  display: grid;
  gap: 22px;
  align-content: start;
}

.previewSingle {
  width: 100%;
}

.scenarioFrame {
  width: 100%;
  min-height: 120px;
  padding-bottom: 12px;
  border-bottom: 1px solid rgba(148, 163, 184, .10);
}

.scenarioFrame > header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 10px 44px;
  color: #dce5f3;
  font-size: 12px;
}

.scenarioFrame > header span {
  color: #6f7b8f;
  font-size: 11px;
}

.conversationReplay {
  height: min(720px, calc(100vh - 76px));
  min-height: 420px;
  display: grid;
  grid-template-rows: 42px minmax(0, 1fr) 66px;
  overflow: hidden;
  background: #111318;
  border: 1px solid rgba(148, 163, 184, .14);
  box-shadow: 0 18px 54px rgba(0, 0, 0, .28);
}

.previewMatrix .conversationReplay {
  height: 520px;
}

.replayTopbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
  padding: 0 18px;
  border-bottom: 1px solid rgba(148, 163, 184, .10);
  background: #151922;
}

.replayTopbar strong {
  min-width: 0;
  color: #edf3ff;
  font-size: 13px;
  font-weight: 800;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.replayTopbar span {
  min-width: 0;
  color: #7f8ca3;
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.replayStatus {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  min-width: 0;
}

.replayStatus button {
  min-height: 25px;
  padding: 3px 8px;
  border: 1px solid rgba(137, 174, 255, .28);
  border-radius: 4px;
  background: rgba(82, 122, 214, .08);
  color: #9db3e7;
  font: inherit;
  font-size: 11px;
  cursor: pointer;
}

.replayStatus button[data-node-ready='true'] {
  border-color: rgba(46, 168, 97, .3);
  background: rgba(46, 168, 97, .08);
  color: #85d5a4;
  cursor: default;
}

.replayComposer {
  align-self: stretch;
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr) auto 32px;
  align-items: center;
  gap: 8px;
  padding: 11px 14px;
  border-top: 1px solid rgba(148, 163, 184, .10);
  background: linear-gradient(180deg, rgba(17, 19, 24, .86), #111318);
}

.replayComposer button,
.replayComposer span {
  height: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(148, 163, 184, .18);
  background: #20242f;
  color: #cdd7e7;
  font-size: 12px;
  font-weight: 800;
}

.replayComposer button {
  width: 32px;
  padding: 0;
  cursor: pointer;
}

.replayComposer span {
  padding: 0 10px;
}

.replayInput {
  min-width: 0;
  height: 40px;
  display: flex;
  align-items: center;
  padding: 0 14px;
  background: #1b1f29;
  border: 1px solid rgba(148, 163, 184, .14);
  color: #7f8ca3;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.livePreviewPage {
  grid-template-columns: 220px minmax(0, 1fr);
}

.liveFacts {
  display: grid;
  gap: 7px;
  padding-top: 4px;
  color: #748198;
  font-family: 'Cascadia Code', 'Consolas', monospace;
  font-size: 11px;
  line-height: 1.45;
}

.liveFacts span {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.livePreviewStage {
  max-width: 1120px;
}

.liveScenarioFrame {
  max-width: 100%;
}

.liveConversationReplay {
  height: calc(100vh - 76px);
  min-height: 560px;
}

.liveError {
  display: grid;
  align-content: center;
  padding: 24px 44px;
  color: #ffb1b4;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
}

@media (max-width: 720px) {
  .previewPage {
    grid-template-columns: 1fr;
    padding: 16px 12px;
  }

  .previewRail {
    position: static;
  }

  .scenarioTabs {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .conversationReplay {
    height: calc(100vh - 220px);
    min-height: 420px;
  }

  .replayComposer {
    grid-template-columns: 32px minmax(0, 1fr) 32px;
  }

  .replayComposer span {
    display: none;
  }
}
`

createRoot(document.getElementById('root') as HTMLElement).render(<Preview />)
