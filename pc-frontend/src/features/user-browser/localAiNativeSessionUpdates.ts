import { getDesktopEventListen, type DesktopUnlisten } from '../shell/desktopShell'

export const LOCAL_AI_SESSION_UPDATED_EVENT = 'elon:local-ai-session-updated'
export const LOCAL_AI_NATIVE_UPDATE_COALESCE_MS = 120

export interface LocalAiNativeSessionUpdate {
  providerId: string
  windowLabel: string
  kind: string
}

export function localAiNativeSessionUpdateMatches(
  update: LocalAiNativeSessionUpdate,
  providerId: string,
  windowLabel: string | undefined,
): boolean {
  return Boolean(
    providerId
      && windowLabel
      && update.providerId === providerId
      && update.windowLabel === windowLabel,
  )
}

export async function listenLocalAiNativeSessionUpdates(
  handler: (update: LocalAiNativeSessionUpdate) => void,
): Promise<DesktopUnlisten> {
  const listen = getDesktopEventListen()
  if (!listen) return () => {}
  return listen<LocalAiNativeSessionUpdate>(LOCAL_AI_SESSION_UPDATED_EVENT, (event) => {
    const update = event.payload
    if (!update || typeof update.providerId !== 'string' || typeof update.windowLabel !== 'string') return
    handler(update)
  })
}
