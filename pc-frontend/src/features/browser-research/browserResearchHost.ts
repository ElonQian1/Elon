import { record, ResearchError } from './browserResearchModel'
import type { ResearchCommand } from './types'

export interface ResearchHost {
  instance_id: string
  sessions: { project_key: string; session_id: string }[]
}
const id = (v: unknown): v is string => typeof v === 'string' && /^[A-Za-z0-9_-]{1,100}$/.test(v)
export function parseResearchHost(value: unknown): ResearchHost {
  if (!record(value) || !id(value.instance_id) || !Array.isArray(value.sessions) || value.sessions.length > 8
    || !value.sessions.every(s => record(s) && typeof s.project_key === 'string'
      && /^[a-f0-9]{64}$/.test(s.project_key) && id(s.session_id))) throw new ResearchError('unsupported')
  return value as unknown as ResearchHost
}

// Broker-only routing metadata must never leak into the native research command.
export function nativeResearchCommand(command: ResearchCommand): ResearchCommand {
  const { instance_id: _instance, ...native } = command
  return native
}
