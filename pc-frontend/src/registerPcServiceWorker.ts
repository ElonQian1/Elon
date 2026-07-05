const SW_FILE = 'pc-workbench-sw.js'

export function registerPcServiceWorker(): void {
  if (!('serviceWorker' in navigator) || !window.isSecureContext) return

  const base = normalizedBase()
  if (!isCurrentPcScope(base)) return

  window.addEventListener('load', () => {
    const swUrl = new URL(SW_FILE, new URL(base, location.origin)).toString()
    navigator.serviceWorker.register(swUrl, { scope: base })
      .then(() => navigator.serviceWorker.ready)
      .then((registration) => {
        postShellUrls(registration)
      })
      .catch(() => {
        // Service Worker is best-effort; the workbench must keep working without it.
      })
  })
}

function normalizedBase(): string {
  const base = import.meta.env.BASE_URL || '/pc/'
  return base.endsWith('/') ? base : `${base}/`
}

function isCurrentPcScope(base: string): boolean {
  const basePath = new URL(base, location.origin).pathname.replace(/\/$/, '')
  return location.pathname === basePath || location.pathname.startsWith(`${basePath}/`)
}

function postShellUrls(registration: ServiceWorkerRegistration): void {
  const worker = registration.active ?? navigator.serviceWorker.controller
  if (!worker) return
  worker.postMessage({
    type: 'CACHE_URLS',
    urls: collectShellUrls(),
  })
}

function collectShellUrls(): string[] {
  const urls = new Set<string>()
  urls.add(new URL(normalizedBase(), location.origin).toString())

  document
    .querySelectorAll<HTMLScriptElement | HTMLLinkElement>('script[src], link[rel="stylesheet"][href], link[rel="modulepreload"][href]')
    .forEach((element) => {
      const raw = element instanceof HTMLScriptElement ? element.src : element.href
      const url = sameOriginPcUrl(raw)
      if (url) urls.add(url)
    })

  return Array.from(urls)
}

function sameOriginPcUrl(raw: string): string {
  try {
    const url = new URL(raw, location.href)
    if (url.origin !== location.origin) return ''
    if (!url.pathname.startsWith('/pc/')) return ''
    return url.toString()
  } catch {
    return ''
  }
}
