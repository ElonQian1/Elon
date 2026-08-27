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
  onCachedState?: (state: LocalAiWebSessionState) => void,
): Promise<LocalAiSessionResumeResult> {
  let latestState = cachedState
  try {
    // The native state getter creates the runtime record and loads its durable
    // semantic snapshot without creating or navigating WebView2. Publish that
    // snapshot first so startup is cache-first instead of website-first.
    latestState = await getLocalAiWebSessionState(providerId, ownerKey)
    onCachedState?.(latestState)
    if (localAiWarmSessionReusable(latestState, providerId)) {
      return { state: latestState, reused: true }
    }
  } catch {
    // A missing/old native state falls through to the normal background opener.
  }

  await openLocalAiWebSession(providerId, ownerKey, { showWindow: false })
  try {
    return {
      state: await getLocalAiWebSessionState(providerId, ownerKey),
      reused: false,
    }
  } catch {
    // Session polling will obtain the state without issuing another open command.
    return { state: latestState, reused: false }
  }
}
