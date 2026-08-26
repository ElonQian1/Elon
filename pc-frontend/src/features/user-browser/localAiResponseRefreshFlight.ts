export type LocalAiResponseRefreshFlightClaim = 'run' | 'queued' | 'stale'
export type LocalAiResponseRefreshFlightSettlement = 'idle' | 'rerun' | 'stale'

/**
 * Keeps one official-page snapshot request active per response generation.
 * Repeated watchdog ticks are coalesced into at most one immediate rerun.
 */
export class LocalAiResponseRefreshFlight {
  private generation = 0
  private activeGeneration = -1
  private queuedGeneration = -1

  reset(): number {
    this.generation += 1
    this.activeGeneration = -1
    this.queuedGeneration = -1
    return this.generation
  }

  currentGeneration(): number {
    return this.generation
  }

  claim(generation: number): LocalAiResponseRefreshFlightClaim {
    if (generation !== this.generation) return 'stale'
    if (this.activeGeneration === generation) {
      this.queuedGeneration = generation
      return 'queued'
    }
    this.activeGeneration = generation
    return 'run'
  }

  settle(generation: number): LocalAiResponseRefreshFlightSettlement {
    if (generation !== this.generation || this.activeGeneration !== generation) return 'stale'
    this.activeGeneration = -1
    if (this.queuedGeneration === generation) {
      this.queuedGeneration = -1
      return 'rerun'
    }
    return 'idle'
  }
}
