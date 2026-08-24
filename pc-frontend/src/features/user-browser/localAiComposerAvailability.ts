export interface LocalAiComposerAvailabilityInput {
  clientReady: boolean
  providerAvailable: boolean
  sendSupported: boolean
  directSendReady: boolean
  newConversationRecoveryActive: boolean
  queuedSendActive: boolean
  busyAction: string
}

export interface LocalAiComposerAvailability {
  canEdit: boolean
  canSubmit: boolean
  shouldQueue: boolean
}

export function localAiComposerAvailability(
  input: LocalAiComposerAvailabilityInput,
): LocalAiComposerAvailability {
  const enabled = input.clientReady && input.providerAvailable && input.sendSupported
  const transitionBusy = input.busyAction === 'new_conversation'
  const canQueue = enabled
    && input.newConversationRecoveryActive
    && !input.queuedSendActive
    && (!input.busyAction || transitionBusy)
  // A provider can briefly keep reporting the previous conversation's composer as
  // ready while a native new-chat boundary is still waiting for the official page
  // to bind its replacement conversation. Treating that stale readiness as a direct
  // send races the first prompt into the old page and can bring its WebView to the
  // foreground. During recovery every first prompt must use the existing bounded
  // queue, even when the previous composer still looks healthy.
  const direct = enabled
    && input.directSendReady
    && !input.newConversationRecoveryActive
    && !input.busyAction
  return {
    canEdit: enabled,
    canSubmit: direct || canQueue,
    shouldQueue: canQueue && !direct,
  }
}
