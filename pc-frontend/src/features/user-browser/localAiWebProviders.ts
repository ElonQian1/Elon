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
    adapterActions: ['snapshot', 'send_prompt', 'stop_generation', 'new_conversation'],
  },
  chatgpt: {
    id: 'chatgpt',
    displayName: 'ChatGPT',
    startHost: 'chatgpt.com',
    loginMode: 'manual_web',
    profileScope: 'local_owner_provider',
    rendererProtocol: 'yilong.ai.ui.v1',
    rendererStatus: 'active',
    adapterActions: [
      'snapshot',
      'send_prompt',
      'stop_generation',
      'regenerate_response',
      'new_conversation',
      'list_conversations',
      'open_conversation',
      'open_project',
      'start_google_login',
      'list_model_options',
      'list_composer_tools',
      'collect_model_options',
      'collect_composer_tools',
      'select_model_option',
      'select_composer_tool',
    ],
  },
}

export const DEFAULT_LOCAL_AI_PROVIDER_ID = 'chatgpt'
