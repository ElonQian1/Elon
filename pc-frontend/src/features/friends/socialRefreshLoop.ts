import { SOCIAL_REFRESH_EVENT } from './socialRealtime'

/** One bounded read at a time. Resume replaces stale reads; push bursts coalesce. */
export function startSocialRefresh(
  read: (signal: AbortSignal) => Promise<void>,
  options: { interval?: number; timeout?: number } = {},
) {
  let stopped = false
  let timer: ReturnType<typeof setTimeout> | undefined
  let controller: AbortController | undefined
  let queued = false
  let lastResume = 0
  const interval = options.interval ?? 5000

  function schedule(delay: number) {
    clearTimeout(timer)
    timer = setTimeout(() => refresh(), delay)
  }
  async function refresh(replace = false) {
    if (stopped) return
    if (controller && !replace) { queued = true; return }
    clearTimeout(timer)
    controller?.abort()
    const current = new AbortController()
    controller = current
    queued = false
    // Race even if a suspended/buggy transport ignores AbortSignal.
    let deadline: ReturnType<typeof setTimeout> | undefined
    const timeout = new Promise<void>((resolve) => {
      deadline = setTimeout(() => { current.abort(new DOMException('同步超时，请重试', 'TimeoutError')); resolve() }, options.timeout ?? 12000)
    })
    try { await Promise.race([read(current.signal), timeout]) }
    catch { /* read owns the user-facing error state */ }
    finally {
      clearTimeout(deadline)
      if (!stopped && controller === current) {
        controller = undefined
        schedule(queued ? 200 : document.hidden ? 30000 : interval)
      }
    }
  }
  function resume() {
    if (document.hidden) return
    const now = Date.now()
    if (now - lastResume < 250) return
    lastResume = now
    void refresh(true)
  }
  function visibility() {
    if (document.hidden) {
      controller?.abort()
      controller = undefined
      schedule(30000)
    } else resume()
  }
  function invalidate() {
    if (controller) queued = true
    else schedule(150)
  }
  window.addEventListener('focus', resume)
  window.addEventListener('pageshow', resume)
  window.addEventListener('online', resume)
  window.addEventListener(SOCIAL_REFRESH_EVENT, invalidate)
  document.addEventListener('visibilitychange', visibility)
  void refresh()
  return {
    refresh: () => refresh(true),
    stop() {
      stopped = true
      controller?.abort()
      clearTimeout(timer)
      window.removeEventListener('focus', resume)
      window.removeEventListener('pageshow', resume)
      window.removeEventListener('online', resume)
      window.removeEventListener(SOCIAL_REFRESH_EVENT, invalidate)
      document.removeEventListener('visibilitychange', visibility)
    },
  }
}
