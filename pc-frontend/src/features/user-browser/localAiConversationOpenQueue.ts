export interface LocalAiDeferredConversationOpen {
  conversationId: string
  sessionIdentity: string
}

/** Latest-selection-wins queue for a single provider/profile session. */
export class LocalAiConversationOpenQueue {
  private pending: LocalAiDeferredConversationOpen | null = null
  private draining = false

  constructor(private readonly sessionIdentity: string) {}

  enqueue(conversationId: string): boolean {
    this.pending = { conversationId, sessionIdentity: this.sessionIdentity }
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
