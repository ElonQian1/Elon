import { useCallback, useEffect, useMemo, useState } from 'react'
import { api } from '../api/client'
import type { Message, ProjectSpace } from '../features/conversation/types'
import { clean } from '../lib/utils'
import { replayCaptureById } from './captures'
import type { ReplayCapture, ReplayPreviewConfig, ReplayRawEvent } from './model'

interface SnapshotResponse {
  task?: Record<string, unknown>
  messages?: Message[]
  events?: Array<Record<string, unknown>>
  has_more?: boolean
  hasMore?: boolean
  last_event_seq?: number
  lastEventSeq?: number
}

interface ReplayCaptureState {
  capture: ReplayCapture
  loading: boolean
  error: string
  refreshedAt: number
  live: boolean
}

const TERMINAL_STATUSES = new Set(['done', 'completed', 'success', 'failed', 'error', 'canceled', 'cancelled', 'interrupted'])

export function useTaskReplayCapture(config: ReplayPreviewConfig) {
  const fallback = useMemo(() => replayCaptureById(config.captureId) ?? replayCaptureById('data-root-failure')!, [config.captureId])
  const [state, setState] = useState<ReplayCaptureState>({
    capture: fallback,
    loading: Boolean(config.taskId),
    error: '',
    refreshedAt: 0,
    live: Boolean(config.taskId),
  })

  const refresh = useCallback(async () => {
    if (!config.taskId) {
      setState({ capture: fallback, loading: false, error: '', refreshedAt: Date.now(), live: false })
      return
    }
    setState((previous) => ({ ...previous, loading: true, error: '', live: true }))
    try {
      const channelId = config.channelId || await resolveReplayChannel(config.projectId)
      if (!channelId) throw new Error('没有找到 AI 开发频道，无法读取任务快照。')
      const capture = await loadSnapshotCapture({ ...config, channelId })
      setState({ capture, loading: false, error: '', refreshedAt: Date.now(), live: true })
    } catch (error) {
      setState((previous) => ({
        ...previous,
        loading: false,
        error: clean((error as { message?: string }).message) || '读取真实任务快照失败。',
      }))
    }
  }, [config, fallback])

  useEffect(() => {
    void refresh()
  }, [refresh])

  useEffect(() => {
    if (!state.live || TERMINAL_STATUSES.has(clean(state.capture.taskStatus).toLowerCase())) return
    const timer = window.setInterval(() => void refresh(), 1200)
    return () => window.clearInterval(timer)
  }, [refresh, state.capture.taskStatus, state.live])

  const importCapture = useCallback((capture: ReplayCapture) => {
    validateImportedCapture(capture)
    setState({ capture: { ...capture, source: 'import' }, loading: false, error: '', refreshedAt: Date.now(), live: false })
  }, [])

  return { ...state, refresh, importCapture }
}

async function loadSnapshotCapture(config: ReplayPreviewConfig): Promise<ReplayCapture> {
  const snapshots: SnapshotResponse[] = []
  let since = 0
  for (let page = 0; page < 20; page += 1) {
    const snapshot = await api.get<SnapshotResponse>(snapshotUrl(config, since))
    snapshots.push(snapshot)
    const events = normalizeEvents(snapshot.events ?? [])
    const nextSince = events.reduce((maximum, event) => Math.max(maximum, event.seq ?? 0), since)
    const hasMore = Boolean(snapshot.has_more ?? snapshot.hasMore)
    if (!hasMore || nextSince <= since) break
    since = nextSince
  }

  const first = snapshots[0] ?? {}
  const latest = snapshots[snapshots.length - 1] ?? first
  const task = latest.task ?? first.task ?? {}
  const messages = dedupeMessages(snapshots.flatMap((snapshot) => snapshot.messages ?? []))
    .filter((message) => messageReplayTaskId(message) === config.taskId)
  const events = dedupeEvents(snapshots.flatMap((snapshot) => normalizeEvents(snapshot.events ?? [])))
  const conversationId = config.conversationId
    || clean(task.conversation_id ?? task.conversationId)
    || clean(messages.find((message) => clean(message.conversation_id ?? message.conversationId))?.conversation_id)
    || `replay-conversation-${config.taskId}`
  const startedAt = clean(task.created_at ?? task.createdAt ?? task.started_at ?? task.startedAt)
    || earliestTimestamp(messages.map((message) => clean(message.created_at)), events.map((event) => event.createdAt))
  const taskStatus = clean(task.status) || latestTaskStatus(messages) || 'running'
  const taskError = clean(task.error ?? task.last_error ?? task.lastError)
  return {
    version: 1,
    id: `snapshot-${config.taskId}`,
    title: `真实快照：${shortId(config.taskId)}`,
    description: '从任务 snapshot 接口录制，保留频道消息和原始事件时间戳。',
    source: 'snapshot',
    projectId: config.projectId,
    channelId: config.channelId,
    conversationId,
    taskId: config.taskId,
    startedAt,
    taskStatus,
    taskError: taskError || undefined,
    messages: messages.map((message) => ({
      ...message,
      conversation_id: clean(message.conversation_id ?? message.conversationId) || conversationId,
      conversationId: clean(message.conversationId ?? message.conversation_id) || conversationId,
    })),
    events,
    hasMoreEvents: Boolean(latest.has_more ?? latest.hasMore),
    lastEventSeq: Number(latest.last_event_seq ?? latest.lastEventSeq ?? events[events.length - 1]?.seq ?? 0),
  }
}

