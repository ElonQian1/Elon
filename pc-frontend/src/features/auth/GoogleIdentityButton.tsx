import { useEffect, useRef, useState } from 'react'
import type { ApiError } from '../../api/client'
import {
  federatedIdentityApi,
  type FederatedCompletion,
} from './federatedIdentityApi'
import styles from './GoogleIdentityButton.module.css'

type GoogleCredentialResponse = { credential?: string }
type GoogleAccountsApi = {
  initialize: (config: {
    client_id: string
    nonce: string
    callback: (response: GoogleCredentialResponse) => void
  }) => void
  renderButton: (parent: HTMLElement, options: Record<string, string | number>) => void
}
declare global {
  interface Window {
    google?: { accounts?: { id?: GoogleAccountsApi } }
  }
}

let googleScriptPromise: Promise<void> | null = null

function loadGoogleIdentityServices() {
  if (window.google?.accounts?.id) return Promise.resolve()
  if (googleScriptPromise) return googleScriptPromise
  googleScriptPromise = new Promise<void>((resolve, reject) => {
    const script = document.createElement('script')
    script.src = 'https://accounts.google.com/gsi/client'
    script.async = true
    script.defer = true
    script.onload = () => resolve()
    script.onerror = () => reject(new Error('无法加载 Google 官方登录组件'))
    document.head.appendChild(script)
  })
  return googleScriptPromise
}

export default function GoogleIdentityButton({
  mode,
  onComplete,
}: {
  mode: 'login' | 'bind'
  onComplete: (result: FederatedCompletion) => void | Promise<void>
}) {
  const hostRef = useRef<HTMLDivElement>(null)
  const completionRef = useRef(onComplete)
  const [status, setStatus] = useState('正在检查 Google 登录…')
  const [error, setError] = useState<string | null>(null)

  useEffect(() => { completionRef.current = onComplete }, [onComplete])

  useEffect(() => {
    let canceled = false
    async function prepare() {
      try {
        const providers = await federatedIdentityApi.providers()
        const google = providers.providers.find((provider) => provider.id === 'google')
        if (!google?.configured || !google.client_id) {
          setStatus('Google 登录等待管理员配置客户端 ID')
          return
        }
        const challenge = await federatedIdentityApi.challenge(mode)
        await loadGoogleIdentityServices()
        if (canceled || !hostRef.current || !window.google?.accounts?.id) return
        hostRef.current.replaceChildren()
        window.google.accounts.id.initialize({
          client_id: google.client_id,
          nonce: challenge.nonce,
          callback: async ({ credential }) => {
            if (!credential) return setError('Google 没有返回 ID token')
            setError(null)
            setStatus(mode === 'login' ? '正在登录一龙账号…' : '正在绑定账号…')
            try {
              const result = await federatedIdentityApi.complete(challenge.id, credential)
              await completionRef.current(result)
              setStatus(mode === 'login' ? '登录成功' : 'Google 账号已绑定')
            } catch (reason) {
              setError((reason as ApiError).message ?? 'Google 登录失败')
              setStatus('请重新打开本页面后再试')
            }
          },
        })
        window.google.accounts.id.renderButton(hostRef.current, {
          type: 'standard',
          theme: 'outline',
          size: 'large',
          width: 300,
          text: mode === 'login' ? 'signin_with' : 'continue_with',
          shape: 'rectangular',
        })
        setStatus(mode === 'login' ? '使用自己的 Google 账号登录' : '绑定后可用同一一龙账号登录')
      } catch (reason) {
        if (!canceled) {
          setError((reason as ApiError).message ?? (reason as Error).message ?? 'Google 登录不可用')
          setStatus('')
        }
      }
    }
    void prepare()
    return () => { canceled = true }
  }, [mode])

  return (
    <div className={styles.root}>
      <div className={styles.buttonHost} ref={hostRef} />
      {status && <p className={styles.status}>{status}</p>}
      {error && <p className={styles.error}>{error}</p>}
    </div>
  )
}
