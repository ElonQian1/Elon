import {
  createPwaDesignDraft,
  readPwaDesignDraft,
  removePwaDesignDraft,
  savePwaDesignDraft,
  type PwaDesignDraft,
  type PwaRouteIdentity,
} from './pwaDesignDraft'

export const PWA_DESIGN_HISTORY_LIMIT = 60
export const PWA_DESIGN_TRANSACTION_IDLE_MS = 450

export interface PwaDesignDraftRestore {
  draft: PwaDesignDraft
  restored: boolean
}

type DraftElementsUpdater = (elements: PwaDesignDraft['elements']) => PwaDesignDraft['elements']

function revisedDraft(draft: PwaDesignDraft, elements: PwaDesignDraft['elements']): PwaDesignDraft {
  return {
    ...draft,
    elements,
    revision: draft.revision + 1,
    updatedAt: new Date().toISOString(),
  }
}

function persistDraft(draft: PwaDesignDraft): void {
  if (Object.keys(draft.elements).length) savePwaDesignDraft(draft)
  else removePwaDesignDraft(draft)
}

export class PwaDesignSessionModel {
  private currentDraft: PwaDesignDraft | null = null
  private pastDrafts: PwaDesignDraft[] = []
  private futureDrafts: PwaDesignDraft[] = []
  private transaction: { key: string; timer: number } | null = null

  get draft(): PwaDesignDraft | null {
    return this.currentDraft
  }

  get canUndo(): boolean {
    return this.pastDrafts.length > 0
  }

  get canRedo(): boolean {
    return this.futureDrafts.length > 0
  }

  restore(project: PwaDesignDraft['project'], route: PwaRouteIdentity): PwaDesignDraftRestore {
    this.closeTransaction()
    const persisted = readPwaDesignDraft(project, route)
    const draft = persisted ?? createPwaDesignDraft(project, route)
    this.currentDraft = draft
    this.pastDrafts = []
    this.futureDrafts = []
    return { draft, restored: persisted !== null }
  }

  update(transactionKey: string, updateElements: DraftElementsUpdater): PwaDesignDraft | null {
    const current = this.currentDraft
    if (!current) return null
    this.beginTransaction(transactionKey, current)
    return this.replace(revisedDraft(current, updateElements(current.elements)))
  }

  replace(draft: PwaDesignDraft, persist = true): PwaDesignDraft {
    this.currentDraft = draft
    if (persist) persistDraft(draft)
    return draft
  }

  undo(): PwaDesignDraft | null {
    this.closeTransaction()
    const previous = this.pastDrafts.pop()
    const current = this.currentDraft
    if (!previous || !current) return null
    this.futureDrafts = [...this.futureDrafts, current].slice(-PWA_DESIGN_HISTORY_LIMIT)
    return this.replace(previous)
  }

  redo(): PwaDesignDraft | null {
    this.closeTransaction()
    const next = this.futureDrafts.pop()
    const current = this.currentDraft
    if (!next || !current) return null
    this.pastDrafts = [...this.pastDrafts, current].slice(-PWA_DESIGN_HISTORY_LIMIT)
    return this.replace(next)
  }

  save(): PwaDesignDraft | null {
    const current = this.currentDraft
    if (current) persistDraft(current)
    return current
  }

  closeTransaction(): void {
    if (this.transaction) window.clearTimeout(this.transaction.timer)
    this.transaction = null
  }

  dispose(): void {
    this.closeTransaction()
  }

  private beginTransaction(key: string, current: PwaDesignDraft): void {
    const active = this.transaction
    if (!active || active.key !== key) {
      this.closeTransaction()
      this.pastDrafts = [...this.pastDrafts, current].slice(-PWA_DESIGN_HISTORY_LIMIT)
      this.futureDrafts = []
    } else {
      window.clearTimeout(active.timer)
    }
    this.transaction = {
      key,
      timer: window.setTimeout(() => { this.transaction = null }, PWA_DESIGN_TRANSACTION_IDLE_MS),
    }
  }
}
