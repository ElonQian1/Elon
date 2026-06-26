export function getPcLegacyUrl() {
  const configured = import.meta.env.VITE_PC_LEGACY_URL?.trim()
  if (configured) return configured

  if (import.meta.env.DEV && typeof window !== 'undefined') {
    const { protocol, hostname } = window.location
    return `${protocol}//${hostname}:8081/pc`
  }

  return '/pc-legacy/'
}

export function rememberPcLegacyToken(token: string | null) {
  if (!token) return
  localStorage.setItem('lodex_token', token)
  localStorage.setItem('elon_token', token)
}
