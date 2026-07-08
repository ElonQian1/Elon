import { api } from '../../api/client'

export const PROJECT_PREWARM_COOLDOWN_MS = 120000

const PROJECT_PREWARM_STORAGE_KEY = 'elon_project_prewarm_enabled'
const PROJECT_PREWARM_DEFAULT_VERSION_KEY = 'elon_project_prewarm_default_version'
const PROJECT_PREWARM_DEFAULT_VERSION = 'pc-open-prewarm-default-on-20260708'

interface ProjectPrewarmPayload {
  conversation_id: string
  conversation_title?: string | null
  agent?: string
  trace_id?: string
}

export function initialProjectPrewarmFromStorage(storage: Storage | null | undefined): boolean {
  if (!storage) return true
  try {
    const version = storage.getItem(PROJECT_PREWARM_DEFAULT_VERSION_KEY)
    if (version !== PROJECT_PREWARM_DEFAULT_VERSION) return true
    return storage.getItem(PROJECT_PREWARM_STORAGE_KEY) !== 'false'
  } catch {
    return true
  }
}

export function persistProjectPrewarmSelection(
  storage: Storage | null | undefined,
  enabled: boolean,
): void {
  if (!storage) return
  try {
    storage.setItem(PROJECT_PREWARM_STORAGE_KEY, enabled ? 'true' : 'false')
    storage.setItem(PROJECT_PREWARM_DEFAULT_VERSION_KEY, PROJECT_PREWARM_DEFAULT_VERSION)
  } catch {
    // Ignore storage failures; prewarm is best effort.
  }
}

export function requestProjectPrewarm(
  projectId: string,
  payload: ProjectPrewarmPayload,
): Promise<unknown> {
  return api.post(`/api/projects/${encodeURIComponent(projectId)}/prewarm`, payload)
}
