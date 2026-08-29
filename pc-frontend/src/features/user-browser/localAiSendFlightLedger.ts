export interface LocalAiSendFlightClaim {
  sessionIdentity: string
  sendId: string
  generation: number
}

/**
 * Owns one native send from optimistic insertion through official-response evidence.
 *
 * React state is intentionally not the lock: two click handlers can run before a
 * render commits, and an old async receipt can arrive after a session or
 * conversation boundary. The synchronous ledger mirrors the Android send
 * coordinator's single-owner generation contract.
 */
export class LocalAiSendFlightLedger {
  private generation = 0
  private active: LocalAiSendFlightClaim | null = null

  begin(sessionIdentity: string, sendId: string): LocalAiSendFlightClaim | null {
    const identity = sessionIdentity.trim()
    const id = sendId.trim()
    if (!identity || !id || this.active) return null
    const claim = {
      sessionIdentity: identity,
      sendId: id,
      generation: ++this.generation,
    }
    this.active = claim
    return { ...claim }
  }

  current(sessionIdentity: string, sendId: string): LocalAiSendFlightClaim | null {
    const claim = this.active
    return claim
      && claim.sessionIdentity === sessionIdentity
      && claim.sendId === sendId
      ? { ...claim }
      : null
  }

  activeClaim(): LocalAiSendFlightClaim | null {
    return this.active ? { ...this.active } : null
  }

  isCurrent(claim: LocalAiSendFlightClaim): boolean {
    return Boolean(this.active
      && this.active.generation === claim.generation
      && this.active.sessionIdentity === claim.sessionIdentity
      && this.active.sendId === claim.sendId)
  }

  isGenerationCurrent(claim: LocalAiSendFlightClaim): boolean {
    return claim.generation === this.generation
  }

  settle(sessionIdentity: string, sendId: string): boolean {
    const claim = this.active
    if (!claim
      || claim.sessionIdentity !== sessionIdentity
      || claim.sendId !== sendId) return false
    this.active = null
    return true
  }

  invalidate(): number {
    this.generation += 1
    this.active = null
    return this.generation
  }
}
