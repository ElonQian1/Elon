import type { Message } from '../features/conversation/types'
import type { ReplayCapture } from './model'

const realFailureTask = 'capture-task-data-root-failure'
const realFailureConversation = 'capture-conversation-data-root-failure'
const realFailureText = '请做一次只读诊断，不要修改文件、不要提交、不要发布。\n\n请先读取最近 5 次 Git 提交、当前服务器版本、最近一次 Codex 会话记录，然后按下面格式总结：\n1. 最近修了什么\n2. 影响哪里\n3. 是否已经发布\n4. 还有什么风险\n\n处理过程中请像 Codex 桌面端一样，在每个关键步骤前先用一句自然语言告诉我你准备做什么或刚发现了什么，不要只显示命令结果。'
const dataRootError = 'PC CLI 执行失败: PC 节点尚未配置有效的统一数据根，已阻止项目工作区回落到系统盘'

export const dataRootFailureCapture: ReplayCapture = {
  version: 1,
  id: 'data-root-failure',
  title: '真实记录：PC 数据根未配置',
  description: '来自“一龙项目自项目”会话。任务在 Codex CLI 启动前失败，保留原始毫秒级启动节奏。',
  source: 'golden',
  projectId: 'elon-self',
  channelId: 'pch_capture_ai_development',
  conversationId: realFailureConversation,
  taskId: realFailureTask,
  startedAt: '2026-07-15T05:09:09.723Z',
  taskStatus: 'failed',
  taskError: dataRootError,
  messages: [
    taskMessage(realFailureTask, realFailureConversation, realFailureText, '2026-07-15T05:09:09.723Z'),
    progressMessage(realFailureTask, realFailureConversation, '2026-07-15T05:09:09.777Z', {
      type: 'runtime_status', phase: 'conversation_worktree_ready', status: 'running', runtime: 'codex',
      message: '本机会话隔离已启用。',
    }),
    progressMessage(realFailureTask, realFailureConversation, '2026-07-15T05:09:09.790Z', {
      type: 'runtime_status', phase: 'pc_execution_granted', status: 'running', runtime: 'codex',
      message: '已获得 PC 会话执行权。',
    }),
    progressMessage(realFailureTask, realFailureConversation, '2026-07-15T05:09:09.805Z', {
      type: 'runtime_status', phase: 'pc_cli_execution_granted', status: 'running', runtime: 'codex',
      message: '已获得 PC 节点本机 CLI 执行权。',
    }),
    progressMessage(realFailureTask, realFailureConversation, '2026-07-15T05:09:09.818Z', {
      type: 'runtime_status', phase: 'hot_session_ready', status: 'running', runtime: 'codex',
      message: '已建立本机会话状态。',
    }),
    progressMessage(realFailureTask, realFailureConversation, '2026-07-15T05:09:09.827Z', {
      type: 'runtime_status', phase: 'pc_node_connecting', status: 'running', runtime: 'codex',
      message: '正在直连 PC 节点一龙4060。',
    }),
    progressMessage(realFailureTask, realFailureConversation, '2026-07-15T05:09:10.105Z', {
      type: 'runtime_status', phase: 'pc_dispatched', status: 'running', runtime: 'codex',
      message: 'PC 节点已接收任务，等待 Codex CLI 输出。',
    }),
    resultMessage(realFailureTask, realFailureConversation, '2026-07-15T05:09:10.119Z', 'failed', `AI 开发任务失败。\n${dataRootError}`, dataRootError),
  ],
  events: [
    rawEvent(1, '2026-07-15T05:09:09.772Z', 'runtime_status', 'conversation_worktree_ready', '本机会话隔离已启用。'),
    rawEvent(2, '2026-07-15T05:09:09.788Z', 'runtime_status', 'pc_execution_granted', '已获得 PC 会话执行权。'),
    rawEvent(3, '2026-07-15T05:09:09.798Z', 'runtime_status', 'pc_cli_execution_granted', '已获得 PC 节点本机 CLI 执行权。'),
    rawEvent(4, '2026-07-15T05:09:09.813Z', 'runtime_status', 'hot_session_ready', '已建立本机会话状态。'),
    rawEvent(5, '2026-07-15T05:09:09.822Z', 'runtime_status', 'pc_node_connecting', '正在直连 PC 节点一龙4060。'),
    rawEvent(6, '2026-07-15T05:09:10.098Z', 'pc_dispatch_started', 'pc_dispatched', 'PC 节点已接收任务。'),
    {
      seq: 7,
      createdAt: '2026-07-15T05:09:10.111Z',
      event: { type: 'error', status: 'error', message: dataRootError },
    },
    {
      seq: 8,
      createdAt: '2026-07-15T05:09:12.506Z',
      event: { type: 'completion_replayed', status: 'failed', message: '已重放任务完成状态。' },
    },
  ],
  lastEventSeq: 8,
}

