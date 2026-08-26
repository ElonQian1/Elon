import type { LocalAiMessageSnapshot, LocalAiVisibleMessage } from './localAiBrowserApi'

export type LocalAiPrivateStreamState = 'streaming' | 'completed' | null

export function localAiPrivateStreamState(
  snapshot: LocalAiMessageSnapshot | null | undefined,
): LocalAiPrivateStreamState {
  if (snapshot?.privateStreamObserved !== true) return null
  if (snapshot.privateStreamState === 'streaming') return 'streaming'
  if (snapshot.privateStreamState === 'completed') return 'completed'
  return null
}

export function localAiSnapshotIsStreaming(
  snapshot: LocalAiMessageSnapshot | null | undefined,
  assistant?: LocalAiVisibleMessage,
): boolean {
  const privateState = localAiPrivateStreamState(snapshot)
  if (privateState === 'streaming') return true
  if (privateState === 'completed') return false
  return snapshot?.streaming === true || assistant?.state === 'streaming'
}
