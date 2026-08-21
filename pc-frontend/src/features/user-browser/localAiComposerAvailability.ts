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
  const direct = enabled && input.directSendReady && !input.busyAction
  return {
    canEdit: enabled,
    canSubmit: direct || canQueue,
    shouldQueue: canQueue && !direct,
  }
}
