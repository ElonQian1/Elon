const CHATKIT_SCRIPT_URL = 'https://cdn.platform.openai.com/deployments/chatkit/chatkit.js'
const CHATKIT_LOAD_TIMEOUT_MS = 12_000

let loading: Promise<void> | null = null

/** ChatKit 只在进入 ChatKit 页面后加载，避免影响普通 PC 工作台首屏。 */
export function loadChatKitScript(): Promise<void> {
  if (customElements.get('openai-chatkit')) return Promise.resolve()
  if (loading) return loading

  loading = new Promise<void>((resolve, reject) => {
    const existing = document.querySelector<HTMLScriptElement>(
      `script[src="${CHATKIT_SCRIPT_URL}"]`,
    )
    const ownsScript = !existing
    const script = existing ?? document.createElement('script')
    let settled = false
    const timer = window.setTimeout(() => {
      finish(new Error('ChatKit 组件加载超时，请检查网络或稍后重试'))
    }, CHATKIT_LOAD_TIMEOUT_MS)

    function cleanup() {
      window.clearTimeout(timer)
      script.removeEventListener('load', onLoad)
      script.removeEventListener('error', onError)
    }

    function finish(error?: Error) {
      if (settled) return
      settled = true
      cleanup()
      if (error && ownsScript) script.remove()
      if (error) reject(error)
      else resolve()
    }

    function onLoad() {
      if (customElements.get('openai-chatkit')) {
        finish()
        return
      }
      customElements.whenDefined('openai-chatkit').then(
        () => finish(),
        () => finish(new Error('ChatKit 组件注册失败')),
      )
    }

    function onError() {
      finish(new Error('ChatKit 组件加载失败，请检查网络或稍后重试'))
    }

    script.addEventListener('load', onLoad, { once: true })
    script.addEventListener('error', onError, { once: true })
    if (!existing) {
      script.async = true
      script.src = CHATKIT_SCRIPT_URL
      document.head.appendChild(script)
    }
  }).catch((error) => {
    loading = null
    throw error
  })

  return loading
}
