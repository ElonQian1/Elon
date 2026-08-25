import {
  getLocalAiWebSessionState,
  openLocalAiWebSession,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import { localAiWarmSessionReusable } from './localAiWarmSessionPolicy'

export interface LocalAiSessionResumeResult {
  state: LocalAiWebSessionState | null
  reused: boolean
}

export async function resumeLocalAiWebSession(
  providerId: string,
  ownerKey: string,
  cachedState: LocalAiWebSessionState | null,
): Promise<LocalAiSessionResumeResult> {
  if (localAiWarmSessionReusable(cachedState, providerId)) {
    try {
      const current = await getLocalAiWebSessionState(providerId, ownerKey)
      if (localAiWarmSessionReusable(current, providerId)) {
        return { state: current, reused: true }
      }
    } catch {
      // A stale frontend cache falls through to the normal background opener.
    }
  }

  await openLocalAiWebSession(providerId, ownerKey, { showWindow: false })
  try {
    return {
      state: await getLocalAiWebSessionState(providerId, ownerKey),
      reused: false,
    }
  } catch {
    // Session polling will obtain the state without issuing another open command.
    return { state: null, reused: false }
  }
}
