import { useEffect } from 'react'
import { getDesktopInvoke } from '../shell/desktopShell'
import { claimWinAction, fetchPendingWinActions, postWinActionReceipt, postWinEvent } from './codexControlApi'
import type { WinControlAction } from './types'

const HEARTBEAT_MS = 8_000
const ACTION_POLL_MS = 2_500
const MAX_PROCESSED_ACTIONS = 240

interface NativeEventPayload {
  events?: Array<{
    seq: number
    trace_id?: string
    level?: string
    kind?: string
    summary?: string
    fields?: Record<string, unknown>
  }>
  next_after?: number
}

interface NativeReceipt {
  status?: string
  message?: string
  route?: string
  at_ms?: number
}

let diagnosticsInstalled = false

export function useCodexControlBridge(): void {
  useEffect(() => {
    installBrowserDiagnostics()
    let disposed = false
    let nativeCursor = 0
    const processed = new Set<string>()

    async function heartbeat() {
      await postWinEvent({
        trace_id: 'pc_frontend', source: 'frontend', level: 'debug',
        kind: 'bridge.heartbeat', summary: 'PC 前端诊断桥在线',
        fields: { route: location.pathname, title: document.title },
      }).catch(() => {})
    }

    async function pollNativeBridge() {
      const invoke = getDesktopInvoke()
      if (!invoke || disposed) return
      try {
        const capabilities = await invoke<Record<string, unknown>>('codex_win_capabilities')
        await postWinEvent({
          trace_id: 'tauri_heartbeat', source: 'tauri', level: 'debug',
          kind: 'bridge.heartbeat', summary: 'Tauri 语义桥在线',
          fields: { schema: capabilities.schema, devtools_supported: capabilities.devtools_supported },
        })
        const payload = await invoke<NativeEventPayload>('codex_read_native_events', {
          after: nativeCursor, limit: 100,
        })
        for (const event of payload.events ?? []) {
          await postWinEvent({
            trace_id: event.trace_id, source: 'tauri', level: event.level,
            kind: event.kind || 'native.event', summary: event.summary || 'Tauri 原生事件',
            fields: { native_seq: event.seq, ...(event.fields ?? {}) },
          })
        }
        nativeCursor = Number(payload.next_after ?? nativeCursor)
      } catch (error) {
        await postWinEvent({
          trace_id: 'tauri_bridge', source: 'tauri', level: 'error',
          kind: 'bridge.error', summary: safeMessage(error),
        }).catch(() => {})
      }
    }

    async function pollActions() {
      const invoke = getDesktopInvoke()
      if (!invoke || disposed) return
      const actions = await fetchPendingWinActions().catch(() => [])
      for (const action of actions) {
        if (processed.has(action.action_id)) continue
        const claimed = await claimWinAction(action.action_id).catch(() => null)
        if (!claimed) continue
        remember(processed, action.action_id)
        await executeAction(invoke, claimed)
      }
    }

    void heartbeat()
    void pollNativeBridge()
    void pollActions()
    const heartbeatTimer = window.setInterval(() => { void heartbeat(); void pollNativeBridge() }, HEARTBEAT_MS)
    const actionTimer = window.setInterval(() => { void pollActions() }, ACTION_POLL_MS)
    return () => {
      disposed = true
      window.clearInterval(heartbeatTimer)
      window.clearInterval(actionTimer)
    }
  }, [])
}

async function executeAction(
  invoke: NonNullable<ReturnType<typeof getDesktopInvoke>>,
  action: WinControlAction,
): Promise<void> {
  try {
    const receipt = await invoke<NativeReceipt>('codex_execute_semantic_action', { action })
    await postWinActionReceipt(action.action_id, {
      status: receipt.status || 'succeeded',
      message: receipt.message,
      route: receipt.route,
      at_ms: receipt.at_ms || Date.now(),
    })
  } catch (error) {
    await postWinActionReceipt(action.action_id, {
      status: 'failed', message: safeMessage(error), route: location.pathname, at_ms: Date.now(),
    }).catch(() => {})
  }
}

function installBrowserDiagnostics(): void {
  if (diagnosticsInstalled) return
  diagnosticsInstalled = true
  window.addEventListener('error', (event) => {
    void postWinEvent({
      trace_id: 'frontend_error', source: 'frontend', level: 'error', kind: 'window.error',
      summary: safeMessage(event.error || event.message),
      fields: { filename: safePath(event.filename), line: event.lineno, column: event.colno },
    }).catch(() => {})
  })
  window.addEventListener('unhandledrejection', (event) => {
    void postWinEvent({
      trace_id: 'frontend_rejection', source: 'frontend', level: 'error',
      kind: 'promise.unhandled', summary: safeMessage(event.reason),
    }).catch(() => {})
  })
  const originalFetch = window.fetch.bind(window)
  window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
    const started = performance.now()
    const method = String(init?.method || (input instanceof Request ? input.method : 'GET')).toUpperCase()
    const url = safeUrl(input)
    try {
      const response = await originalFetch(input, init)
      if (!isControlEventUrl(url)) {
        void postWinEvent({
          trace_id: response.headers.get('x-trace-id') || 'network', source: 'network',
          level: response.ok ? 'info' : 'warn', kind: 'request.completed',
          summary: `${method} ${url.path} → ${response.status}`,
          fields: { method, path: url.path, origin: url.origin, status: response.status, duration_ms: Math.round(performance.now() - started) },
        }).catch(() => {})
      }
      return response
    } catch (error) {
      if (!isControlEventUrl(url)) {
        void postWinEvent({
          trace_id: 'network_error', source: 'network', level: 'error', kind: 'request.failed',
          summary: `${method} ${url.path} 请求失败`,
          fields: { method, path: url.path, origin: url.origin, duration_ms: Math.round(performance.now() - started), error: safeMessage(error) },
        }).catch(() => {})
      }
      throw error
    }
  }
}

function safeUrl(input: RequestInfo | URL): { path: string; origin: string } {
  try {
    const raw = input instanceof Request ? input.url : String(input)
    const url = new URL(raw, location.href)
    return { path: url.pathname.slice(0, 240), origin: url.origin === location.origin ? 'same-origin' : url.origin.slice(0, 120) }
  } catch {
    return { path: '[invalid-url]', origin: 'unknown' }
  }
}

function isControlEventUrl(url: { path: string }): boolean {
  return url.path === '/api/codex-control/events'
}

function safePath(value: string): string {
  try { return value ? new URL(value, location.href).pathname.slice(0, 240) : '' } catch { return '' }
}

function safeMessage(value: unknown): string {
  const message = value instanceof Error ? value.message : String(value ?? '未知错误')
  return message.slice(0, 500)
}

function remember(set: Set<string>, actionId: string): void {
  set.add(actionId)
  if (set.size > MAX_PROCESSED_ACTIONS) {
    const oldest = set.values().next().value
    if (oldest) set.delete(oldest)
  }
}
