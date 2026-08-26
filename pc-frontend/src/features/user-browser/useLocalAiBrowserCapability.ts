import { useCallback, useEffect, useState } from 'react'
import {
  isLocalAiBrowserAvailable,
  isLocalAiBrowserUpgradeRequired,
  listLocalAiWebProviders,
  localAiBrowserErrorMessage,
  type LocalAiWebProvider,
} from './localAiBrowserApi'
import { localAiWebProviderPresets } from './localAiWebProviders'

export type LocalAiBrowserCapabilityState =
  | 'desktop_required'
  | 'checking'
  | 'ready'
  | 'upgrade_required'
  | 'error'

export interface LocalAiBrowserCapability {
  state: LocalAiBrowserCapabilityState
  providers: LocalAiWebProvider[]
  message: string
  refresh: () => Promise<void>
}

export default function useLocalAiBrowserCapability(): LocalAiBrowserCapability {
  const desktopDetected = isLocalAiBrowserAvailable()
  const [state, setState] = useState<LocalAiBrowserCapabilityState>(
    desktopDetected ? 'ready' : 'desktop_required',
  )
  const [providers, setProviders] = useState<LocalAiWebProvider[]>(
    desktopDetected ? localAiWebProviderPresets() : [],
  )
  const [message, setMessage] = useState(
    desktopDetected ? '已载入 Win 私有能力预设；正在后台核对当前运行时版本。' : '',
  )

  const verify = useCallback(async (preservePreset: boolean) => {
    if (!desktopDetected) {
      setState('desktop_required')
      setProviders([])
      setMessage('请在一龙 Windows 客户端中使用本地 ChatGPT。')
      return
    }
    if (!preservePreset) {
      setState('checking')
      setMessage('正在核对当前 Win 运行时的网页 AI 适配器版本…')
    }
    try {
      const items = await listLocalAiWebProviders()
      if (!items.length) {
        setProviders([])
        setState('error')
        setMessage('当前 Win 客户端没有登记可用的 AI 网页厂商。')
        return
      }
      setProviders(items)
      setState('ready')
      setMessage('')
    } catch (error) {
      setProviders([])
      setState(isLocalAiBrowserUpgradeRequired(error) ? 'upgrade_required' : 'error')
      setMessage(localAiBrowserErrorMessage(error))
    }
  }, [desktopDetected])

  const refresh = useCallback(() => verify(false), [verify])

  useEffect(() => { void verify(true) }, [verify])

  return { state, providers, message, refresh }
}
