export interface PwaBridgeHealth {
  reason: string
  ready: boolean
  mode: 'select' | 'interact'
  selected: boolean
  editablePropertyCount: number
  canApplyDraft: boolean
  canVerifySource: boolean
  draft: {
    requestedCount: number
    appliedCount: number
    unresolvedCount: number
    complete: boolean
    revision: number
    retrying: boolean
    exhausted: boolean
  } | null
  route?: unknown
}
