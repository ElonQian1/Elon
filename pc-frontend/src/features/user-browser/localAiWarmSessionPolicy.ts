import type { LocalAiWebSessionState } from './localAiBrowserApi'

export function localAiWarmSessionReusable(
  state: LocalAiWebSessionState | null,
  providerId: string,
): boolean {
  return Boolean(
    state
      && state.providerId === providerId
      && state.windowStatus !== 'closed'
      && state.windowStatus !== 'blocked'
      && state.windowStatus !== 'error'
      && !state.lastError,
  )
}
