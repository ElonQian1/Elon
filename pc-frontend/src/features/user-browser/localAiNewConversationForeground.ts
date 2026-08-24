import { requestReturnToAiChat } from './internalBrowserApi'
import { controlLocalAiWebSession, type LocalAiWebSessionState } from './localAiBrowserApi'

interface LocalAiProviderIdentity {
  id: string
  displayName: string
}

export function requestLocalAiNewConversationNativeForeground(
  provider: LocalAiProviderIdentity,
  ownerKey: string,
) {
  requestReturnToAiChat({
    providerId: provider.id,
    providerName: provider.displayName,
    ownerKey,
  })
}

export async function keepLocalAiNewConversationInNativeForeground(
  provider: LocalAiProviderIdentity,
  ownerKey: string,
  fallback: LocalAiWebSessionState,
): Promise<LocalAiWebSessionState> {
  requestLocalAiNewConversationNativeForeground(provider, ownerKey)
  try {
    return await controlLocalAiWebSession(provider.id, ownerKey, 'background')
  } catch {
    // The return-to-chat event already owns the foreground. Polling and the page
    // load callback will retry parking without turning a successful new chat into
    // a false failure solely because a late WebView2 focus raced this command.
    return fallback
  }
}
