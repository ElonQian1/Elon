import { create } from 'zustand'
import { formatTime } from '../../lib/utils'
import { localJson } from './localApi'
import { safeNodeAdminUrl } from '../../lib/utils'
import type {
  DoctorMessage, DoctorSessionSummary, DoctorResult,
  DoctorSection, SnapshotData, MemoryItem,
} from './types'

function normalizeMessages(raw: unknown[]): DoctorMessage[] {
  return raw.map((m) => {
    const msg = m as Record<string, unknown>
    const createdAtMs = Number(msg.createdAtMs) || Date.now()
    return {
      id: String(msg.id ?? ''),
      role: msg.role === 'user' ? 'user' : 'assistant',
      content: String(msg.content ?? ''),
      kind: String(msg.kind ?? ''),
      createdAtMs,
      time: formatTime(createdAtMs),
    } satisfies DoctorMessage
  })
}

interface DoctorState {
  nodeAdminUrl: string
  sessions: DoctorSessionSummary[]
  sessionsLoaded: boolean
  activeSessionId: string
  messages: DoctorMessage[]
  section: DoctorSection
  problem: string
  analysis: string
  snapshot: SnapshotData | null
  memories: MemoryItem[] | null
  result: DoctorResult | null
  selectedAgentName: string

  setNodeAdminUrl: (url: string) => void
  setSelectedAgent: (name: string) => void
  loadSessions: () => Promise<void>
  loadSession: (id: string) => Promise<void>
  createSession: () => Promise<void>
  setSection: (s: DoctorSection) => void
  loadSnapshot: () => Promise<void>
  analyze: (problem?: string) => Promise<void>
  repair: (action: string, adapterName?: string) => Promise<void>
  loadMemory: () => Promise<void>
  saveMemory: () => Promise<void>
}

