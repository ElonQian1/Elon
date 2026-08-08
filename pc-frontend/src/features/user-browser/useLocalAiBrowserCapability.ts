import { useCallback, useEffect, useState } from 'react'
import {
  isLocalAiBrowserAvailable,
  isLocalAiBrowserUpgradeRequired,
  listLocalAiWebProviders,
  localAiBrowserErrorMessage,
  type LocalAiWebProvider,
} from './localAiBrowserApi'

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
    desktopDetected ? 'checking' : 'desktop_required',
  )
  const [providers, setProviders] = useState<LocalAiWebProvider[]>([])
  const [message, setMessage] = useState('')

  const refresh = useCallback(async () => {
    if (!desktopDetected) {
      setState('desktop_required')
      setProviders([])
      setMessage('请在一龙 Windows 客户端中使用本地 ChatGPT。')
      return
    }
    setState('checking')
    setMessage('')
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

  useEffect(() => { void refresh() }, [refresh])

  return { state, providers, message, refresh }
}
