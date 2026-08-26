import type { LocalAiMessageSnapshot, LocalAiWebSessionState } from './localAiBrowserApi'
import { localAiSnapshotIsStreaming } from './localAiPrivateStreamSignal'

export type LocalAiProviderActivityPhase = 'idle' | 'streaming' | 'completed' | 'attention'

export interface LocalAiProviderActivity {
  phase: LocalAiProviderActivityPhase
  label: string
  unread: boolean
  observedSemanticAtMs: number
  lastAssistantId: string
}

export function initializeLocalAiProviderActivity(
  state: LocalAiWebSessionState | null,
): LocalAiProviderActivity {
  const signal = providerSignal(state)
  return activityFromSignal(signal, false, false)
}

export function updateLocalAiProviderActivity(
  previous: LocalAiProviderActivity,
  state: LocalAiWebSessionState | null,
  selected: boolean,
): LocalAiProviderActivity {
  const signal = providerSignal(state)
  const assistantChanged = Boolean(
    signal.lastAssistantId && signal.lastAssistantId !== previous.lastAssistantId,
  )
  const streamCompleted = previous.phase === 'streaming'
    && !signal.streaming
    && Boolean(signal.lastAssistantId)
  const receivedBackgroundAnswer = !selected && (assistantChanged || streamCompleted)
  return activityFromSignal(
    signal,
    selected ? false : previous.unread || receivedBackgroundAnswer,
    selected,
  )
}

function activityFromSignal(
  signal: ProviderSignal,
  unread: boolean,
  selected: boolean,
): LocalAiProviderActivity {
  if (signal.attention) {
    return {
      phase: 'attention',
      label: '需要处理',
      unread: false,
      observedSemanticAtMs: signal.semanticAtMs,
      lastAssistantId: signal.lastAssistantId,
    }
  }
  if (signal.streaming) {
    return {
      phase: 'streaming',
      label: selected ? '正在回答' : '后台回答中',
      unread,
      observedSemanticAtMs: signal.semanticAtMs,
      lastAssistantId: signal.lastAssistantId,
    }
  }
  return {
    phase: unread ? 'completed' : 'idle',
    label: unread ? '新回答' : '',
    unread,
    observedSemanticAtMs: signal.semanticAtMs,
    lastAssistantId: signal.lastAssistantId,
  }
}

interface ProviderSignal {
  streaming: boolean
  attention: boolean
  semanticAtMs: number
  lastAssistantId: string
}

function providerSignal(state: LocalAiWebSessionState | null): ProviderSignal {
  const snapshot = messageSnapshot(state?.semanticEvent)
  const lastAssistant = [...(snapshot?.messages ?? [])]
    .reverse()
    .find((message) => message.role === 'assistant')
  return {
    streaming: localAiSnapshotIsStreaming(snapshot, lastAssistant),
    attention: Boolean(
      state?.lastError?.trim()
      || state?.windowStatus === 'error'
      || state?.windowStatus === 'blocked',
    ),
    semanticAtMs: Math.max(0, state?.semanticUpdatedAtMs ?? 0),
    lastAssistantId: lastAssistant?.id ?? '',
  }
}

function messageSnapshot(value: unknown): LocalAiMessageSnapshot | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const candidate = value as Partial<LocalAiMessageSnapshot>
  return candidate.type === 'message_snapshot' && Array.isArray(candidate.messages)
    ? candidate as LocalAiMessageSnapshot
    : null
}