const successTask = 'capture-task-complete-diagnostic'
const successConversation = 'capture-conversation-complete-diagnostic'

export const completeSuccessCapture: ReplayCapture = {
  version: 1,
  id: 'complete-success',
  title: '完整记录：诊断、命令、文件与验证',
  description: '覆盖自然语言进展、命令输入输出、文件修改、测试构建、运行摘要和最终回复。',
  source: 'golden',
  projectId: 'elon-self',
  channelId: 'pch_capture_ai_development',
  conversationId: successConversation,
  taskId: successTask,
  startedAt: '2026-07-15T06:20:00.000Z',
  taskStatus: 'done',
  messages: [
    taskMessage(successTask, successConversation, '请检查任务过程样式，修复问题并运行 lint/build，不要发布。', '2026-07-15T06:20:00.000Z'),
    plainProgress(successTask, successConversation, '2026-07-15T06:20:00.650Z', '我先读取项目入口规则和任务过程组件，确认本次修改边界。'),
    progressMessage(successTask, successConversation, '2026-07-15T06:20:01.100Z', {
      type: 'tool_call', id: 'tool-1', tool: 'shell', status: 'running', args: { command: 'Get-Content -Raw AGENTS.md' },
    }),
    progressMessage(successTask, successConversation, '2026-07-15T06:20:01.720Z', {
      type: 'tool_result', id: 'tool-1', tool: 'shell', status: 'done', exit_code: 0, result: '# 一龙项目 AI 工作入口\n最后更新：2026-07-14',
    }),
    plainProgress(successTask, successConversation, '2026-07-15T06:20:03.000Z', '规则和渲染链路已经确认。接下来调整信息层级，同时保留命令与文件的轻量详情。'),
    progressMessage(successTask, successConversation, '2026-07-15T06:20:04.200Z', {
      type: 'tool_result', tool: 'file_change', id: 'file-1', status: 'done', args: { changes: [
        { path: 'pc-frontend/src/features/dev/DevTaskGroup.tsx' },
        { path: 'pc-frontend/src/features/dev/DevTaskGroup.module.css' },
      ] },
      diff: { files: ['pc-frontend/src/features/dev/DevTaskGroup.tsx', 'pc-frontend/src/features/dev/DevTaskGroup.module.css'], preview: '+ preservePublicProgress\n+ lightweightToolDetails\n+ stableTaskTurn' },
    }),
    plainProgress(successTask, successConversation, '2026-07-15T06:20:05.000Z', '界面层级已经调整，现在运行 lint 和 build 验证类型、样式与生产构建。'),
    progressMessage(successTask, successConversation, '2026-07-15T06:20:05.300Z', {
      type: 'tool_call', id: 'tool-2', tool: 'shell', status: 'running', args: { command: 'npm.cmd run lint && npm.cmd run build' },
    }),
    progressMessage(successTask, successConversation, '2026-07-15T06:20:12.800Z', {
      type: 'tool_result', id: 'tool-2', tool: 'shell', status: 'done', exit_code: 0, result: 'eslint: passed\ntsc: passed\nvite build: passed',
    }),
    progressMessage(successTask, successConversation, '2026-07-15T06:20:13.050Z', {
      type: 'runtime_summary', status: 'done', duration_ms: 13050, total_tools: 2, failed_tools: 0, input_tokens: 6840, output_tokens: 1240,
    }),
    resultMessage(successTask, successConversation, '2026-07-15T06:20:13.300Z', 'done', '已完成任务过程界面调整。自然语言进展保留在主对话流中，命令和文件详情可就地展开；lint 与 build 均通过。'),
  ],
  events: [
    rawEvent(1, '2026-07-15T06:20:00.180Z', 'runtime_status', 'thinking', '正在读取项目规则。'),
    { seq: 2, createdAt: '2026-07-15T06:20:01.100Z', event: { type: 'tool_call', id: 'tool-1', tool: 'shell', status: 'running' } },
    { seq: 3, createdAt: '2026-07-15T06:20:01.720Z', event: { type: 'tool_result', id: 'tool-1', tool: 'shell', status: 'done', exit_code: 0 } },
    { seq: 4, createdAt: '2026-07-15T06:20:04.200Z', event: { type: 'tool_result', tool: 'file_change', id: 'file-1', status: 'done', files: 2 } },
    { seq: 5, createdAt: '2026-07-15T06:20:05.300Z', event: { type: 'tool_call', id: 'tool-2', tool: 'shell', status: 'running' } },
    { seq: 6, createdAt: '2026-07-15T06:20:12.800Z', event: { type: 'tool_result', id: 'tool-2', tool: 'shell', status: 'done', exit_code: 0 } },
    { seq: 7, createdAt: '2026-07-15T06:20:13.050Z', event: { type: 'runtime_summary', status: 'done', duration_ms: 13050 } },
    { seq: 8, createdAt: '2026-07-15T06:20:13.300Z', event: { type: 'completed', status: 'done', message: '任务完成。' } },
  ],
  lastEventSeq: 8,
}