export const useDoctorStore = create<DoctorState>()((set, get) => ({
  nodeAdminUrl: safeNodeAdminUrl(),
  sessions: [],
  sessionsLoaded: false,
  activeSessionId: '',
  messages: [],
  section: 'diagnosis',
  problem: '',
  analysis: '',
  snapshot: null,
  memories: null,
  result: null,
  selectedAgentName: '',

  setNodeAdminUrl: (url) => set({ nodeAdminUrl: safeNodeAdminUrl(url) }),
  setSelectedAgent: (name) => set({ selectedAgentName: name }),

  loadSessions: async () => {
    const { nodeAdminUrl } = get()
    try {
      const data = await localJson<{ items?: unknown[] }>(nodeAdminUrl, '/api/doctor/sessions')
      const sessions = (data.items ?? []) as DoctorSessionSummary[]
      set({ sessions, sessionsLoaded: true })
      if (!get().activeSessionId && sessions.length > 0) {
        await get().loadSession(sessions[0].id)
      }
    } catch {
      set({ sessions: [], sessionsLoaded: true })
    }
  },

  loadSession: async (id) => {
    const { nodeAdminUrl } = get()
    set({ result: { kind: '', text: '正在读取诊断会话…' } })
    try {
      const data = await localJson<{ session?: Record<string, unknown> }>(
        nodeAdminUrl, `/api/doctor/sessions/${encodeURIComponent(id)}`,
      )
      const session = data.session ?? {}
      const messages = normalizeMessages(Array.isArray(session.messages) ? session.messages : [])
      set({
        activeSessionId: String(session.id ?? id),
        messages,
        problem: [...messages].reverse().find((m) => m.role === 'user')?.content ?? '',
        analysis: [...messages].reverse().find((m) => m.role === 'assistant' && m.kind === 'ok')?.content ?? '',
        result: null,
      })
    } catch (err) {
      set({ result: { kind: 'err', text: `读取失败：${(err as Error).message}` } })
    }
  },

  createSession: async () => {
    const { nodeAdminUrl } = get()
    set({ section: 'diagnosis', result: { kind: '', text: '正在创建新的诊断会话…' }, messages: [], problem: '', analysis: '' })
    try {
      const data = await localJson<{ session?: Record<string, unknown>; sessions?: DoctorSessionSummary[] }>(
        nodeAdminUrl, '/api/doctor/sessions',
        { method: 'POST', body: JSON.stringify({ title: '新的电脑诊断' }) },
      )
      if (data.sessions) set({ sessions: data.sessions })
      const session = data.session ?? {}
      set({ activeSessionId: String(session.id ?? ''), result: { kind: 'ok', text: '新的诊断会话已创建。' } })
    } catch (err) {
      set({ result: { kind: 'err', text: `创建失败：${(err as Error).message}` } })
    }
  },

  setSection: (section) => set({ section }),

  loadSnapshot: async () => {
    const { nodeAdminUrl, activeSessionId } = get()
    set({ result: { kind: '', text: '正在采集只读系统快照…' } })
    try {
      let sessionId = activeSessionId
      if (!sessionId) {
        const d = await localJson<{ session?: Record<string, unknown>; sessions?: DoctorSessionSummary[] }>(
          nodeAdminUrl, '/api/doctor/sessions',
          { method: 'POST', body: JSON.stringify({ title: '只读系统体检' }) },
        )
        if (d.sessions) set({ sessions: d.sessions })
        sessionId = String(d.session?.id ?? '')
        set({ activeSessionId: sessionId })
      }
      const data = await localJson<{ snapshot?: SnapshotData; sessions?: DoctorSessionSummary[]; session?: Record<string, unknown> }>(
        nodeAdminUrl, `/api/doctor/snapshot?sessionId=${encodeURIComponent(sessionId)}`,
      )
      if (data.sessions) set({ sessions: data.sessions })
      if (data.session) {
        const messages = normalizeMessages(Array.isArray((data.session as Record<string, unknown>).messages) ? ((data.session as Record<string, unknown>).messages as unknown[]) : [])
        set({ messages })
      }
      const count = Array.isArray(data.snapshot?.commands) ? data.snapshot!.commands!.length : 0
      set({ snapshot: data.snapshot ?? null, result: { kind: 'ok', text: `只读体检完成，采集 ${count} 组系统状态。` } })
    } catch (err) {
      set({ result: { kind: 'err', text: `体检失败：${(err as Error).message}` } })
    }
  },

  analyze: async (problemFromArg?: string) => {
    const { nodeAdminUrl, activeSessionId, problem: storedProblem, messages, selectedAgentName } = get()
    const problem = problemFromArg ?? storedProblem
    if (!problem) { set({ result: { kind: 'err', text: '请先描述电脑问题。' } }); return }
    const now = Date.now()
    const userMsg: DoctorMessage = { id: `local-${now}-u`, role: 'user', content: problem, kind: '', createdAtMs: now, time: formatTime(now) }
    const assistantMsg: DoctorMessage = { id: `local-${now}-a`, role: 'assistant', content: '正在采集只读快照，并请求远程 AI 分析…', kind: '', createdAtMs: now + 1, time: formatTime(now) }
    set({ section: 'diagnosis', problem, messages: [...messages, userMsg, assistantMsg], result: { kind: '', text: '正在分析…' } })
    try {
      const data = await localJson<{ analysis?: string; snapshot?: SnapshotData; sessions?: DoctorSessionSummary[]; session?: Record<string, unknown> }>(
        nodeAdminUrl, '/api/doctor/analyze',
        { method: 'POST', body: JSON.stringify({ problem, sessionId: activeSessionId || null, agent: selectedAgentName || null }) },
      )
      if (data.sessions) set({ sessions: data.sessions })
      if (data.session) {
        const msgs = normalizeMessages(Array.isArray((data.session as Record<string, unknown>).messages) ? ((data.session as Record<string, unknown>).messages as unknown[]) : [])
        set({ messages: msgs, analysis: data.analysis ?? '', snapshot: data.snapshot ?? get().snapshot, result: { kind: 'ok', text: '远程 AI 已完成分析。' } })
      } else {
        const updated = get().messages.map((m) =>
          m.id === assistantMsg.id ? { ...m, content: data.analysis ?? '已完成分析。', kind: 'ok' } : m,
        )
        set({ messages: updated, analysis: data.analysis ?? '', snapshot: data.snapshot ?? get().snapshot, result: { kind: 'ok', text: '远程 AI 已完成分析。' } })
      }
    } catch (err) {
      const updated = get().messages.map((m) =>
        m.id === assistantMsg.id ? { ...m, content: `分析失败：${(err as Error).message}`, kind: 'err' } : m,
      )
      set({ messages: updated, result: { kind: 'err', text: `分析失败：${(err as Error).message}` } })
    }
  },

  repair: async (action, adapterName) => {
    const { nodeAdminUrl, activeSessionId } = get()
    const labels: Record<string, string> = {
      flush_dns: '清 DNS 缓存',
      reset_winhttp_proxy: '重置 WinHTTP 代理',
      clear_user_proxy: '关闭当前用户代理',
      restart_adapter: '重启指定网卡',
    }
    set({ result: { kind: '', text: `正在执行：${labels[action] ?? action}…` } })
    try {
      let sessionId = activeSessionId
      if (!sessionId) {
        const d = await localJson<{ session?: Record<string, unknown>; sessions?: DoctorSessionSummary[] }>(
          nodeAdminUrl, '/api/doctor/sessions',
          { method: 'POST', body: JSON.stringify({ title: labels[action] ?? action }) },
        )
        if (d.sessions) set({ sessions: d.sessions })
        sessionId = String(d.session?.id ?? '')
        set({ activeSessionId: sessionId })
      }
      const data = await localJson<{ title?: string; outcome?: Record<string, string>; sessions?: DoctorSessionSummary[] }>(
        nodeAdminUrl, '/api/doctor/repair',
        { method: 'POST', body: JSON.stringify({ action, confirm: true, adapterName: adapterName || null, sessionId }) },
      )
      if (data.sessions) set({ sessions: data.sessions })
      const outcome = data.outcome ?? {}
      const detail = [outcome.stdout && `stdout:\n${outcome.stdout}`, outcome.stderr && `stderr:\n${outcome.stderr}`].filter(Boolean).join('\n\n') || '完成'
      set({ result: { kind: 'ok', text: `${data.title ?? labels[action] ?? action} 已执行。\n\n${detail}` } })
      if (sessionId) await get().loadSession(sessionId)
    } catch (err) {
      set({ result: { kind: 'err', text: `修复失败：${(err as Error).message}` } })
    }
  },

  loadMemory: async () => {
    const { nodeAdminUrl } = get()
    try {
      const data = await localJson<{ items?: MemoryItem[] }>(nodeAdminUrl, '/api/doctor/memory')
      set({ memories: data.items ?? [] })
    } catch {
      set({ memories: [] })
    }
  },

  saveMemory: async () => {
    const { nodeAdminUrl, problem, analysis, activeSessionId } = get()
    if (!problem || !analysis) {
      set({ result: { kind: 'err', text: '需要先描述问题并完成 AI 分析，才能保存为问题记忆。' } })
      return
    }
    try {
      let sessionId = activeSessionId
      if (!sessionId) {
        const d = await localJson<{ session?: Record<string, unknown>; sessions?: DoctorSessionSummary[] }>(
          nodeAdminUrl, '/api/doctor/sessions',
          { method: 'POST', body: JSON.stringify({ title: problem }) },
        )
        if (d.sessions) set({ sessions: d.sessions })
        sessionId = String(d.session?.id ?? '')
        set({ activeSessionId: sessionId })
      }
      await localJson(nodeAdminUrl, '/api/doctor/memory', {
        method: 'POST',
        body: JSON.stringify({ problem, summary: analysis, sessionId }),
      })
      set({ result: { kind: 'ok', text: '已保存为电脑问题记忆。' } })
      await get().loadMemory()
    } catch (err) {
      set({ result: { kind: 'err', text: `保存失败：${(err as Error).message}` } })
    }
  },
}))
