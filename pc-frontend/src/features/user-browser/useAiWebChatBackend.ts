import { useEffect, useMemo, useState } from 'react'
import type { AiMessage, AiSource } from '../ai/AiChatMessageRow'
import type { AiHomeMode } from '../ai/AiHomeModeSwitch'
import {
  DEFAULT_LOCAL_AI_PROVIDER_ID,
  LOCAL_AI_PROVIDER_FALLBACKS,
} from './localAiWebProviders'
import useLocalAiBrowserCapability from './useLocalAiBrowserCapability'
import useLocalAiWebChatController from './useLocalAiWebChatController'

const PROVIDER_STORAGE_KEY = 'elon.pc.aiChatProvider'

export default function useAiWebChatBackend(mode: AiHomeMode, ownerKey: string) {
  const capability = useLocalAiBrowserCapability()
  const [providerId, setProviderId] = useState(readProviderPreference)
  const providers = useMemo(() => (
    capability.providers.length
      ? capability.providers
      : Object.values(LOCAL_AI_PROVIDER_FALLBACKS)
  ), [capability.providers])
  const provider = providers.find((item) => item.id === providerId) || providers[0]
  const controller = useLocalAiWebChatController(
    mode === 'chat' && capability.state === 'ready' ? provider : undefined,
    mode === 'chat' ? ownerKey : '',
    capability.state,
  )

  useEffect(() => {
    if (!provider || provider.id === providerId) return
    setProviderId(provider.id)
  }, [provider, providerId])

  const messages = useMemo<AiMessage[]>(() => (
    controller.snapshot?.messages.map((item) => {
      const sources = item.content
        .filter((part): part is Extract<typeof part, { type: 'citation' }> => part.type === 'citation')
        .map<AiSource>((part) => ({ title: part.title || publicHost(part.url), url: part.url }))
      const content = item.content
        .filter((part): part is Extract<typeof part, { type: 'text' }> => part.type === 'text')
        .map((part) => part.text)
        .filter(Boolean)
        .join('\n\n')
      return {
        id: `web:${provider?.id || 'ai'}:${item.id}`,
        role: item.role,
        content: content || (sources.length ? '相关来源' : '官方网页暂未返回可见文本。'),
        tool_used: item.role === 'assistant' && provider?.id === 'google-ai-mode' ? 'web_search' : null,
        sources,
      }
    }) ?? []
  ), [controller.snapshot?.messages, provider?.id])
  const ready = capability.state === 'ready' && Boolean(ownerKey && provider)
  const canCompose = ready && controller.userState.canSend
  const streamingMessageId = [...(controller.snapshot?.messages ?? [])]
    .reverse()
    .find((item) => item.state === 'streaming')?.id

  function selectProvider(id: string) {
    setProviderId(id)
    try { window.localStorage.setItem(PROVIDER_STORAGE_KEY, id) } catch {}
  }

  return {
    capability,
    providers,
    provider,
    selectProvider,
    controller,
    userState: controller.userState,
    messages,
    ready,
    canCompose,
    title: controller.snapshot?.title || '新对话',
    streamingMessageId: streamingMessageId ? `web:${provider?.id || 'ai'}:${streamingMessageId}` : null,
    accessMode: canCompose
      ? controller.snapshot?.authenticated ? 'account' as const : 'guest' as const
      : 'unavailable' as const,
    status: controller.userState.title,
    message: controller.sessionState?.lastError
      || controller.message
      || capability.message
      || (controller.userState.degraded ? controller.userState.detail : ''),
    modelButtonCopy: {
      source: '网页 AI',
      detail: provider?.displayName || '选择厂商',
      title: `聊天来源：${provider?.displayName || '网页 AI'}；点击切换或管理官方会话`,
    },
  }
}

export type AiWebChatBackend = ReturnType<typeof useAiWebChatBackend>

function readProviderPreference() {
  try {
    const stored = window.localStorage.getItem(PROVIDER_STORAGE_KEY) || ''
    if (stored in LOCAL_AI_PROVIDER_FALLBACKS) return stored
  } catch {}
  return DEFAULT_LOCAL_AI_PROVIDER_ID
}

function publicHost(url: string) {
  try { return new URL(url).hostname }
  catch { return url }
}