export const replayCaptures = [dataRootFailureCapture, completeSuccessCapture]

export function replayCaptureById(id: string): ReplayCapture | undefined {
  return replayCaptures.find((capture) => capture.id === id)
}

function taskMessage(taskId: string, conversationId: string, content: string, createdAt: string): Message {
  return {
    id: `${taskId}-request`, kind: 'ai_task', role: 'user', content, text: content,
    task_id: taskId, taskId, task_status: 'running', taskStatus: 'running',
    conversation_id: conversationId, conversationId, user_id: 'capture-user', sender_name: '钱一龙', outgoing: true,
    created_at: createdAt,
  }
}

function plainProgress(taskId: string, conversationId: string, createdAt: string, content: string): Message {
  return {
    id: `${taskId}-progress-${createdAt}`, kind: 'ai_progress', role: 'assistant', content, text: content,
    task_id: taskId, taskId, task_status: 'running', taskStatus: 'running', conversation_id: conversationId, conversationId,
    assistant_progress_event: true, cli_name: 'codex',
    created_at: createdAt,
  }
}

function progressMessage(taskId: string, conversationId: string, createdAt: string, event: Record<string, unknown>): Message {
  return {
    id: `${taskId}-event-${createdAt}`, kind: 'ai_progress', role: 'assistant', content: JSON.stringify(event),
    task_id: taskId, taskId, task_status: 'running', taskStatus: 'running', conversation_id: conversationId, conversationId,
    created_at: createdAt,
  }
}

function resultMessage(taskId: string, conversationId: string, createdAt: string, status: string, content: string, error?: string): Message {
  return {
    id: `${taskId}-result`, kind: 'ai_result', role: 'assistant', content, text: content,
    task_id: taskId, taskId, task_status: status, taskStatus: status, task_error: error, taskError: error,
    conversation_id: conversationId, conversationId, created_at: createdAt,
  }
}

function rawEvent(seq: number, createdAt: string, type: string, phase: string, message: string) {
  return { seq, createdAt, event: { type, phase, status: 'running', runtime: 'codex', message } }
}
