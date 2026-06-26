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

export type DoctorSection = 'diagnosis' | 'snapshot' | 'repair' | 'memory'

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
