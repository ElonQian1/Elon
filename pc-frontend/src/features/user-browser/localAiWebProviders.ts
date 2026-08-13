import type { LocalAiWebProvider } from './localAiBrowserApi'

export const LOCAL_AI_PROVIDER_FALLBACKS: Record<string, LocalAiWebProvider> = {
  'google-ai-mode': {
    id: 'google-ai-mode',
    displayName: 'Google AI 模式',
    startHost: 'google.com/aimode',
    loginMode: 'guest_web_system_login',
    profileScope: 'local_owner_provider',
    rendererProtocol: 'yilong.ai.ui.v1',
    rendererStatus: 'active',
  },
  chatgpt: {
    id: 'chatgpt',
    displayName: 'ChatGPT',
    startHost: 'chatgpt.com',
    loginMode: 'manual_web',
    profileScope: 'local_owner_provider',
    rendererProtocol: 'yilong.ai.ui.v1',
    rendererStatus: 'active',
  },
}

export const DEFAULT_LOCAL_AI_PROVIDER_ID = 'chatgpt'