function snapshotUrl(config: ReplayPreviewConfig, since: number): string {
  return `/api/projects/${encodeURIComponent(config.projectId)}/channels/${encodeURIComponent(config.channelId)}/ai-tasks/${encodeURIComponent(config.taskId)}/snapshot?since=${since}&limit=200`
}

async function resolveReplayChannel(projectId: string): Promise<string> {
  const space = await api.get<ProjectSpace>(`/api/projects/${encodeURIComponent(projectId)}/space`)
  const channels = Array.isArray(space.channels) ? space.channels : []
  const channel = channels.find((candidate) => candidate.kind === 'ai_development')
    ?? channels.find((candidate) => /ai|开发|codex/i.test(candidate.name ?? ''))
  return clean(channel?.id)
}

function normalizeEvents(events: Array<Record<string, unknown>>): ReplayRawEvent[] {
  return events.map((raw, index) => {
    const nested = isRecord(raw.event) ? raw.event : isRecord(raw.data) ? raw.data : raw
    return {
      seq: finiteNumber(raw.seq ?? raw.sequence ?? nested.seq) ?? index + 1,
      createdAt: clean(raw.created_at ?? raw.createdAt ?? raw.timestamp ?? nested.created_at ?? nested.createdAt),
      event: nested,
    }
  })
}

function dedupeMessages(messages: Message[]): Message[] {
  const seen = new Set<string>()
  return messages.filter((message, index) => {
    const key = clean(message.id) || `${clean(message.created_at)}|${clean(message.kind ?? message.role)}|${clean(message.content ?? message.text)}|${index}`
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
}

function dedupeEvents(events: ReplayRawEvent[]): ReplayRawEvent[] {
  const seen = new Set<string>()
  return events.filter((event, index) => {
    const key = event.seq != null ? `seq:${event.seq}` : `${event.createdAt}|${JSON.stringify(event.event)}|${index}`
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })
}

function latestTaskStatus(messages: Message[]): string {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const status = clean(messages[index]?.task_status ?? messages[index]?.taskStatus)
    if (status) return status
  }
  return ''
}

function messageReplayTaskId(message: Message): string {
  return clean(message.task_id ?? message.taskId)
}

function earliestTimestamp(messageTimes: string[], eventTimes: string[]): string {
  const timestamps = [...messageTimes, ...eventTimes]
    .map((value) => Date.parse(value))
    .filter(Number.isFinite)
  return timestamps.length > 0 ? new Date(Math.min(...timestamps)).toISOString() : new Date().toISOString()
}

function validateImportedCapture(value: ReplayCapture) {
  if (!value || value.version !== 1 || !value.taskId || !value.conversationId) {
    throw new Error('回放文件格式无效。')
  }
  if (!Array.isArray(value.messages) || !Array.isArray(value.events)) {
    throw new Error('回放文件缺少 messages 或 events。')
  }
}

function finiteNumber(value: unknown): number | undefined {
  const number = Number(value)
  return Number.isFinite(number) ? number : undefined
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function shortId(value: string): string {
  return value.length > 18 ? `${value.slice(0, 12)}…` : value
}
