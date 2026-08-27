export interface LocalAiComposerAvailabilityInput {
  clientReady: boolean
  providerAvailable: boolean
  sendSupported: boolean
  directSendReady: boolean
  newConversationRecoveryActive: boolean
  sessionResumeRecoveryActive: boolean
  queuedSendActive: boolean
  sendFlightActive: boolean
  busyAction: string
}

export interface LocalAiComposerAvailability {
  canEdit: boolean
  canSubmit: boolean
  shouldQueue: boolean
  queueReason: 'new_conversation' | 'session_resume' | null
}

export function localAiComposerAvailability(
  input: LocalAiComposerAvailabilityInput,
): LocalAiComposerAvailability {
  const enabled = input.clientReady && input.providerAvailable && input.sendSupported
  const transitionBusy = input.busyAction === 'new_conversation'
  const canQueueNewConversation = enabled
    && input.newConversationRecoveryActive
    && !input.queuedSendActive
    && (!input.busyAction || transitionBusy)
  const canQueueSessionResume = enabled
    && input.sessionResumeRecoveryActive
    && !input.newConversationRecoveryActive
    && !input.queuedSendActive
    && !input.busyAction
  const queueReason = canQueueNewConversation
    ? 'new_conversation'
    : canQueueSessionResume
      ? 'session_resume'
      : null
  // A provider can briefly keep reporting the previous conversation's composer as
  // ready while a native new-chat boundary is still waiting for the official page
  // to bind its replacement conversation. Treating that stale readiness as a direct
  // send races the first prompt into the old page and can bring its WebView to the
  // foreground. During recovery every first prompt must use the existing bounded
  // queue, even when the previous composer still looks healthy.
  const direct = enabled
    && input.directSendReady
    && !input.newConversationRecoveryActive
    && !input.sessionResumeRecoveryActive
    && !input.sendFlightActive
    && !input.busyAction
  return {
    canEdit: enabled,
    canSubmit: direct || Boolean(queueReason),
    shouldQueue: Boolean(queueReason) && !direct,
    queueReason,
  }
}
