export interface LocalAiDeferredConversationOpen {
  conversationId: string
  sessionIdentity: string
  action?: LocalAiDeferredConversationAction
}

export type LocalAiDeferredConversationAction = 'open_conversation' | 'open_project'

/** Latest-selection-wins queue for a single provider/profile session. */
export class LocalAiConversationOpenQueue {
  private pending: LocalAiDeferredConversationOpen | null = null
  private draining = false

  constructor(private readonly sessionIdentity: string) {}

  enqueue(conversationId: string, action?: LocalAiDeferredConversationAction): boolean {
    if (this.pending?.conversationId === conversationId && this.pending.action === action) {
      return false
    }
    this.pending = { conversationId, sessionIdentity: this.sessionIdentity, ...(action ? { action } : {}) }
    if (this.draining) return false
    this.draining = true
    return true
  }

  take(): LocalAiDeferredConversationOpen | null {
    const request = this.pending
    this.pending = null
    return request
  }

  hasPending(): boolean {
    return this.pending !== null
  }

  finish(): void {
    this.draining = false
  }
}
