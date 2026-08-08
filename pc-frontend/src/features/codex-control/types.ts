export type WinLogSource = 'frontend' | 'rust' | 'cli' | 'network' | 'tauri' | 'control'
export type WinActionKind = 'show_window' | 'focus_window' | 'navigate' | 'reload_page' | 'open_devtools' | 'close_devtools' | 'capture_state'

export interface WinControlCapabilities {
  schema: string
  actions: WinActionKind[]
  routes: string[]
  sources: WinLogSource[]
  frontend_available: boolean
  tauri_available: boolean
  security: Record<string, boolean>
  retention: { events: number; actions: number; action_ttl_ms: number }
}

export interface WinControlEvent {
  seq: number
  event_id: string
  trace_id: string
  source: WinLogSource
  level: 'debug' | 'info' | 'warn' | 'error'
  kind: string
  summary: string
  at_ms: number
  fields: Record<string, unknown>
}

export interface WinControlAction {
  action_id: string
  trace_id: string
  kind: WinActionKind
  route?: string | null
  requested_at_ms: number
  expires_at_ms: number
  status: string
}

export interface WinTimelineResponse {
  ok: boolean
  schema: string
  next_since: number
  events: WinControlEvent[]
  capabilities: WinControlCapabilities
}
