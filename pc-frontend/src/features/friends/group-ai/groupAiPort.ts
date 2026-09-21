import { getDesktopInvoke } from '../../shell/desktopShell'
import { useAuthStore } from '../../../store/auth'
import { listLocalAiWebProviders, type LocalAiWebSessionState } from '../../user-browser/localAiBrowserApi'
import { socialRequest } from '../socialChatOperations'
import type { GroupAiPort, GroupAiRequest, GroupAiInput } from './groupAiTask'

function endpoint(input: GroupAiInput) { return '/api/me/groups/' + encodeURIComponent(input.group) }
export function createGroupAiPort(owner: string): GroupAiPort {
  const checkOwner = () => {
    if (useAuthStore.getState().user?.id !== owner) throw new Error('账号已切换，已停止此账号的群聊 AI')
  }
  const post = async (path: string, payload: object) => {
    checkOwner()
    const response = await socialRequest<{ request: GroupAiRequest }>(path, {
      method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload),
    })
    checkOwner(); return response.request
  }
  return {
    checkOwner, now: Date.now, wait: ms => new Promise(resolve => setTimeout(resolve, ms)),
    prepare: (operation, input) => post(endpoint(input) + '/messages/' + encodeURIComponent(input.source) + '/web-ai', {
      operation_id: operation, selected_context: input.selection,
    }),
    action: (operation, input, request, action, content) => post(endpoint(input) + '/web-ai/requests/' + encodeURIComponent(request.id), {
      operation_id: operation, action, content,
      web_provider: input.provider === 'chatgpt' ? 'chatgpt_web' : 'google_web',
    }),
    async host(operation, input, action, value, requestId) {
      if (action !== 'close') checkOwner()
      const invoke = getDesktopInvoke()
      if (!invoke) throw new Error('请在一龙 Windows 客户端使用本人的网页 AI')
      if (action === 'open') {
        const provider = (await listLocalAiWebProviders()).find(p => p.id === input.provider)
        if (!provider || provider.desktopRuntimeVersion < 13) throw new Error('请升级 Windows 客户端后使用群聊网页 AI')
      }
      let timer: ReturnType<typeof setTimeout> | undefined
      try {
        return await Promise.race([
          invoke<LocalAiWebSessionState>('group_ai_web_session', {
            ownerKey: owner, providerId: input.provider, taskId: operation, action, value: value ?? null, requestId: requestId ?? null,
          }),
          new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('桌面 AI 会话响应超时')), 20_000) }),
        ])
      } finally { clearTimeout(timer) }
    },
  }
}
