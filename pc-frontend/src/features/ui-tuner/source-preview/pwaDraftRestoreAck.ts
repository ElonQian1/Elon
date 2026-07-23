export interface PwaDraftUnresolvedEntry {
  index: number
  selector: string
  identityKey: string
  reason: string
}

export interface PwaDraftAppliedAck {
  requestedCount: number
  appliedCount: number
  unresolved: PwaDraftUnresolvedEntry[]
  complete: boolean
  draftKey: string
  revision: number
  attempt?: number
  maxAttempts?: number
  retrying?: boolean
  exhausted?: boolean
}

export interface PwaDraftRestoreState {
  draftKey: string
  revision: number
  requestedCount: number
  appliedCount: number
  unresolved: PwaDraftUnresolvedEntry[]
  phase: 'restoring' | 'pending' | 'failed' | 'complete'
  attempt: number
  maxAttempts: number
  signature: string
}

function safeCount(value: unknown): number | null {
  const count = Number(value)
  return Number.isInteger(count) && count >= 0 ? count : null
}

function normalizedUnresolved(value: unknown): PwaDraftUnresolvedEntry[] {
  if (!Array.isArray(value)) return []
  return value.map((entry, index) => {
    const item = entry && typeof entry === 'object' ? entry as Partial<PwaDraftUnresolvedEntry> : {}
    return {
      index: safeCount(item.index) ?? index,
      selector: String(item.selector || ''),
      identityKey: String(item.identityKey || ''),
      reason: String(item.reason || 'unknown'),
    }
  })
}

export function beginPwaDraftRestore(
  draftKey: string,
  revision: number,
  requestedCount: number,
): PwaDraftRestoreState {
  return {
    draftKey,
    revision,
    requestedCount,
    appliedCount: 0,
    unresolved: [],
    phase: 'restoring',
    attempt: 0,
    maxAttempts: 0,
    signature: `restoring:${draftKey}@${revision}:${requestedCount}`,
  }
}

export function consumePwaDraftAppliedAck(
  current: PwaDraftRestoreState,
  ack: Partial<PwaDraftAppliedAck>,
): PwaDraftRestoreState {
  if (String(ack.draftKey || '') !== current.draftKey || Number(ack.revision) !== current.revision) return current
  const requestedCount = safeCount(ack.requestedCount)
  const appliedCount = safeCount(ack.appliedCount)
  let unresolved = normalizedUnresolved(ack.unresolved)
  const invalidCounts = requestedCount !== current.requestedCount
    || appliedCount === null
    || (appliedCount ?? 0) > current.requestedCount
    || (appliedCount ?? 0) + unresolved.length !== current.requestedCount
  if (invalidCounts) {
    unresolved = [{ index: -1, selector: '', identityKey: '', reason: 'protocol-mismatch' }]
  }
  const exactAppliedCount = invalidCounts ? 0 : appliedCount as number
  const complete = !invalidCounts
    && ack.complete === true
    && exactAppliedCount === current.requestedCount
    && unresolved.length === 0
  const terminalIdentityFailure = unresolved.some((entry) => (
    entry.reason === 'identity-mismatch' || entry.reason === 'identity-insufficient'
  ))
  const failed = !complete && (invalidCounts || terminalIdentityFailure || ack.exhausted === true)
  const attempt = safeCount(ack.attempt) ?? current.attempt
  const maxAttempts = safeCount(ack.maxAttempts) ?? current.maxAttempts
  const signature = JSON.stringify({
    requestedCount: current.requestedCount,
    appliedCount: exactAppliedCount,
    unresolved,
    complete,
    failed,
    attempt,
    maxAttempts,
  })
  if (signature === current.signature) return current
  return {
    ...current,
    appliedCount: exactAppliedCount,
    unresolved,
    phase: complete ? 'complete' : failed ? 'failed' : 'pending',
    attempt,
    maxAttempts,
    signature,
  }
}

export function pwaDraftRestoreLabel(state: PwaDraftRestoreState): string {
  const progress = `${state.appliedCount}/${state.requestedCount}`
  if (state.phase === 'complete') return `已恢复本页草稿 · r${state.revision}`
  if (state.phase === 'restoring') return `正在恢复本页草稿 · ${progress} · r${state.revision}`
  const mismatchCount = state.unresolved.filter((entry) => entry.reason === 'identity-mismatch').length
  if (mismatchCount) return `草稿恢复失败 · ${progress} · r${state.revision}：${mismatchCount} 个目标身份不匹配，已拒绝修改`
  const insufficientCount = state.unresolved.filter((entry) => entry.reason === 'identity-insufficient').length
  if (insufficientCount) return `草稿恢复失败 · ${progress} · r${state.revision}：${insufficientCount} 个旧草稿目标缺少安全身份，已拒绝修改`
  if (state.unresolved.some((entry) => entry.reason === 'protocol-mismatch')) {
    return `草稿恢复失败 · r${state.revision}：iframe 回执计数不一致`
  }
  const missingCount = state.unresolved.filter((entry) => entry.reason === 'target-missing').length
  if (state.phase === 'failed') return `草稿恢复未完成 · ${progress} · r${state.revision}：${missingCount || state.unresolved.length} 个目标未解析`
  return `草稿恢复待处理 · ${progress} · r${state.revision}：等待 ${missingCount || state.unresolved.length} 个目标出现`
}
