import {
  controlLocalAiWebSession,
  getLocalAiWebSessionState,
  localAiBrowserErrorMessage,
  openLocalAiWebResearchDirectory,
  openLocalAiWebSession,
  type LocalAiBrowserControlAction,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import { requestOfficialAiTab } from './internalBrowserApi'

interface Options {
  provider: LocalAiWebProvider | undefined
  ownerKey: string
  busyAction: string
  onBusyAction: (action: string) => void
  onMessage: (message: string) => void
  onState: (state: LocalAiWebSessionState | null) => void
}

export default function useLocalAiWindowActions({
  provider,
  ownerKey,
  busyAction,
  onBusyAction,
  onMessage,
  onState,
}: Options) {
  async function openOfficial() {
    if (!provider || !ownerKey || busyAction) return
    onBusyAction('open')
    onMessage('')
    try {
      await openLocalAiWebSession(provider.id, ownerKey, { showWindow: false })
      try {
        onState(await getLocalAiWebSessionState(provider.id, ownerKey))
      } catch {
        // The bounded poll recovers a state refresh without reopening the window.
      }
      requestOfficialAiTab({ providerId: provider.id, providerName: provider.displayName, ownerKey })
      onMessage(`已切换到 ${provider.displayName} 官方原生标签；天气、地图、图标和交互内容由官网直接显示。`)
    } catch (error) {
      onMessage(localAiBrowserErrorMessage(error))
    } finally {
      onBusyAction('')
    }
  }

  async function openResearchDirectory() {
    if (!provider || !ownerKey || busyAction) return
    onBusyAction('research-directory')
    onMessage('')
    try {
      await openLocalAiWebResearchDirectory(provider.id, ownerKey)
      onMessage('已打开当前厂商的本机原始响应研究目录。')
    } catch (error) {
      onMessage(localAiBrowserErrorMessage(error))
    } finally {
      onBusyAction('')
    }
  }

  async function control(action: LocalAiBrowserControlAction) {
    if (!provider || !ownerKey || busyAction) return
    onBusyAction(action)
    onMessage('')
    try {
      onState(await controlLocalAiWebSession(provider.id, ownerKey, action))
      if (action === 'external') {
        onMessage('已打开系统浏览器；系统浏览器不会与一龙本地窗口共享 Cookie。')
      } else if (action === 'background') {
        onMessage(`${provider.displayName} 官方页已收起到本机后台，一龙聊天界面可以继续使用。`)
      }
    } catch (error) {
      onMessage(localAiBrowserErrorMessage(error))
    } finally {
      onBusyAction('')
    }
  }

  return { control, openOfficial, openResearchDirectory }
}
