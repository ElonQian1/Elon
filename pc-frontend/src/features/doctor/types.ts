export interface DoctorMessage {
  id: string
  role: 'user' | 'assistant'
  content: string
  kind: string
  createdAtMs: number
  time: string
}

export interface DoctorSessionSummary {
  id: string
  title: string
  messageCount?: number
  updatedAtMs?: number
}

export interface DoctorSession {
  id: string
  title: string
  messages: DoctorMessage[]
}

export type DoctorSection = 'diagnosis' | 'snapshot' | 'router' | 'repair' | 'memory'

export interface DoctorResult {
  kind: '' | 'ok' | 'err'
  text: string
}

export interface SnapshotData {
  commands?: unknown[]
  [key: string]: unknown
}

export interface MemoryItem {
  id?: string
  problem: string
  summary: string
  createdAtMs?: number
}

export interface RepairOutcome {
  stdout?: string
  stderr?: string
  error?: string
}

export interface DownloadRouterProfile {
  enabled: boolean
  mode: 'auto' | 'direct' | 'system_proxy' | 'off' | string
  failOpen?: boolean
  cacheMinutes?: number
  updatedAt?: string
}

export interface DownloadRouterStatus {
  ok?: boolean
  routerVersion?: string
  profile?: DownloadRouterProfile
  profilePath?: string
  traceScope?: string
  wrapperPolicy?: string
  availableModes?: string[]
}

export interface DownloadRouterDoctorReport {
  ok?: boolean
  recommendation?: Record<string, unknown>
  probes?: unknown[]
  [key: string]: unknown
}
